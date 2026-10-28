use encoding_rs::GBK;
use serde::{Deserialize, Serialize};

use crate::{BufferTarget, DrawOperation, Emulator, Result, emulator::HostAction};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct FileListState {
    pub(crate) destination: Option<u32>,
    entries: Vec<String>,
    pub(crate) selected: usize,
    pub(crate) first: usize,
    pub(crate) flags: u8,
}

impl FileListState {
    pub(crate) fn selection_is_valid(&self) -> bool {
        self.selected < self.entries.len() && self.first <= self.selected
    }
}

impl Emulator {
    pub(crate) fn begin_file_list(&mut self, destination: u32) -> Result<HostAction> {
        let entries = std::iter::once("..".to_owned())
            .chain(
                self.files
                    .list(self.files.current_directory())
                    .into_iter()
                    .filter(|name| {
                        let (encoded, _, errors) = GBK.encode(name);
                        !errors && encoded.len() <= 14
                    }),
            )
            .collect();
        self.file_list.entries = entries;
        self.file_list.destination = Some(destination);
        if self.file_list.flags & 0x80 == 0 {
            self.file_list.selected = 1;
            self.file_list.first = 0;
        }
        self.file_list.selected = self
            .file_list
            .selected
            .min(self.file_list.entries.len() - 1);
        self.constrain_file_list();
        self.render_file_list();
        Ok(HostAction::WaitForInput)
    }

    pub(crate) fn poll_file_list(&mut self) -> Result<bool> {
        let Some(key) = self.input.pop_key() else {
            return Ok(true);
        };
        let result = match key {
            27 => Some(0),
            13 => {
                Some((self.file_list.selected + 1) as i32 | ((self.file_list.first as i32) << 16))
            }
            29 if self.file_list.flags & 0x40 != 0 => Some(-1),
            25 if self.file_list.flags & 0x20 != 0 => Some(-2),
            _ => None,
        };
        if let Some(result) = result {
            let destination = self.file_list.destination.expect("active file list");
            let mut name = if result == 0 {
                Vec::new()
            } else {
                GBK.encode(&self.file_list.entries[self.file_list.selected])
                    .0
                    .into_owned()
            };
            name.push(0);
            self.write_memory(destination, &name)?;
            self.vm.push_value(result)?;
            self.file_list.destination = None;
            return Ok(false);
        }
        let rows = self.file_list_rows();
        match key {
            20 => self.file_list.selected = self.file_list.selected.saturating_sub(1),
            21 => {
                self.file_list.selected =
                    (self.file_list.selected + 1).min(self.file_list.entries.len() - 1)
            }
            19 => {
                let amount = self.file_list.first.min(rows);
                self.file_list.first -= amount;
                self.file_list.selected = self.file_list.selected.saturating_sub(amount);
            }
            14 if self.file_list.first + rows < self.file_list.entries.len() => {
                self.file_list.first += rows;
                self.file_list.selected =
                    (self.file_list.selected + rows).min(self.file_list.entries.len() - 1);
            }
            _ => return Ok(true),
        }
        self.constrain_file_list();
        self.render_file_list();
        Ok(true)
    }

    fn file_list_rows(&self) -> usize {
        (usize::from(self.display.height()) / 16).max(1)
    }

    fn constrain_file_list(&mut self) {
        let rows = self.file_list_rows();
        self.file_list.first = self.file_list.first.min(self.file_list.selected);
        if self.file_list.selected >= self.file_list.first + rows {
            self.file_list.first = self.file_list.selected + 1 - rows;
        }
    }

    fn render_file_list(&mut self) {
        self.console
            .set_mode(self.display.width(), self.display.height(), false);
        for (row, name) in self
            .file_list
            .entries
            .iter()
            .skip(self.file_list.first)
            .take(self.file_list_rows())
            .enumerate()
        {
            self.console.locate(0, row);
            for byte in GBK.encode(name).0.iter() {
                self.console.write(*byte);
            }
        }
        self.console.render(&mut self.display);
        let top = (self.file_list.selected - self.file_list.first) as i32 * 16;
        self.display.draw_rectangle(
            BufferTarget::Front,
            0,
            top,
            i32::from(self.display.width()) - 1,
            top + 15,
            true,
            DrawOperation::Invert,
        );
        self.write_graphics_memory();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FrameStatus, LAV_HEADER_SIZE, LAV_MAGIC, Program};

