use crate::error::{Error, Result};
use crate::fold::{Expander, Grammar};
use crate::rc::{BitCoder, Decoder, Encoder, HALF};
use crate::varint;

const LITERALS: usize = 256;
const RULE_BUCKETS: usize = 16;
const START: usize = LITERALS + RULE_BUCKETS;
const CONTEXTS: usize = START + 1;
const MAX_LEN_BITS: u32 = 25;
const MAX_EXPANSION: usize = 64;
const MAX_RULES: usize = 1 << 24;

fn bit_len(v: u32) -> u32 {
    32 - v.leading_zeros()
}

struct SymbolModel {
    flag: Vec<u16>,
    literal: Vec<u16>,
    len: Vec<u16>,
    top: Vec<u16>,
    low: Vec<u16>,
}

fn context(value: u32) -> usize {
    if value < 256 {
        value as usize
    } else {
        LITERALS + (bit_len(value - 255) as usize - 1).min(RULE_BUCKETS - 1)
    }
}

impl SymbolModel {
    fn new() -> Self {
        SymbolModel {
            flag: vec![HALF; CONTEXTS],
            literal: vec![HALF; CONTEXTS * 256],
            len: vec![HALF; CONTEXTS * 32],
            top: vec![HALF; 32 * 8],
            low: vec![HALF; 32 * 32],
        }
    }

    fn code<C: BitCoder>(&mut self, c: &mut C, value: u32, ctx: usize) -> Result<u32> {
        let is_rule = c.code(&mut self.flag[ctx], (value >= 256) as u32);
        let out = if is_rule == 0 {
            let mut node = 1usize;
            for i in (0..8).rev() {
                let bit = c.code(&mut self.literal[ctx * 256 + node], (value >> i) & 1);
                node = (node << 1) | bit as usize;
            }
            (node & 0xff) as u32
        } else {
            let v = value.wrapping_sub(255);
            let want = bit_len(v);
            let mut node = 1usize;
            for i in (0..5).rev() {
                let bit = c.code(&mut self.len[ctx * 32 + node], (want >> i) & 1);
                node = (node << 1) | bit as usize;
            }
            let nb = (node & 31) as u32;
            if nb == 0 || nb > MAX_LEN_BITS {
                return Err(Error::Corrupt);
            }
            let mut r = 1u32;
            let mut tree = 1usize;
            for k in 0..nb - 1 {
                let shift = nb - 2 - k;
                let want_bit = (v >> shift) & 1;
                let bit = if k < 3 {
                    let b = c.code(&mut self.top[nb as usize * 8 + tree], want_bit);
                    tree = (tree << 1) | b as usize;
                    b
                } else {
                    c.code(&mut self.low[nb as usize * 32 + shift as usize], want_bit)
                };
                r = (r << 1) | bit;
            }
            r + 255
        };
        Ok(out)
    }
}

pub(crate) const ABSOLUTE: u8 = 0;
pub(crate) const DISTANCE: u8 = 1;

fn relative(child: u32, index: usize, mode: u8) -> u32 {
    if child < 256 || mode == ABSOLUTE {
        child
    } else {
        255 + (256 + index as u32 - child)
    }
}

pub fn encode(g: &Grammar) -> Vec<u8> {
    let absolute = encode_with(g, ABSOLUTE);
    let distance = encode_with(g, DISTANCE);
    if distance.len() < absolute.len() {
        distance
    } else {
        absolute
    }
}

pub(crate) fn encode_with(g: &Grammar, mode: u8) -> Vec<u8> {
    let mut out = vec![mode];
    varint::put(&mut out, g.rules.len() as u64);
    varint::put(&mut out, g.seq.len() as u64);
    let mut enc = Encoder::new();
    let mut left = SymbolModel::new();
    let mut right = SymbolModel::new();
    let (mut prev_left, mut prev_right) = (START, START);
    for (i, &(a, b)) in g.rules.iter().enumerate() {
        let a = relative(a, i, mode);
        let b = relative(b, i, mode);
        left.code(&mut enc, a, prev_left).expect("symbol fits");
        right.code(&mut enc, b, prev_right).expect("symbol fits");
        prev_left = context(a);
        prev_right = context(b);
    }
    let mut model = SymbolModel::new();
    let mut prev = START;
    for &s in &g.seq {
        model.code(&mut enc, s, prev).expect("symbol fits");
        prev = context(s);
    }
    out.extend_from_slice(&enc.finish());
    out
}

fn absolute(coded: u32, index: usize, mode: u8) -> Result<u32> {
    if coded < 256 {
        return Ok(coded);
    }
    if mode == ABSOLUTE {
        if coded as usize >= 256 + index {
            return Err(Error::Corrupt);
        }
        return Ok(coded);
    }
    let distance = coded - 255;
    if distance as usize > index {
        return Err(Error::Corrupt);
    }
    Ok(256 + index as u32 - distance)
}

pub fn decode(src: &[u8], raw_len: usize) -> Result<Vec<u8>> {
    let mut pos = 0usize;
    let mode = *src.first().ok_or(Error::Truncated)?;
    if mode != ABSOLUTE && mode != DISTANCE {
        return Err(Error::Corrupt);
    }
    pos += 1;
    let rule_count = usize::try_from(varint::get(src, &mut pos)?).map_err(|_| Error::Corrupt)?;
    let seq_count = usize::try_from(varint::get(src, &mut pos)?).map_err(|_| Error::Corrupt)?;
    let budget = (src.len() + 16).saturating_mul(MAX_EXPANSION);
    if rule_count > MAX_RULES || rule_count > budget || seq_count > budget {
        return Err(Error::Corrupt);
    }

    let mut dec = Decoder::new(&src[pos..]);
    let mut left = SymbolModel::new();
    let mut right = SymbolModel::new();
    let (mut prev_left, mut prev_right) = (START, START);
    let mut rules: Vec<(u32, u32)> = Vec::with_capacity(rule_count.min(1 << 16));
    for i in 0..rule_count {
        let a = left.code(&mut dec, 0, prev_left)?;
        let b = right.code(&mut dec, 0, prev_right)?;
        prev_left = context(a);
        prev_right = context(b);
        if dec.overrun() {
            return Err(Error::Truncated);
        }
        rules.push((absolute(a, i, mode)?, absolute(b, i, mode)?));
    }

    let symbol_limit = 256 + rule_count as u64;
    let mut ex = Expander::new(rules, raw_len);
    let mut model = SymbolModel::new();
    let mut prev = START;
    for _ in 0..seq_count {
        let s = model.code(&mut dec, 0, prev)?;
        prev = context(s);
        if s as u64 >= symbol_limit {
            return Err(Error::Corrupt);
        }
        if dec.overrun() {
            return Err(Error::Truncated);
        }
        ex.push(s)?;
    }

    if dec.overrun() {
        return Err(Error::Truncated);
    }
    if !dec.exhausted() {
        return Err(Error::Corrupt);
    }
    ex.finish()
}
