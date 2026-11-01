use crate::error::{Error, Result};

pub const DELTA1: u8 = 1;
pub const DELTA2: u8 = 2;
pub const DELTA4: u8 = 3;
pub const XOR1: u8 = 4;
pub const XOR2: u8 = 5;
pub const XOR4: u8 = 6;
pub const ACCEL1: u8 = 7;
pub const ACCEL2: u8 = 8;
pub const ACCEL4: u8 = 9;

pub const ALL: [u8; 9] = [DELTA1, DELTA2, DELTA4, XOR1, XOR2, XOR4, ACCEL1, ACCEL2, ACCEL4];

pub fn name(id: u8) -> &'static str {
    match id {
        DELTA1 => "delta1",
        DELTA2 => "delta2",
        DELTA4 => "delta4",
        XOR1 => "xor1",
        XOR2 => "xor2",
        XOR4 => "xor4",
        ACCEL1 => "accel1",
        ACCEL2 => "accel2",
        ACCEL4 => "accel4",
        _ => "unknown",
    }
}

fn width(id: u8) -> Result<usize> {
    match id {
        DELTA1 | XOR1 | ACCEL1 => Ok(1),
        DELTA2 | XOR2 | ACCEL2 => Ok(2),
        DELTA4 | XOR4 | ACCEL4 => Ok(4),
        _ => Err(Error::UnknownCodec),
    }
}

pub fn is_known(id: u8) -> bool {
    width(id).is_ok()
}

fn mask(w: usize) -> u32 {
    if w == 4 {
        u32::MAX
    } else {
        (1u32 << (8 * w)) - 1
    }
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

pub fn forward(id: u8, data: &[u8]) -> Vec<u8> {
    let w = width(id).expect("known transform");
    let mut out = data.to_vec();
    let (mut prev, mut prev2) = (0u32, 0u32);
    for (src, dst) in data.chunks_exact(w).zip(out.chunks_exact_mut(w)) {
        let cur = read(src);
        let coded = match id {
            XOR1 | XOR2 | XOR4 => cur ^ prev,
            ACCEL1 | ACCEL2 | ACCEL4 => cur.wrapping_sub(prev.wrapping_mul(2)).wrapping_add(prev2),
            _ => cur.wrapping_sub(prev),
        };
        write(dst, coded);
        prev2 = prev;
        prev = cur;
    }
    out
}

pub fn inverse(id: u8, data: &mut [u8]) -> Result<()> {
    let w = width(id)?;
    let m = mask(w);
    let (mut prev, mut prev2) = (0u32, 0u32);
    for chunk in data.chunks_exact_mut(w) {
        let coded = read(chunk);
        let cur = match id {
            XOR1 | XOR2 | XOR4 => coded ^ prev,
            ACCEL1 | ACCEL2 | ACCEL4 => coded.wrapping_add(prev.wrapping_mul(2)).wrapping_sub(prev2) & m,
            _ => coded.wrapping_add(prev) & m,
        };
        write(chunk, cur);
        prev2 = prev;
        prev = cur;
    }
    Ok(())
}
