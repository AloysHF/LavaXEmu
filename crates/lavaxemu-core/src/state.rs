use crc32fast::hash;

use crate::{Emulator, Error, GUEST_MEMORY_SIZE, Result};

const STATE_MAGIC: [u8; 4] = *b"LXST";
const STATE_VERSION: u16 = 2;
const STATE_HEADER_SIZE: usize = 14;
const MAX_STATE_SIZE: usize = 64 * 1024 * 1024;

impl Emulator {
    pub fn save_state(&self) -> Result<Vec<u8>> {
        let payload =
            bincode::serialize(self).map_err(|error| Error::InvalidSaveState(error.to_string()))?;
        if payload.len() > MAX_STATE_SIZE {
            return Err(Error::InvalidSaveState(
                "state exceeds size limit".to_owned(),
            ));
        }

        let payload_len = u32::try_from(payload.len())
            .map_err(|_| Error::InvalidSaveState("state is too large".to_owned()))?;
        let mut state = Vec::with_capacity(STATE_HEADER_SIZE + payload.len());
        state.extend_from_slice(&STATE_MAGIC);
        state.extend_from_slice(&STATE_VERSION.to_le_bytes());
        state.extend_from_slice(&payload_len.to_le_bytes());
        state.extend_from_slice(&hash(&payload).to_le_bytes());
        state.extend_from_slice(&payload);
        Ok(state)
    }

    pub fn load_state(&mut self, state: &[u8]) -> Result<()> {
        if state.len() < STATE_HEADER_SIZE || state[..4] != STATE_MAGIC {
            return Err(Error::InvalidSaveState("invalid header".to_owned()));
        }

        let version = u16::from_le_bytes([state[4], state[5]]);
        if version != STATE_VERSION {
            return Err(Error::InvalidSaveState(format!(
                "unsupported version {version}"
            )));
        }

        let payload_len =
            u32::from_le_bytes(state[6..10].try_into().expect("fixed state field")) as usize;
        if payload_len > MAX_STATE_SIZE || state.len() < STATE_HEADER_SIZE + payload_len {
            return Err(Error::InvalidSaveState("invalid payload length".to_owned()));
        }
        let payload = &state[STATE_HEADER_SIZE..STATE_HEADER_SIZE + payload_len];
        let expected_checksum =
            u32::from_le_bytes(state[10..14].try_into().expect("fixed state field"));
        if hash(payload) != expected_checksum {
            return Err(Error::InvalidSaveState("checksum mismatch".to_owned()));
        }

        let restored: Self = bincode::deserialize(payload)
            .map_err(|error| Error::InvalidSaveState(error.to_string()))?;
        if restored.vm.program() != self.vm.program() {
            return Err(Error::InvalidSaveState(
                "state belongs to a different program".to_owned(),
            ));
        }
        if restored.vm.memory().len() != GUEST_MEMORY_SIZE
            || restored.display.width() != self.display.width()
            || restored.display.height() != self.display.height()
        {
            return Err(Error::InvalidSaveState(
                "state contains invalid machine dimensions".to_owned(),
            ));
        }

        *self = restored;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{LAV_HEADER_SIZE, LAV_MAGIC, Program};

    use super::*;

    fn program(last_opcode: u8) -> Program {
        let mut image = vec![0; LAV_HEADER_SIZE + 1];
        image[..4].copy_from_slice(&LAV_MAGIC);
        image[9] = 10;
        image[10] = 5;
        image[LAV_HEADER_SIZE] = last_opcode;
        Program::load(&image).unwrap()
    }

    #[test]
    fn round_trips_machine_state() {
        let mut emulator = Emulator::new(program(0x04));
        emulator.vm_mut().memory_mut()[0x100] = 42;
        emulator.set_command_line(b"one two".to_vec());
        emulator.files_mut().import_file("/save.dat", vec![1, 2, 3]);
        let state = emulator.save_state().unwrap();

        emulator.vm_mut().memory_mut()[0x100] = 0;
        emulator.reset();
        emulator.load_state(&state).unwrap();

        assert_eq!(emulator.vm().memory()[0x100], 42);
        assert_eq!(emulator.files().file("/save.dat"), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn rejects_corruption_and_other_programs() {
        let emulator = Emulator::new(program(0x04));
        let mut state = emulator.save_state().unwrap();
        *state.last_mut().unwrap() ^= 1;
        assert!(emulator.clone().load_state(&state).is_err());

        let valid_state = emulator.save_state().unwrap();
        let mut other = Emulator::new(program(0x05));
        assert!(other.load_state(&valid_state).is_err());
    }
}
