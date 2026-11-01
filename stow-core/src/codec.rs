use crate::error::{Error, Result};
use crate::{ac, fold, rc, transform, varint};

pub const STORE: u8 = 0;
pub const FOLD: u8 = 1;
pub const FOLD_RC: u8 = 2;
pub const FOLD_AC: u8 = 3;
pub const XFORM: u8 = 4;

const SAMPLE: usize = 64 << 10;

pub fn name(id: u8) -> &'static str {
    match id {
        STORE => "store",
        FOLD => "fold",
        FOLD_RC => "fold+rc",
        FOLD_AC => "fold+ac",
        XFORM => "xform",
        _ => "unknown",
    }
}

fn plain_candidates(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut all = vec![(STORE, data.to_vec())];
    if data.len() >= 2 && data.len() <= fold::MAX_INPUT {
        let grammar = fold::build(data);
        let folded = fold::encode_grammar(&grammar);
        let mut packed = Vec::new();
        varint::put(&mut packed, folded.len() as u64);
        packed.extend_from_slice(&rc::encode(&folded));
        all.push((FOLD, folded));
        all.push((FOLD_RC, packed));
        all.push((FOLD_AC, ac::encode(&grammar)));
    }
    all
}

fn transformed(data: &[u8], transform_id: u8) -> Option<Vec<u8>> {
    if data.len() < 2 {
        return None;
    }
    let moved = transform::forward(transform_id, data);
    let (inner_id, inner) = plain_candidates(&moved)
        .into_iter()
        .filter(|(id, _)| *id != STORE)
        .min_by_key(|(_, p)| p.len())?;
    let mut out = vec![transform_id, inner_id];
    out.extend_from_slice(&inner);
    Some(out)
}

pub fn candidates(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut all = plain_candidates(data);
    for &t in &transform::ALL {
        if let Some(payload) = transformed(data, t) {
            all.push((XFORM, payload));
        }
    }
    all
}

pub fn describe(id: u8, payload: &[u8]) -> String {
    match (id, payload) {
        (XFORM, [t, inner, ..]) => format!("{}+{}", transform::name(*t), name(*inner)),
        _ => name(id).to_owned(),
    }
}

fn best_plain_len(data: &[u8]) -> usize {
    plain_candidates(data)
        .iter()
        .filter(|(id, _)| *id != STORE)
        .map(|(_, p)| p.len())
        .min()
        .unwrap_or(usize::MAX)
}

fn transforms_to_try(data: &[u8]) -> Vec<u8> {
    if data.len() <= SAMPLE {
        return transform::ALL.to_vec();
    }
    let sample = &data[..SAMPLE];
    let mut best_len = best_plain_len(sample);
    let mut best = Vec::new();
    for &t in &transform::ALL {
        if let Some(p) = transformed(sample, t) {
            if p.len() < best_len {
                best_len = p.len();
                best = vec![t];
            }
        }
    }
    best
}

pub fn smallest(data: &[u8]) -> (u8, Vec<u8>) {
    let mut all = plain_candidates(data);
    for t in transforms_to_try(data) {
        if let Some(payload) = transformed(data, t) {
            all.push((XFORM, payload));
        }
    }
    let mut best: Option<(u8, Vec<u8>)> = None;
    for (id, payload) in all {
        match &best {
            Some((_, b)) if b.len() <= payload.len() => {}
            _ => best = Some((id, payload)),
        }
    }
    best.expect("store candidate always exists")
}

pub fn decode(id: u8, payload: &[u8], raw_len: usize) -> Result<Vec<u8>> {
    match id {
        STORE => {
            if payload.len() != raw_len {
                return Err(Error::Corrupt);
            }
            Ok(payload.to_vec())
        }
        FOLD => fold::decode(payload, raw_len),
        FOLD_RC => {
            let mut pos = 0usize;
            let inner_len = varint::get(payload, &mut pos)?;
            let inner_len = usize::try_from(inner_len).map_err(|_| Error::TooLarge)?;
            let folded = rc::decode(&payload[pos..], inner_len)?;
            fold::decode(&folded, raw_len)
        }
        FOLD_AC => ac::decode(payload, raw_len),
        XFORM => {
            let [t, inner, rest @ ..] = payload else {
                return Err(Error::Truncated);
            };
            if !transform::is_known(*t) || *inner == XFORM {
                return Err(Error::UnknownCodec);
            }
            let mut data = decode(*inner, rest, raw_len)?;
            transform::inverse(*t, &mut data)?;
            Ok(data)
        }
        _ => Err(Error::UnknownCodec),
    }
}
