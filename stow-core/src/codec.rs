use crate::error::{Error, Result};
use crate::{fold, rc, varint};

pub const STORE: u8 = 0;
pub const FOLD: u8 = 1;
pub const FOLD_RC: u8 = 2;

pub fn name(id: u8) -> &'static str {
    match id {
        STORE => "store",
        FOLD => "fold",
        FOLD_RC => "fold+rc",
        _ => "unknown",
    }
}

pub fn candidates(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut all = vec![(STORE, data.to_vec())];
    if data.len() >= 2 && data.len() <= fold::MAX_INPUT {
        let folded = fold::encode(data);
        let mut packed = Vec::new();
        varint::put(&mut packed, folded.len() as u64);
        packed.extend_from_slice(&rc::encode(&folded));
        all.push((FOLD, folded));
        all.push((FOLD_RC, packed));
    }
    all
}

pub fn smallest(data: &[u8]) -> (u8, Vec<u8>) {
    let mut best: Option<(u8, Vec<u8>)> = None;
    for (id, payload) in candidates(data) {
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
        _ => Err(Error::UnknownCodec),
    }
}
