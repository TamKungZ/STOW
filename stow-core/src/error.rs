use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidMagic,
    UnsupportedVersion,
    Truncated,
    Corrupt,
    ChecksumMismatch,
    UnknownCodec,
    InvalidName,
    DuplicateName,
    TooLarge,
    IndexOutOfRange,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn code(self) -> i32 {
        match self {
            Error::InvalidMagic => 16,
            Error::UnsupportedVersion => 17,
            Error::Truncated => 18,
            Error::Corrupt => 19,
            Error::ChecksumMismatch => 20,
            Error::UnknownCodec => 21,
            Error::InvalidName => 22,
            Error::DuplicateName => 23,
            Error::TooLarge => 24,
            Error::IndexOutOfRange => 25,
        }
    }

    pub fn from_code(code: i32) -> Option<Error> {
        Some(match code {
            16 => Error::InvalidMagic,
            17 => Error::UnsupportedVersion,
            18 => Error::Truncated,
            19 => Error::Corrupt,
            20 => Error::ChecksumMismatch,
            21 => Error::UnknownCodec,
            22 => Error::InvalidName,
            23 => Error::DuplicateName,
            24 => Error::TooLarge,
            25 => Error::IndexOutOfRange,
            _ => return None,
        })
    }

    pub fn message(self) -> &'static str {
        match self {
            Error::InvalidMagic => "not a STOW archive",
            Error::UnsupportedVersion => "unsupported STOW version",
            Error::Truncated => "archive is truncated",
            Error::Corrupt => "archive is corrupt",
            Error::ChecksumMismatch => "checksum mismatch",
            Error::UnknownCodec => "unknown codec",
            Error::InvalidName => "invalid entry name",
            Error::DuplicateName => "duplicate entry name",
            Error::TooLarge => "data too large",
            Error::IndexOutOfRange => "entry index out of range",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for Error {}
