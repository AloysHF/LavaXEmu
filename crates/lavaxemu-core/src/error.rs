use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("LAV file is too short: expected at least {expected} bytes, got {actual}")]
    FileTooShort { expected: usize, actual: usize },
    #[error("invalid LAV magic: {0:02x?}")]
    InvalidMagic([u8; 4]),
}
