use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const LAV_MAGIC: [u8; 4] = [b'L', b'A', b'V', 0x12];
pub const LAV_HEADER_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressWidth {
    Bits16,
    Bits24,
    Bits32,
}

impl AddressWidth {
    pub const fn bits(self) -> u8 {
        match self {
            Self::Bits16 => 16,
            Self::Bits24 => 24,
            Self::Bits32 => 32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphicsMode {
    Mono,
    Color4,
    Color8,
}

impl GraphicsMode {
    pub const fn bits_per_pixel(self) -> u8 {
        match self {
            Self::Mono => 1,
            Self::Color4 => 4,
            Self::Color8 => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramHeader {
    pub flags: u8,
    pub address_width: AddressWidth,
    pub graphics_mode: GraphicsMode,
    pub has_pointer: bool,
    pub width: u16,
    pub height: u16,
    pub reserved: [u8; 5],
}

impl ProgramHeader {
    fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < LAV_HEADER_SIZE {
            return Err(Error::FileTooShort {
                expected: LAV_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let magic = [data[0], data[1], data[2], data[3]];
        if magic != LAV_MAGIC {
            return Err(Error::InvalidMagic(magic));
        }

        let flags = data[8];
        let address_width = if flags & 0x10 != 0 {
            AddressWidth::Bits32
        } else if flags & 0x80 != 0 {
            AddressWidth::Bits24
        } else {
            AddressWidth::Bits16
        };
        let graphics_mode = match flags & 0x60 {
            0x40 => GraphicsMode::Color4,
            0x60 => GraphicsMode::Color8,
            _ => GraphicsMode::Mono,
        };
        let width = u16::from(data[9]).saturating_mul(16).clamp(160, 320);
        let height = u16::from(data[10]).saturating_mul(16).clamp(80, 240);

        Ok(Self {
            flags,
            address_width,
            graphics_mode,
            has_pointer: flags & 1 != 0,
            width,
            height,
            reserved: data[11..16].try_into().expect("fixed header slice"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    header: ProgramHeader,
    image: Vec<u8>,
}

impl Program {
    pub fn load(data: &[u8]) -> Result<Self> {
        let header = ProgramHeader::parse(data)?;
        Ok(Self {
            header,
            image: data.to_vec(),
        })
    }

    pub const fn header(&self) -> &ProgramHeader {
        &self.header
    }

    pub fn image(&self) -> &[u8] {
        &self.image
    }

    pub fn bytecode(&self) -> &[u8] {
        &self.image[LAV_HEADER_SIZE..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(flags: u8, width: u8, height: u8) -> Vec<u8> {
        let mut data = vec![0; LAV_HEADER_SIZE + 1];
        data[..4].copy_from_slice(&LAV_MAGIC);
        data[8] = flags;
        data[9] = width;
        data[10] = height;
        data[16] = 0x7f;
        data
    }

    #[test]
    fn parses_color8_program_header() {
        let program = Program::load(&image(0xf0, 15, 10)).unwrap();
        assert_eq!(program.header.address_width, AddressWidth::Bits32);
        assert_eq!(program.header.graphics_mode, GraphicsMode::Color8);
        assert_eq!((program.header.width, program.header.height), (240, 160));
        assert!(!program.header.has_pointer);
        assert_eq!(program.bytecode(), &[0x7f]);
    }

    #[test]
    fn applies_legacy_dimension_limits() {
        let program = Program::load(&image(0x41, 0, 255)).unwrap();
        assert_eq!(program.header.address_width, AddressWidth::Bits16);
        assert_eq!(program.header.graphics_mode, GraphicsMode::Color4);
        assert_eq!((program.header.width, program.header.height), (160, 240));
        assert!(program.header.has_pointer);
    }

    #[test]
    fn rejects_invalid_magic() {
        let error = Program::load(&[0; LAV_HEADER_SIZE]).unwrap_err();
        assert_eq!(error, Error::InvalidMagic([0; 4]));
    }

    #[test]
    fn rejects_truncated_input() {
        let error = Program::load(&LAV_MAGIC).unwrap_err();
        assert_eq!(
            error,
            Error::FileTooShort {
                expected: LAV_HEADER_SIZE,
                actual: 4,
            }
        );
    }
}
