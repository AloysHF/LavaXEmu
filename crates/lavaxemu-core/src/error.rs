use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("LAV file is too short: expected at least {expected} bytes, got {actual}")]
    FileTooShort { expected: usize, actual: usize },
    #[error("invalid LAV magic: {0:02x?}")]
    InvalidMagic([u8; 4]),
    #[error("unexpected end of bytecode at program offset 0x{pc:x}")]
    UnexpectedEnd { pc: usize },
    #[error("guest memory access 0x{address:x}..0x{end:x} is out of range")]
    MemoryOutOfBounds { address: u32, end: u32 },
    #[error("LavaX evaluation stack underflow")]
    StackUnderflow,
    #[error("LavaX evaluation stack overflow")]
    StackOverflow,
    #[error("invalid LavaX opcode 0x{opcode:02x} at program offset 0x{pc:x}")]
    InvalidOpcode { opcode: u8, pc: usize },
    #[error("division by zero at program offset 0x{pc:x}")]
    DivisionByZero { pc: usize },
    #[error("invalid jump target 0x{target:x}")]
    InvalidJump { target: usize },
    #[error("unsupported LavaX feature: {0}")]
    UnsupportedFeature(&'static str),
    #[error("invalid save state: {0}")]
    InvalidSaveState(String),
}
