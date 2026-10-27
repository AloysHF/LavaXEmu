use encoding_rs::GBK;

use crate::{Emulator, Error, Program, Result};

impl Emulator {
    /// Loads a standalone LAV program or the first program and all files in a PAC.
    pub fn load_content(data: &[u8]) -> Result<Self> {
        if !data.starts_with(b"PAC ") {
            return Ok(Self::new(Program::load(data)?));
        }
        let entries = package_entries(data)?;
        let program = entries
            .iter()
            .find(|(name, _)| name.to_ascii_lowercase().ends_with(".lav"))
            .ok_or(Error::InvalidPackage("no LAV program"))?;
        let mut emulator = Self::new(Program::load(program.1)?);
        for (name, bytes) in entries {
            emulator.files_mut().import_file(&name, bytes.to_vec());
        }
        Ok(emulator)
    }

    /// Imports missing resources only when the package contains this exact program.
    pub fn import_matching_package(&mut self, data: &[u8]) -> Result<bool> {
        let entries = package_entries(data)?;
        if !entries.iter().any(|(name, bytes)| {
            name.to_ascii_lowercase().ends_with(".lav") && *bytes == self.vm.program().image()
        }) {
            return Ok(false);
        }
        for (name, bytes) in entries {
            if self.files().file(&name).is_none() {
                self.files_mut().import_file(&name, bytes.to_vec());
            }
        }
        Ok(true)
    }
}

fn package_entries(data: &[u8]) -> Result<Vec<(String, &[u8])>> {
    if data.len() < 18 || !data.starts_with(b"PAC ") {
        return Err(Error::InvalidPackage("invalid header"));
    }
    let count = u16::from_le_bytes([data[16], data[17]]);
    let mut offset = 18usize;
    let mut entries = Vec::new();
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
        entries.push((name, bytes));
    }
    Ok(entries)
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

    #[test]
    fn matching_packages_only_supply_missing_files_and_preserve_loose_saves() {
        let mut lav = vec![0; LAV_HEADER_SIZE];
        lav[..4].copy_from_slice(&LAV_MAGIC);
        lav.push(0x40);
        let mut emulator = Emulator::load_content(&lav).unwrap();
        emulator.files_mut().import_file("slot.sav", vec![9]);
        emulator
            .files_mut()
            .import_file("/LavaData/sprite.dat", vec![8]);
        let data = package(&[
            ("LAVA/helper.lav", b"another program"),
            ("LAVA/main.lav", &lav),
            ("LavaData/slot.sav", &[0]),
            ("LavaData/sprite.dat", &[0]),
            ("CBMap/stagepack.dat", &[1, 2, 3]),
        ]);
        assert!(emulator.import_matching_package(&data).unwrap());
        assert_eq!(emulator.files().file("/LavaData/slot.sav"), Some(&[9][..]));
        assert_eq!(
            emulator.files().file("/LavaData/sprite.dat"),
            Some(&[8][..])
        );
        assert_eq!(
            emulator.files().file("/CBMap/stagepack.dat"),
            Some(&[1, 2, 3][..])
        );
        let handle = emulator
            .files_mut()
            .open("/LavaData/slot.sav", "w")
            .unwrap();
        emulator.files_mut().write(handle, &[7]);
        emulator.files_mut().close(handle);
        assert_eq!(
            emulator.files_mut().take_dirty_files(),
            [("/slot.sav".to_owned(), vec![7])]
        );
    }

    #[test]
    fn mismatched_or_incomplete_packages_do_not_import_any_files() {
        let mut lav = vec![0; LAV_HEADER_SIZE];
        lav[..4].copy_from_slice(&LAV_MAGIC);
        lav.push(0x40);
        let mut emulator = Emulator::load_content(&lav).unwrap();
        let mut other = lav.clone();
        other.push(0x40);
        assert!(
            !emulator
                .import_matching_package(&package(&[("main.lav", &other), ("data.dat", &[1])]))
                .unwrap()
        );
        let data = package(&[("main.lav", &lav), ("data.dat", &[1])]);
        assert!(
            emulator
                .import_matching_package(&data[..data.len() - 1])
                .is_err()
        );
        assert!(emulator.files().files().is_empty());
    }
}
