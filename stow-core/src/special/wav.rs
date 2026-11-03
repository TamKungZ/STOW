use super::{Special, BASE_ID};
use crate::codec;
use crate::error::{Error, Result};
use crate::varint;
use std::thread;

pub const SPECIAL: Special = Special { id: BASE_ID, name: "wav", detect, encode, decode };

const BLOCK: usize = 4096;
const MAX_CHANNELS: usize = 8;
const MAX_ORDER: u8 = 3;

struct Layout {
    data_start: usize,
    frames: usize,
    channels: usize,
}

fn layout(data: &[u8]) -> Option<Layout> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12usize;
    let mut channels = None;
    while pos.checked_add(8)? <= data.len() {
        let size = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body = pos + 8;
        match &data[pos..pos + 4] {
            b"fmt " => {
                let f = data.get(body..body.checked_add(16)?)?;
                let tag = u16::from_le_bytes([f[0], f[1]]);
                let ch = u16::from_le_bytes([f[2], f[3]]) as usize;
                let bits = u16::from_le_bytes([f[14], f[15]]);
                if tag != 1 || bits != 16 || ch == 0 || ch > MAX_CHANNELS {
                    return None;
                }
                channels = Some(ch);
            }
            b"data" => {
                let channels = channels?;
                let frames = size.min(data.len() - body) / (2 * channels);
                return (frames >= 2).then_some(Layout { data_start: body, frames, channels });
            }
            _ => {}
        }
        pos = body.checked_add(size)?.checked_add(size & 1)?;
    }
    None
}

fn detect(data: &[u8]) -> bool {
    layout(data).is_some()
}

fn history(sig: &[i16], i: usize) -> [i16; 3] {
    let at = |k: usize| if i >= k { sig[i - k] } else { 0 };
    [at(1), at(2), at(3)]
}

fn predict(order: u8, h: [i16; 3]) -> i16 {
    let (a, b, c) = (h[0] as i32, h[1] as i32, h[2] as i32);
    let p = match order {
        0 => 0,
        1 => a,
        2 => 2 * a - b,
        _ => 3 * a - 3 * b + c,
    };
    p as i16
}

fn zigzag(r: i16) -> u16 {
    ((r << 1) ^ (r >> 15)) as u16
}

fn unzigzag(z: u16) -> i16 {
    ((z >> 1) as i16) ^ -((z & 1) as i16)
}

fn residual(sig: &[i16], i: usize, order: u8) -> i16 {
    sig[i].wrapping_sub(predict(order, history(sig, i)))
}

fn best_order(sig: &[i16], range: std::ops::Range<usize>) -> u8 {
    (0..=MAX_ORDER)
        .min_by_key(|&o| range.clone().map(|i| residual(sig, i, o).unsigned_abs() as u64).sum::<u64>())
        .expect("orders are not empty")
}

fn stream(data: &[u8]) -> Vec<u8> {
    let (id, payload) = codec::best_plain(data);
    let mut out = vec![id];
    varint::put(&mut out, payload.len() as u64);
    out.extend_from_slice(&payload);
    out
}

fn take<'a>(payload: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8]> {
    let end = pos.checked_add(len).ok_or(Error::Truncated)?;
    let slice = payload.get(*pos..end).ok_or(Error::Truncated)?;
    *pos = end;
    Ok(slice)
}

fn take_sized<'a>(payload: &'a [u8], pos: &mut usize) -> Result<&'a [u8]> {
    let len = usize::try_from(varint::get(payload, pos)?).map_err(|_| Error::TooLarge)?;
    take(payload, pos, len)
}

fn get_stream(payload: &[u8], pos: &mut usize, raw_len: usize) -> Result<Vec<u8>> {
    let id = take(payload, pos, 1)?[0];
    if id > codec::FOLD_AC {
        return Err(Error::UnknownCodec);
    }
    let body = take_sized(payload, pos)?;
    codec::decode(id, body, raw_len)
}

