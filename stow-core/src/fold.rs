use crate::error::{Error, Result};
use crate::varint;
use std::collections::{BinaryHeap, HashMap};

const NONE: u32 = u32::MAX;
const DEAD: u32 = u32::MAX;
const MAX_RULES: usize = 1 << 24;

pub const MAX_INPUT: usize = 1 << 30;

#[derive(Default)]
struct Slot {
    pos: Vec<u32>,
    queued: u32,
}

#[derive(Clone)]
pub struct Grammar {
    pub rules: Vec<(u32, u32)>,
    pub seq: Vec<u32>,
}

fn key(a: u32, b: u32) -> u64 {
    ((a as u64) << 32) | b as u64
}

fn unkey(k: u64) -> (u32, u32) {
    ((k >> 32) as u32, k as u32)
}

fn is_pair(seq: &[u32], next: &[u32], p: u32, a: u32, b: u32) -> bool {
    let p = p as usize;
    if seq[p] != a {
        return false;
    }
    let q = next[p];
    q != NONE && seq[q as usize] == b
}

fn add_pair(
    occ: &mut HashMap<u64, Slot>,
    heap: &mut BinaryHeap<(u32, u64)>,
    a: u32,
    b: u32,
    pos: u32,
    min_count: u32,
) {
    let k = key(a, b);
    let slot = occ.entry(k).or_default();
    slot.pos.push(pos);
    let count = slot.pos.len() as u32;
    if count >= min_count {
        slot.queued = count;
        heap.push((count, k));
    }
}

pub const STAGES: [u32; 3] = [4, 3, 2];

pub fn build(input: &[u8]) -> Grammar {
    build_stages(input, &[3]).pop().expect("one stage")
}

fn snapshot(seq: &[u32], next: &[u32], rules: &[(u32, u32)]) -> Grammar {
    let mut out = Vec::new();
    let mut i = 0u32;
    while i != NONE {
        out.push(seq[i as usize]);
        i = next[i as usize];
    }
    Grammar { rules: rules.to_vec(), seq: out }
}

pub fn build_stages(input: &[u8], stages: &[u32]) -> Vec<Grammar> {
    let min_count = *stages.last().expect("stages are not empty");
    let mut pending: Vec<u32> = stages.to_vec();
    let mut done: Vec<Grammar> = Vec::new();
    let n = input.len();
    let mut seq: Vec<u32> = input.iter().map(|&b| b as u32).collect();
    let mut rules: Vec<(u32, u32)> = Vec::new();
    if n < 2 || n > MAX_INPUT {
        return stages.iter().map(|_| Grammar { rules: rules.clone(), seq: seq.clone() }).collect();
    }

    let mut next: Vec<u32> = (1..=n as u32).collect();
    next[n - 1] = NONE;
    let mut prev: Vec<u32> = (0..n as u32).map(|i| i.wrapping_sub(1)).collect();

    let mut occ: HashMap<u64, Slot> = HashMap::new();
    for i in 0..n - 1 {
        occ.entry(key(seq[i], seq[i + 1])).or_default().pos.push(i as u32);
    }
    let mut heap: BinaryHeap<(u32, u64)> = BinaryHeap::new();
    for (k, slot) in occ.iter_mut() {
        let count = slot.pos.len() as u32;
        if count >= min_count {
            slot.queued = count;
            heap.push((count, *k));
        }
    }

    while let Some((count, k)) = heap.pop() {
        if count < min_count || rules.len() >= MAX_RULES {
            break;
        }
        match occ.get(&k) {
            Some(slot) if slot.queued == count => {}
            _ => continue,
        }
        while pending.len() > 1 && count < pending[0] {
            done.push(snapshot(&seq, &next, &rules));
            pending.remove(0);
        }
        let mut list = occ.remove(&k).map(|slot| slot.pos).unwrap_or_default();
        let (a, b) = unkey(k);
        list.retain(|&p| is_pair(&seq, &next, p, a, b));
        list.sort_unstable();
        list.dedup();

        let mut real = 0u32;
        let mut consumed = NONE;
        for &p in &list {
            if p == consumed {
                continue;
            }
            real += 1;
            consumed = next[p as usize];
        }

        if real < min_count {
            if !list.is_empty() {
                occ.insert(k, Slot { pos: list, queued: 0 });
            }
            continue;
        }
        if real < count {
            let top = heap.peek().map(|e| e.0).unwrap_or(0);
            if real < top {
                occ.insert(k, Slot { pos: list, queued: real });
                heap.push((real, k));
                continue;
            }
        }

        let sym = 256 + rules.len() as u32;
        rules.push((a, b));
        for &p in &list {
            if !is_pair(&seq, &next, p, a, b) {
                continue;
            }
            let q = next[p as usize];
            let after = next[q as usize];
            seq[p as usize] = sym;
            seq[q as usize] = DEAD;
            next[p as usize] = after;
            if after != NONE {
                prev[after as usize] = p;
            }
            let before = prev[p as usize];
            if before != NONE {
                add_pair(&mut occ, &mut heap, seq[before as usize], sym, before, min_count);
            }
            if after != NONE {
                add_pair(&mut occ, &mut heap, sym, seq[after as usize], p, min_count);
            }
        }
    }

    while done.len() < stages.len() {
        done.push(snapshot(&seq, &next, &rules));
    }
    done
}

