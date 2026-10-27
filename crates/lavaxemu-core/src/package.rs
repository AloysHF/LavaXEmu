use encoding_rs::GBK;

use crate::{Emulator, Error, Program, Result};

impl Emulator {
    /// Loads a standalone LAV program or the first program and all files in a PAC.
    pub fn load_content(data: &[u8]) -> Result<Self> {
        if !data.starts_with(b"PAC ") {
            return Ok(Self::new(Program::load(data)?));
        }
        if data.len() < 18 {
            return Err(Error::InvalidPackage("truncated header"));
        }
        let count = u16::from_le_bytes([data[16], data[17]]);
        let mut offset = 18usize;
        let mut entries = Vec::new();
        let mut program = None;
        for _ in 0..count {
            let header = data
                .get(offset..offset + 64)
                .ok_or(Error::InvalidPackage("truncated entry header"))?;
            offset += 64;
            let length = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
            let end = offset
                .checked_add(length)
                .ok_or(Error::InvalidPackage("entry length overflow"))?;
            let bytes = data
                .get(offset..end)
                .ok_or(Error::InvalidPackage("truncated entry data"))?;
            offset = end;
            if length == 0 {
                continue;
            }
            let name = &header[4..];
            let terminator = name
                .iter()
                .position(|&byte| byte == 0)
                .ok_or(Error::InvalidPackage("unterminated entry name"))?;
            let (name, _, errors) = GBK.decode(&name[..terminator]);
            let name = name.replace('\\', "/");
            if errors
                || name.is_empty()
                || name.starts_with('/')
                || name.contains(':')
                || name.split('/').any(|part| part == "..")
            {
                return Err(Error::InvalidPackage("invalid entry path"));
            }
            if program.is_none() && name.to_ascii_lowercase().ends_with(".lav") {
                program = Some(Program::load(bytes)?);
            }
            entries.push((name, bytes));
        }
        let mut emulator = Self::new(program.ok_or(Error::InvalidPackage("no LAV program"))?);
        for (name, bytes) in entries {
            if !emulator.files_mut().import_file(&name, bytes.to_vec()) {
                return Err(Error::InvalidPackage("invalid virtual file path"));
            }
        }
        Ok(emulator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FrameStatus, LAV_HEADER_SIZE, LAV_MAGIC};

    fn package(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut data = vec![0; 18];
        data[..4].copy_from_slice(b"PAC ");
        data[16..18].copy_from_slice(&(entries.len() as u16).to_le_bytes());
        for (name, bytes) in entries {
            let mut header = [0; 64];
            header[..4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
            let name = GBK.encode(name).0;
            header[4..4 + name.len()].copy_from_slice(&name);
            data.extend_from_slice(&header);
            data.extend_from_slice(bytes);
        }
        data
    }

    #[test]
    fn loads_the_first_program_and_gbk_device_resources() {
        let mut lav = vec![0; LAV_HEADER_SIZE];
        lav[..4].copy_from_slice(&LAV_MAGIC);
        lav.extend_from_slice(&[0x40]);
        let data = package(&[
            ("LAVA\\first.LAV", &lav),
            ("LavaData/图片.dat", &[1, 2, 3]),
            ("LAVA/second.lav", b"not the selected program"),
        ]);
        let mut emulator = Emulator::load_content(&data).unwrap();
        assert_eq!(
            emulator.files().file("/LavaData/图片.dat"),
            Some(&[1, 2, 3][..])
        );
        assert_eq!(emulator.run_frame().unwrap().status, FrameStatus::Halted(0));
        assert_eq!(
            emulator.files().file("/lava/first.lav"),
            Some(lav.as_slice())
        );
        assert!(Emulator::load_content(&lav).is_ok());
    }

    #[test]
    fn rejects_truncation_missing_programs_and_escaping_paths() {
        let data = package(&[("LavaData/data.dat", &[1, 2, 3])]);
        for end in 4..data.len() {
            assert!(Emulator::load_content(&data[..end]).is_err());
        }
        assert!(Emulator::load_content(&data).is_err());
        for name in ["../escape.lav", "C:/escape.lav", "/escape.lav"] {
            assert!(Emulator::load_content(&package(&[(name, &[1])])).is_err());
        }
    }
}
