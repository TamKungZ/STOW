use crate::error::{Error, Result};

const TOP: u32 = 1 << 24;
const ONE: u16 = 1 << 11;
const BITS: u32 = 11;
const ADAPT: u32 = 5;
const MAX_EXPANSION: usize = 64;

struct Encoder {
    low: u64,
    range: u32,
    cache: u8,
    cache_size: u64,
    out: Vec<u8>,
}

impl Encoder {
    fn new() -> Self {
        Encoder { low: 0, range: u32::MAX, cache: 0, cache_size: 1, out: Vec::new() }
    }

    fn shift_low(&mut self) {
        if (self.low as u32) < 0xFF00_0000 || (self.low >> 32) != 0 {
            let carry = (self.low >> 32) as u8;
            let mut byte = self.cache;
            loop {
                self.out.push(byte.wrapping_add(carry));
                byte = 0xFF;
                self.cache_size -= 1;
                if self.cache_size == 0 {
                    break;
                }
            }
            self.cache = ((self.low >> 24) & 0xFF) as u8;
        }
        self.cache_size += 1;
        self.low = (self.low & 0x00FF_FFFF) << 8;
    }

    fn bit(&mut self, p: &mut u16, bit: u32) {
        let bound = (self.range >> BITS) * (*p as u32);
        if bit == 0 {
            self.range = bound;
            *p += (ONE - *p) >> ADAPT;
        } else {
            self.low += bound as u64;
            self.range -= bound;
            *p -= *p >> ADAPT;
        }
        while self.range < TOP {
            self.range <<= 8;
            self.shift_low();
        }
    }

    fn finish(mut self) -> Vec<u8> {
        for _ in 0..5 {
            self.shift_low();
        }
        self.out
    }
}

struct Decoder<'a> {
    src: &'a [u8],
    pos: usize,
    range: u32,
    code: u32,
}

impl<'a> Decoder<'a> {
    fn new(src: &'a [u8]) -> Self {
        let mut d = Decoder { src, pos: 0, range: u32::MAX, code: 0 };
        for _ in 0..5 {
            d.code = (d.code << 8) | d.next_byte() as u32;
        }
        d
    }

    fn next_byte(&mut self) -> u8 {
        let b = self.src.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }

    fn bit(&mut self, p: &mut u16) -> u32 {
        let bound = (self.range >> BITS) * (*p as u32);
        let bit;
        if self.code < bound {
            self.range = bound;
            *p += (ONE - *p) >> ADAPT;
            bit = 0;
        } else {
            self.code -= bound;
            self.range -= bound;
            *p -= *p >> ADAPT;
            bit = 1;
        }
        while self.range < TOP {
            self.range <<= 8;
            self.code = (self.code << 8) | self.next_byte() as u32;
        }
        bit
    }
}

pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut probs = [ONE / 2; 256];
    let mut enc = Encoder::new();
    for &byte in data {
        let mut node = 1usize;
        for i in (0..8).rev() {
            let bit = ((byte >> i) & 1) as u32;
            enc.bit(&mut probs[node], bit);
            node = (node << 1) | bit as usize;
        }
    }
    enc.finish()
}

pub fn decode(src: &[u8], len: usize) -> Result<Vec<u8>> {
    if len > (src.len() + 16).saturating_mul(MAX_EXPANSION) {
        return Err(Error::Corrupt);
    }
    let mut probs = [ONE / 2; 256];
    let mut dec = Decoder::new(src);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        let mut node = 1usize;
        for _ in 0..8 {
            let bit = dec.bit(&mut probs[node]);
            node = (node << 1) | bit as usize;
        }
        out.push((node & 0xff) as u8);
    }
    if dec.pos > src.len() {
        return Err(Error::Truncated);
    }
    if dec.pos < src.len() {
        return Err(Error::Corrupt);
    }
    Ok(out)
}
