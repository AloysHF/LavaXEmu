use crate::{BufferTarget, Emulator, GraphicsMode, Result};

const BUFFER_SIZE: usize = 160 * 80 / 8;
pub(crate) const MEMORY_SIZE: usize = BUFFER_SIZE * 2;

impl Emulator {
    pub(crate) fn request_graphics_memory(&mut self, back: bool) -> Result<()> {
        // GVmaker 1.0 places two packed monochrome buffers below runtime RAM.
        let supported = self.display.width() == 160
            && self.display.height() == 80
            && self.display.graphics_mode() == GraphicsMode::Mono;
        if supported && !self.legacy_graphics_memory {
            self.legacy_graphics_memory = true;
            self.write_graphics_memory();
        }
        self.vm.push_value(if supported && back {
            BUFFER_SIZE as i32
        } else {
            0
        })
    }

    fn graphics_memory_enabled(&self) -> bool {
        self.legacy_graphics_memory && self.display.graphics_mode() == GraphicsMode::Mono
    }

    fn packed_graphics(&self) -> Vec<u8> {
        let mut packed = self.display.capture(BufferTarget::Front, 0, 0, 160, 80);
        packed.extend(self.display.capture(BufferTarget::Back, 0, 0, 160, 80));
        packed
    }

    pub(crate) fn write_graphics_memory(&mut self) {
        if !self.graphics_memory_enabled() {
            return;
        }
        let packed = self.packed_graphics();
        self.vm.memory_mut()[..MEMORY_SIZE].copy_from_slice(&packed);
        self.vm.graphics_memory_dirty = false;
    }

    pub(crate) fn read_graphics_memory(&mut self) {
        if !self.graphics_memory_enabled() || !self.vm.graphics_memory_dirty {
            return;
        }
        for (target, offset) in [(BufferTarget::Front, 0), (BufferTarget::Back, BUFFER_SIZE)] {
            self.display.blit(
                target,
                0,
                0,
                160,
                80,
                &self.vm.memory()[offset..offset + BUFFER_SIZE],
                0,
                false,
            );
        }
        self.vm.graphics_memory_dirty = false;
    }

    pub(crate) fn sync_graphics_system_call(&mut self, call: u8) {
        if !self.graphics_memory_enabled() {
            return;
        }
        // Only drawing calls modify pixels; memory-writing calls modify RAM.
        if matches!(call, 0 | 2 | 6 | 8..=10 | 14 | 22 | 24 | 25 | 69 | 75) {
            self.write_graphics_memory();
        } else {
            self.read_graphics_memory();
        }
    }

    pub(crate) fn write_graphics_region(
        &mut self,
        target: BufferTarget,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    ) {
        if !self.graphics_memory_enabled() {
            return;
        }
        let left = x0.min(x1).clamp(0, 160) as usize / 8 * 8;
        let right = (x0.max(x1) + 1).clamp(0, 160) as usize;
        let right = right.div_ceil(8) * 8;
        let top = y0.min(y1).clamp(0, 80) as usize;
        let bottom = (y0.max(y1) + 1).clamp(0, 80) as usize;
        if right <= left || bottom <= top {
            return;
        }
        let packed = self.display.capture(
            target,
            left as i32,
            top as i32,
            (right - left) as u16,
            (bottom - top) as u16,
        );
        let stride = (right - left) / 8;
        let offset = if target == BufferTarget::Back {
            BUFFER_SIZE
        } else {
            0
        };
        for (row, bytes) in packed.chunks_exact(stride).enumerate() {
            let start = offset + (top + row) * 20 + left / 8;
            self.vm.memory_mut()[start..start + stride].copy_from_slice(bytes);
        }
        self.vm.graphics_memory_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FrameStatus, LAV_HEADER_SIZE, LAV_MAGIC, Program};

    fn emulator(bytecode: &[u8]) -> Emulator {
        let mut image = vec![0; LAV_HEADER_SIZE];
        image[..4].copy_from_slice(&LAV_MAGIC);
        image[9] = 10;
        image[10] = 5;
        image.extend_from_slice(bytecode);
        Emulator::new(Program::load(&image).unwrap())
    }

    #[test]
    fn direct_guest_assignments_update_the_front_buffer() {
        let mut emulator = emulator(&[
            0x1b, 0x37, // front-buffer byte descriptor
            0x01, 0xa5, 0x35, 0x38, 0x40,
        ]);
        assert_eq!(emulator.run_frame().unwrap().status, FrameStatus::Halted(0));
        assert_eq!(
            &emulator.display().indexed_frame()[..8],
            &[1, 0, 1, 0, 0, 1, 0, 1]
        );
        assert_eq!(emulator.vm.memory()[0], 0xa5);
    }

    #[test]
    fn a_box_crossing_a_byte_boundary_updates_front_ram_without_touching_back_ram() {
        let mut emulator = emulator(&[
            0x1b, 0x38, 0x01, 4, 0x01, 3, 0x01, 11, 0x01, 3, 0x01, 1, 0x01, 1, 0x97, 0x40,
        ]);
        assert_eq!(emulator.run_frame().unwrap().status, FrameStatus::Halted(0));
        assert_eq!(&emulator.vm.memory()[60..62], &[0x0f, 0xf0]);
        assert!(
            emulator.vm.memory()[BUFFER_SIZE..MEMORY_SIZE]
                .iter()
                .all(|&byte| byte == 0)
        );
        assert_eq!(
            &emulator.display().indexed_frame()[3 * 160 + 4..3 * 160 + 12],
            &[1; 8]
        );
    }

    #[test]
    fn drawing_bulk_memory_and_state_restore_share_the_same_buffers() {
        let mut emulator = emulator(&[
            0x1b, 0x38, // enable direct graphics access
            0x01, 0, 0x01, 0xa5, 0x01, 1, 0xac, // memset front byte
            0x01, 0, 0x01, 0, 0x01, 0, 0x94, // clear its first pixel
            0x42, 0x01, 0, 0x01, 1, 0xad, // copy front byte to back buffer
            0x89, 0x89, 0x40,
        ]);
        assert_eq!(emulator.run_frame().unwrap().status, FrameStatus::Presented);
        assert_eq!(emulator.vm.memory()[0], 0x25);
        assert_eq!(emulator.vm.memory()[BUFFER_SIZE], 0x25);
        assert_eq!(
            &emulator.display().indexed_frame()[..8],
            &[0, 0, 1, 0, 0, 1, 0, 1]
        );
        let state = emulator.save_state().unwrap();
        emulator.reset();
        assert!(!emulator.legacy_graphics_memory);
        emulator.load_state(&state).unwrap();
        assert!(emulator.legacy_graphics_memory);
        emulator.vm.memory_mut()[BUFFER_SIZE] = 0xff;
        assert_eq!(emulator.run_frame().unwrap().status, FrameStatus::Presented);
        assert_eq!(&emulator.display().indexed_frame()[..8], &[1; 8]);
    }
}