fn encode(data: &[u8]) -> Option<Vec<u8>> {
    let Layout { data_start, frames, channels } = layout(data)?;
    let pcm_end = data_start + frames * 2 * channels;
    let mut sigs: Vec<Vec<i16>> = (0..channels)
        .map(|c| {
            (0..frames)
                .map(|f| {
                    let at = data_start + (f * channels + c) * 2;
                    i16::from_le_bytes([data[at], data[at + 1]])
                })
                .collect()
        })
        .collect();
    if channels == 2 {
        for f in 0..frames {
            sigs[1][f] = sigs[1][f].wrapping_sub(sigs[0][f]);
        }
    }

    let blocks = frames.div_ceil(BLOCK);
    let mut orders = Vec::with_capacity(blocks * channels);
    let mut low = Vec::with_capacity(channels);
    let mut high = Vec::with_capacity(channels);
    for sig in &sigs {
        let (mut lo, mut hi) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
        for b in 0..blocks {
            let range = b * BLOCK..((b + 1) * BLOCK).min(frames);
            let order = best_order(sig, range.clone());
            orders.push(order);
            for i in range {
                let z = zigzag(residual(sig, i, order));
                lo.push(z as u8);
                hi.push((z >> 8) as u8);
            }
        }
        low.push(lo);
        high.push(hi);
    }

    let mut out = Vec::new();
    varint::put(&mut out, data_start as u64);
    out.extend_from_slice(&data[..data_start]);
    varint::put(&mut out, (data.len() - pcm_end) as u64);
    out.extend_from_slice(&data[pcm_end..]);
    varint::put(&mut out, frames as u64);
    out.push(channels as u8);
    let planes: Vec<&[u8]> = std::iter::once(orders.as_slice())
        .chain((0..channels).flat_map(|c| [low[c].as_slice(), high[c].as_slice()]))
        .collect();
    let streams: Vec<Vec<u8>> = thread::scope(|scope| {
        let handles: Vec<_> = planes.iter().map(|&p| scope.spawn(move || stream(p))).collect();
        handles.into_iter().map(|h| h.join().expect("stream encoder panicked")).collect()
    });
    for s in streams {
        out.extend_from_slice(&s);
    }
    Some(out)
}

fn decode(payload: &[u8], raw_len: usize) -> Result<Vec<u8>> {
    let mut pos = 0usize;
    let header = take_sized(payload, &mut pos)?;
    let tail = take_sized(payload, &mut pos)?;
    let frames = usize::try_from(varint::get(payload, &mut pos)?).map_err(|_| Error::TooLarge)?;
    let channels = take(payload, &mut pos, 1)?[0] as usize;
    if channels == 0 || channels > MAX_CHANNELS {
        return Err(Error::Corrupt);
    }
    let pcm = frames.checked_mul(2 * channels).ok_or(Error::TooLarge)?;
    let total = header.len().checked_add(pcm).and_then(|n| n.checked_add(tail.len()));
    if total != Some(raw_len) {
        return Err(Error::Corrupt);
    }

    let blocks = frames.div_ceil(BLOCK);
    let orders = get_stream(payload, &mut pos, blocks * channels)?;
    if orders.iter().any(|&o| o > MAX_ORDER) {
        return Err(Error::Corrupt);
    }
    let mut sigs = Vec::with_capacity(channels);
    for c in 0..channels {
        let lo = get_stream(payload, &mut pos, frames)?;
        let hi = get_stream(payload, &mut pos, frames)?;
        let mut sig: Vec<i16> = Vec::with_capacity(frames);
        for i in 0..frames {
            let order = orders[c * blocks + i / BLOCK];
            let z = lo[i] as u16 | (hi[i] as u16) << 8;
            sig.push(unzigzag(z).wrapping_add(predict(order, history(&sig, i))));
        }
        sigs.push(sig);
    }
    if pos != payload.len() {
        return Err(Error::Corrupt);
    }
    if channels == 2 {
        for f in 0..frames {
            sigs[1][f] = sigs[1][f].wrapping_add(sigs[0][f]);
        }
    }

    let mut out = Vec::with_capacity(raw_len);
    out.extend_from_slice(header);
    for f in 0..frames {
        for sig in &sigs {
            out.extend_from_slice(&sig[f].to_le_bytes());
        }
    }
    out.extend_from_slice(tail);
    Ok(out)
}
