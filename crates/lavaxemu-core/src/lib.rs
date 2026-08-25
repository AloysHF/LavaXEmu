mod error;
mod program;

pub use error::{Error, Result};
pub use program::{AddressWidth, GraphicsMode, LAV_HEADER_SIZE, LAV_MAGIC, Program, ProgramHeader};
