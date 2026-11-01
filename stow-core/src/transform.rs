use crate::error::{Error, Result};

pub const DELTA1: u8 = 1;
pub const DELTA2: u8 = 2;
pub const DELTA4: u8 = 3;
pub const XOR1: u8 = 4;
pub const XOR2: u8 = 5;
pub const XOR4: u8 = 6;

pub const ALL: [u8; 6] = [DELTA1, DELTA2, DELTA4, XOR1, XOR2, XOR4];

pub fn name(id: u8) -> &'static str {
    match id {
        DELTA1 => "delta1",
        DELTA2 => "delta2",
        DELTA4 => "delta4",
        XOR1 => "xor1",
        XOR2 => "xor2",
        XOR4 => "xor4",
        _ => "unknown",
    }
}

fn width(id: u8) -> Result<usize> {
    match id {
        DELTA1 | XOR1 => Ok(1),
        DELTA2 | XOR2 => Ok(2),
        DELTA4 | XOR4 => Ok(4),
        _ => Err(Error::UnknownCodec),
    }
}

pub fn is_known(id: u8) -> bool {
    width(id).is_ok()
}

fn read(chunk: &[u8]) -> u32 {
    chunk.iter().rev().fold(0u32, |acc, &b| (acc << 8) | b as u32)
}

fn write(chunk: &mut [u8], mut value: u32) {
    for b in chunk {
        *b = value as u8;
        value >>= 8;
    }
}

fn is_xor(id: u8) -> bool {
    id >= XOR1
}

pub fn forward(id: u8, data: &[u8]) -> Vec<u8> {
    let w = width(id).expect("known transform");
    let mut out = data.to_vec();
    let mut prev = 0u32;
    for (src, dst) in data.chunks_exact(w).zip(out.chunks_exact_mut(w)) {
        let cur = read(src);
        let coded = if is_xor(id) { cur ^ prev } else { cur.wrapping_sub(prev) };
        write(dst, coded);
        prev = cur;
    }
    out
}

pub fn inverse(id: u8, data: &mut [u8]) -> Result<()> {
    let w = width(id)?;
    let mask = if w == 4 { u32::MAX } else { (1u32 << (8 * w)) - 1 };
    let mut prev = 0u32;
    for chunk in data.chunks_exact_mut(w) {
        let coded = read(chunk);
        let cur = if is_xor(id) { coded ^ prev } else { coded.wrapping_add(prev) & mask };
        write(chunk, cur);
        prev = cur;
    }
    Ok(())
}
