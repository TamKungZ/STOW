use crate::error::Result;

mod wav;

pub struct Special {
    pub id: u8,
    pub name: &'static str,
    pub detect: fn(&[u8]) -> bool,
    pub encode: fn(&[u8]) -> Option<Vec<u8>>,
    pub decode: fn(&[u8], usize) -> Result<Vec<u8>>,
}

pub const BASE_ID: u8 = 16;

pub const ALL: &[Special] = &[wav::SPECIAL];

pub fn find(id: u8) -> Option<&'static Special> {
    ALL.iter().find(|s| s.id == id)
}