    fn emulator(prefix: &[u8]) -> Emulator {
        let mut image = vec![0; LAV_HEADER_SIZE];
        image[..4].copy_from_slice(&LAV_MAGIC);
        image[9] = 10;
        image[10] = 5;
        image.extend_from_slice(prefix);
        image.extend_from_slice(&[0x02, 0, 0x20, 0xc1, 0x38, 0x40]);
        Emulator::new(Program::load(&image).unwrap())
    }

    #[test]
    fn waits_moves_and_returns_the_selected_gbk_filename() {
        let mut emulator = emulator(&[]);
        emulator.files_mut().import_file("a.dat", vec![]);
        emulator.files_mut().import_file("测试.dat", vec![]);
        emulator
            .files_mut()
            .import_file("name-too-long-for-lvm.dat", vec![]);
        assert_eq!(
            emulator.run_frame().unwrap().status,
            FrameStatus::WaitingForInput
        );
        assert_eq!(emulator.file_list.entries, ["..", "a.dat", "测试.dat"]);
        let before = emulator.display().indexed_frame().to_vec();
        emulator.input_mut().set_keys([21]);
        assert_eq!(
            emulator.run_frame().unwrap().status,
            FrameStatus::WaitingForInput
        );
        assert!(before != emulator.display().indexed_frame());
        emulator.input_mut().set_keys([13]);
        assert_eq!(emulator.run_frame().unwrap().status, FrameStatus::Halted(0));
        assert_eq!(emulator.vm().result(), 3);
        let mut expected = GBK.encode("测试.dat").0.into_owned();
        expected.push(0);
        assert_eq!(
            &emulator.vm().memory()[0x2000..0x2000 + expected.len()],
            expected
        );
    }

    #[test]
    fn pages_and_returns_the_scroll_position() {
        let mut emulator = emulator(&[]);
        for index in 0..10 {
            emulator
                .files_mut()
                .import_file(&format!("{index:02}.dat"), vec![]);
        }
        emulator.run_frame().unwrap();
        emulator.input_mut().set_keys([14]);
        emulator.run_frame().unwrap();
        assert_eq!(
            (emulator.file_list.selected, emulator.file_list.first),
            (6, 5)
        );
        emulator.input_mut().set_keys([13]);
        emulator.run_frame().unwrap();
        assert_eq!(emulator.vm().result(), (5 << 16) | 7);
        assert_eq!(&emulator.vm().memory()[0x2000..0x2007], b"05.dat\0");
    }

    #[test]
    fn setlist_preserves_selection_and_enables_help_and_delete() {
        for (key, result, name) in [(29, -1, "b.dat"), (25, -2, "b.dat"), (27, 0, "")] {
            let mut emulator = emulator(&[0x01, 0xe0, 0x01, 3, 0xce]);
            emulator.files_mut().import_file("a.dat", vec![]);
            emulator.files_mut().import_file("b.dat", vec![]);
            emulator.run_frame().unwrap();
            assert_eq!(emulator.file_list.selected, 2);
            emulator.input_mut().set_keys([key]);
            emulator.run_frame().unwrap();
            assert_eq!(emulator.vm().result(), result);
            let expected = [name.as_bytes(), &[0]].concat();
            assert_eq!(
                &emulator.vm().memory()[0x2000..0x2000 + expected.len()],
                expected
            );
            assert!(emulator.files().file("b.dat").is_some());
        }
    }

    #[test]
    fn restores_a_pending_file_dialog_and_clears_it_on_reset() {
        let mut emulator = emulator(&[]);
        emulator.files_mut().import_file("a.dat", vec![]);
        emulator.files_mut().import_file("b.dat", vec![]);
        emulator.run_frame().unwrap();
        let state = emulator.save_state().unwrap();
        emulator.input_mut().set_keys([21]);
        emulator.run_frame().unwrap();
        assert_eq!(emulator.file_list.selected, 2);
        emulator.load_state(&state).unwrap();
        assert_eq!(emulator.file_list.selected, 1);
        emulator.input_mut().set_keys([13]);
        emulator.run_frame().unwrap();
        assert_eq!(emulator.vm().result(), 2);
        assert_eq!(&emulator.vm().memory()[0x2000..0x2006], b"a.dat\0");
        emulator.reset();
        assert!(emulator.file_list.destination.is_none());
    }
}