pub fn encode(input: &[u8]) -> Vec<u8> {
    encode_grammar(&build(input))
}

pub fn encode_grammar(g: &Grammar) -> Vec<u8> {
    let mut out = Vec::new();
    varint::put(&mut out, g.rules.len() as u64);
    varint::put(&mut out, g.seq.len() as u64);
    for &(a, b) in &g.rules {
        varint::put(&mut out, a as u64);
        varint::put(&mut out, b as u64);
    }
    for &s in &g.seq {
        varint::put(&mut out, s as u64);
    }
    out
}

pub fn decode(src: &[u8], raw_len: usize) -> Result<Vec<u8>> {
    let mut pos = 0usize;
    let rule_count = varint::get(src, &mut pos)? as usize;
    let seq_count = varint::get(src, &mut pos)? as usize;
    if rule_count > src.len() || seq_count > src.len() {
        return Err(Error::Corrupt);
    }

    let mut rules: Vec<(u32, u32)> = Vec::with_capacity(rule_count);
    for i in 0..rule_count {
        let a = varint::get(src, &mut pos)?;
        let b = varint::get(src, &mut pos)?;
        let limit = 256 + i as u64;
        if a >= limit || b >= limit {
            return Err(Error::Corrupt);
        }
        rules.push((a as u32, b as u32));
    }

    let symbol_limit = 256 + rule_count as u64;
    let mut ex = Expander::from_pairs(&rules, raw_len);
    for _ in 0..seq_count {
        let s = varint::get(src, &mut pos)?;
        if s >= symbol_limit {
            return Err(Error::Corrupt);
        }
        ex.push(s as u32)?;
    }

    if pos != src.len() {
        return Err(Error::Corrupt);
    }
    ex.finish()
}

pub(crate) struct Expander {
    start: Vec<usize>,
    kids: Vec<u32>,
    raw_len: usize,
    out: Vec<u8>,
    stack: Vec<u32>,
}

impl Expander {
    pub(crate) fn new(start: Vec<usize>, kids: Vec<u32>, raw_len: usize) -> Self {
        Expander {
            start,
            kids,
            raw_len,
            out: Vec::with_capacity(raw_len.min(1 << 26)),
            stack: Vec::new(),
        }
    }

    fn from_pairs(rules: &[(u32, u32)], raw_len: usize) -> Self {
        let start = (0..=rules.len()).map(|i| i * 2).collect();
        let kids = rules.iter().flat_map(|&(a, b)| [a, b]).collect();
        Self::new(start, kids, raw_len)
    }

    pub(crate) fn push(&mut self, symbol: u32) -> Result<()> {
        self.stack.push(symbol);
        while let Some(x) = self.stack.pop() {
            if x < 256 {
                if self.out.len() >= self.raw_len {
                    return Err(Error::Corrupt);
                }
                self.out.push(x as u8);
            } else {
                let r = (x - 256) as usize;
                for j in (self.start[r]..self.start[r + 1]).rev() {
                    self.stack.push(self.kids[j]);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn finish(self) -> Result<Vec<u8>> {
        if self.out.len() != self.raw_len {
            return Err(Error::Corrupt);
        }
        Ok(self.out)
    }
}
