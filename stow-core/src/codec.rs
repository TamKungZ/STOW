use crate::error::{Error, Result};
use crate::{ac, fold, rc, special, transform, varint};

pub const STORE: u8 = 0;
pub const FOLD: u8 = 1;
pub const FOLD_RC: u8 = 2;
pub const FOLD_AC: u8 = 3;
pub const XFORM: u8 = 4;
pub const SPECIAL_BASE: u8 = special::BASE_ID;

const SAMPLE: usize = 64 << 10;
const WINDOW: usize = 12 << 10;
const WINDOWS: usize = 8;
const LARGE: usize = 4 << 20;
const PROBE: usize = 1 << 20;

pub fn name(id: u8) -> &'static str {
    match id {
        STORE => "store",
        FOLD => "fold",
        FOLD_RC => "fold+rc",
        FOLD_AC => "fold+ac",
        XFORM => "xform",
        _ => special::find(id).map_or("unknown", |s| s.name),
    }
}

fn plain_candidates(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut all = vec![(STORE, data.to_vec())];
    if data.len() >= 2 && data.len() <= fold::MAX_INPUT {
        let stages = fold::build_stages(data, &fold::STAGES);
        let folded = fold::encode_grammar(&stages[1]);
        let mut packed = Vec::new();
        varint::put(&mut packed, folded.len() as u64);
        packed.extend_from_slice(&rc::encode(&folded));
        all.push((FOLD, folded));
        all.push((FOLD_RC, packed));
        let coded = stages
            .iter()
            .enumerate()
            .filter(|(i, g)| *i == 0 || g.rules.len() != stages[i - 1].rules.len())
            .map(|(_, g)| ac::encode(g))
            .min_by_key(Vec::len)
            .expect("at least one stage");
        all.push((FOLD_AC, coded));
    }
    all
}

pub(crate) fn best_plain(data: &[u8]) -> (u8, Vec<u8>) {
    plain_candidates(data)
        .into_iter()
        .min_by_key(|(_, p)| p.len())
        .expect("store candidate always exists")
}

fn special_candidates(data: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    for s in special::ALL.iter().filter(|s| (s.detect)(data)) {
        if let Some(payload) = (s.encode)(data) {
            if (s.decode)(&payload, data.len()).is_ok_and(|back| back == data) {
                out.push((s.id, payload));
            }
        }
    }
    out
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
    all.extend(special_candidates(data));
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

struct Strategy {
    plain: bool,
    transform: Option<u8>,
}

fn sample_of(data: &[u8]) -> Vec<u8> {
    let span = data.len() - WINDOW;
    (0..WINDOWS)
        .flat_map(|k| {
            let at = (span / (WINDOWS - 1) * k) & !3;
            data[at..at + WINDOW].iter().copied()
        })
        .collect()
}

fn probe_pays(data: &[u8]) -> bool {
    if data.len() <= LARGE {
        return true;
    }
    let at = (data.len() / 2 - PROBE / 2) & !3;
    let probe = &data[at..at + PROBE];
    best_plain_len(probe) < probe.len() / 100 * 95
}

fn strategy(data: &[u8]) -> Strategy {
    if data.len() <= SAMPLE {
        return Strategy { plain: true, transform: None };
    }
    let sample = sample_of(data);
    let plain = best_plain_len(&sample);
    let mut best: Option<(u8, usize)> = None;
    for &t in &transform::ALL {
        if let Some(p) = transformed(&sample, t) {
            if p.len() < best.map_or(plain, |(_, n)| n) {
                best = Some((t, p.len()));
            }
        }
    }
    let smallest = best.map_or(plain, |(_, n)| n);
    if smallest / 49 * 50 >= sample.len() {
        return Strategy { plain: false, transform: None };
    }
    match best {
        Some((t, n)) if n / 10 * 9 < plain => Strategy { plain: false, transform: Some(t) },
        Some((t, _)) => Strategy { plain: probe_pays(data), transform: Some(t) },
        None => Strategy { plain: probe_pays(data), transform: None },
    }
}

pub fn smallest(data: &[u8]) -> (u8, Vec<u8>) {
    let special = special_candidates(data);
    let decisive = special
        .iter()
        .enumerate()
        .filter(|(_, (_, p))| p.len() / 10 * 9 < data.len())
        .min_by_key(|(_, (_, p))| p.len())
        .map(|(i, _)| i);
    if let Some(i) = decisive {
        return special.into_iter().nth(i).expect("index from the same list");
    }
    let mut all = special;
    if data.len() <= SAMPLE {
        all = plain_candidates(data);
        for &t in &transform::ALL {
            if let Some(payload) = transformed(data, t) {
                all.push((XFORM, payload));
            }
        }
    } else {
        let plan = strategy(data);
        if plan.plain {
            all = plain_candidates(data);
        } else {
            all.push((STORE, data.to_vec()));
        }
        if let Some(payload) = plan.transform.and_then(|t| transformed(data, t)) {
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
        _ => match special::find(id) {
            Some(s) => (s.decode)(payload, raw_len),
            None => Err(Error::UnknownCodec),
        },
    }
}
