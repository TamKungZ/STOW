use crate::codec;
use crate::crc32::crc32;
use crate::error::{Error, Result};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

const MAX_THREADS: usize = 4;

const MAGIC: [u8; 4] = *b"STOW";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 10;
const ENTRY_FIXED_LEN: usize = 2 + 1 + 8 + 8 + 4;

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > u16::MAX as usize {
        return Err(Error::InvalidName);
    }
    if name.contains(['\0', '\\', ':']) || name.starts_with('/') {
        return Err(Error::InvalidName);
    }
    for segment in name.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(Error::InvalidName);
        }
    }
    Ok(())
}

#[derive(Default)]
pub struct Writer {
    entries: Vec<(String, Vec<u8>)>,
    names: HashSet<String>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: &str, data: &[u8]) -> Result<()> {
        validate_name(name)?;
        if self.entries.len() >= u32::MAX as usize {
            return Err(Error::TooLarge);
        }
        if !self.names.insert(name.to_owned()) {
            return Err(Error::DuplicateName);
        }
        self.entries.push((name.to_owned(), data.to_vec()));
        Ok(())
    }

    fn encode_all(&self) -> Vec<(u8, Vec<u8>)> {
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(self.entries[i].1.len()));
        let workers = thread::available_parallelism().map_or(1, |n| n.get()).min(MAX_THREADS);
        let next = AtomicUsize::new(0);
        let done = Mutex::new(vec![None; self.entries.len()]);
        thread::scope(|scope| {
            for _ in 0..workers.min(order.len()) {
                scope.spawn(|| loop {
                    let slot = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&i) = order.get(slot) else { break };
                    let encoded = codec::smallest(&self.entries[i].1);
                    done.lock().expect("encoder thread panicked")[i] = Some(encoded);
                });
            }
        });
        done.into_inner()
            .expect("encoder thread panicked")
            .into_iter()
            .map(|e| e.expect("every entry is encoded"))
            .collect()
    }

    pub fn finish(&self) -> Vec<u8> {
        let mut encoded = self.encode_all().into_iter();
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC);
        out.push(VERSION);
        out.push(0);
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for (name, data) in &self.entries {
            let (id, payload) = encoded.next().expect("one result per entry");
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.push(id);
            out.extend_from_slice(&(data.len() as u64).to_le_bytes());
            out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
            out.extend_from_slice(&crc32(data).to_le_bytes());
            out.extend_from_slice(&payload);
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub name: String,
    pub codec: u8,
    pub raw_len: u64,
    pub stored_len: u64,
    pub crc32: u32,
    offset: usize,
}

pub struct Reader {
    bytes: Vec<u8>,
    entries: Vec<EntryInfo>,
}

fn take<'a>(bytes: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8]> {
    let end = pos.checked_add(len).ok_or(Error::Truncated)?;
    let slice = bytes.get(*pos..end).ok_or(Error::Truncated)?;
    *pos = end;
    Ok(slice)
}

impl Reader {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < MAGIC.len() {
            return Err(Error::Truncated);
        }
        if bytes[..4] != MAGIC {
            return Err(Error::InvalidMagic);
        }
        if bytes.len() < HEADER_LEN {
            return Err(Error::Truncated);
        }
        if bytes[4] != VERSION || bytes[5] != 0 {
            return Err(Error::UnsupportedVersion);
        }
        let count = u32::from_le_bytes(bytes[6..10].try_into().unwrap()) as usize;
        if count > (bytes.len() - HEADER_LEN) / (ENTRY_FIXED_LEN + 1) {
            return Err(Error::Truncated);
        }

        let mut pos = HEADER_LEN;
        let mut entries = Vec::with_capacity(count);
        let mut seen = HashSet::new();
        for _ in 0..count {
            let name_len = u16::from_le_bytes(take(&bytes, &mut pos, 2)?.try_into().unwrap());
            let name = std::str::from_utf8(take(&bytes, &mut pos, name_len as usize)?)
                .map_err(|_| Error::InvalidName)?
                .to_owned();
            validate_name(&name)?;
            if !seen.insert(name.clone()) {
                return Err(Error::DuplicateName);
            }
            let codec = take(&bytes, &mut pos, 1)?[0];
            let raw_len = u64::from_le_bytes(take(&bytes, &mut pos, 8)?.try_into().unwrap());
            let stored_len = u64::from_le_bytes(take(&bytes, &mut pos, 8)?.try_into().unwrap());
            let crc = u32::from_le_bytes(take(&bytes, &mut pos, 4)?.try_into().unwrap());
            let stored = usize::try_from(stored_len).map_err(|_| Error::TooLarge)?;
            let offset = pos;
            take(&bytes, &mut pos, stored)?;
            entries.push(EntryInfo { name, codec, raw_len, stored_len, crc32: crc, offset });
        }
        if pos != bytes.len() {
            return Err(Error::Corrupt);
        }
        Ok(Reader { bytes, entries })
    }

    pub fn entries(&self) -> &[EntryInfo] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn read(&self, index: usize) -> Result<Vec<u8>> {
        let e = self.entries.get(index).ok_or(Error::IndexOutOfRange)?;
        let raw_len = usize::try_from(e.raw_len).map_err(|_| Error::TooLarge)?;
        let payload = &self.bytes[e.offset..e.offset + e.stored_len as usize];
        let data = codec::decode(e.codec, payload, raw_len)?;
        if crc32(&data) != e.crc32 {
            return Err(Error::ChecksumMismatch);
        }
        Ok(data)
    }
}
