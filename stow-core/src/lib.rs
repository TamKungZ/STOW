pub mod codec;
mod container;
mod crc32;
mod error;
pub mod fold;
pub mod rc;
pub mod varint;

pub use container::{validate_name, EntryInfo, Reader, Writer};
pub use crc32::crc32;
pub use error::{Error, Result};

#[cfg(test)]
mod tests;
