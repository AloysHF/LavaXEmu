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
        let mut image = data.to_vec();
        if image.len() == 23_739
            && crc32fast::hash(&image) == 0x12e8_a226
            && image[0x573a] == 0x31
            && image[0x5742] == 0x50
            && image[0x57c7] == 0x39
        {
            // Correct this exact Q Dancer release's contest input and sequence bounds.
            image[0x573a] = 0x34;
            image[0x5742] = 0xcb;
            image[0x57c7] = 0x3a;
        }
        if image.len() == 18_430 && crc32fast::hash(&image) == 0xb914_9cc5 {
            // Initialize this release's name buffer before it reuses local storage.
            let trampoline = image.len() as u32;
            let entry = image[0x0ff7..0x1000].to_vec();
            image.extend_from_slice(&entry);
            image.extend_from_slice(&[
                0x19, 0x07, 0x00, 0x02, 0x28, 0x2b, 0x83, 0x3b, 0x00, 0x10, 0x00,
            ]);
            image[0x0ff7..0x0ffb].copy_from_slice(&[
                0x3b,
                trampoline as u8,
                (trampoline >> 8) as u8,
                (trampoline >> 16) as u8,
            ]);
            image[0x0ffb..0x1000].fill(0);
        }
        if image.len() == 8_201
            && crc32fast::hash(&image) == 0xf533_81e2
            && image[0x185a..0x185e] == [0x3b, 0x99, 0x1a, 0x00]
            && image[0x1aed] == 0x27
        {
            // Restore this release's new-game state and reachable five-stage ending.
            image[0x1aed] = 0x28;
            let trampoline = image.len() as u32;
            for (address, value) in [
                (0x3632u16, 1u16),
                (0x3634, 0),
                (0x3636, 0),
                (0x3638, 200),
                (0x363a, 580),
            ] {
                let [low, high] = address.to_le_bytes();
                let [value_low, value_high] = value.to_le_bytes();
                image.extend_from_slice(&[
                    0x03, low, high, 0x02, 0x00, 0x02, value_low, value_high, 0x35, 0x38,
                ]);
            }
            image.extend_from_slice(&[0x3b, 0x99, 0x1a, 0x00]);
            image[0x185a..0x185e].copy_from_slice(&[
                0x3b,
                trampoline as u8,
                (trampoline >> 8) as u8,
                (trampoline >> 16) as u8,
            ]);
        }
        if image.len() == 32_559
            && crc32fast::hash(&image) == 0x6f8d_adb3
            && image[0x33e5..0x33eb] == [0x04, 0x7a, 0x4f, 0xaf, 0x8e, 0x89]
            && image[0x3098..0x309d] == [0x04, 0x47, 0x4f, 0xb6, 0x38]
        {
            // Give this release's three records real file offsets before saving.
            let trampoline = image.len() as u32;
            let entry = image[0x33e5..0x33eb].to_vec();
            image.extend_from_slice(&entry);
            image.extend_from_slice(&[0x03, 0x46, 0x4f, 0x01, 0x00, 0x0d]);
            image.extend_from_slice(b"/LavaData/zqtop.dat\0");
            image.extend_from_slice(&[
                0x0d, b'a', b'+', 0x00, 0xae, 0x35, 0x38, 0x04, 0x46, 0x4f, 0x01, 0x00, 0x01, 0x02,
                0xb2, 0x38,
            ]);
            let pad_loop = image.len() as u32;
            image.extend_from_slice(&[0x04, 0x46, 0x4f, 0xb3, 0x4f, 0x03, 0x00, 0x38, 0x39]);
            let done_branch = image.len();
            image.extend_from_slice(&[0, 0, 0]);
            image.extend_from_slice(&[0x01, 0x00, 0x04, 0x46, 0x4f, 0xb7, 0x38, 0x3b]);
            image.extend_from_slice(&pad_loop.to_le_bytes()[..3]);
            let done = image.len() as u32;
            image[done_branch..done_branch + 3].copy_from_slice(&done.to_le_bytes()[..3]);
            image.extend_from_slice(&[0x04, 0x46, 0x4f, 0xaf, 0x3b, 0xeb, 0x33, 0x00]);
            image[0x33e5] = 0x3b;
            image[0x33e6..0x33e9].copy_from_slice(&trampoline.to_le_bytes()[..3]);
            image[0x33e9..0x33eb].fill(0);
            // Rewind after reading the shooting record before replacing slot zero.
            image[0x3098..0x309d].copy_from_slice(&[0x04, 0x46, 0x4f, 0xb5, 0x00]);
        }
        if image.len() == 12_883
            && crc32fast::hash(&image) == 0xf2bc_baa6
            && image[0x2dfb] == 0x01
            && image[0x2e11] == 0x01
        {
            // Present this release's ending pictures before waiting for input.
            image[0x2dfb] = 0x41;
            image[0x2e11] = 0x41;
        }
        if image.len() == 80_098
            && crc32fast::hash(&image) == 0x1d36_19b1
            && image[0x11116..0x11119] == [0x08, 0x97, 0x32]
            && image[0x113b3..0x113b8] == [0x03, 0xa1, 0x37, 0x02, 0x00]
            && image[0x112a8..0x112ad] == [0x01, 0x01, 0x05, 0xad, 0x37]
            && image[0x1133e..0x11343] == [0x01, 0x01, 0x05, 0xa1, 0x37]
        {
            // Read the second player's own HP for its roster and replacement check.
            for site in [0x10ff5, 0x1101b, 0x11116] {
                image[site + 1..site + 3].copy_from_slice(&0x32d9u16.to_le_bytes());
            }
            // Count defeated party members instead of changing the active slot.
            image[0x113b3..0x113b8].copy_from_slice(&[0x03, 0x05, 0x00, 0x82, 0x00]);
            // Preserve the move catalog before party setup overlaps its first entry.
            let catalog_reads = [
                0xf4c3, 0xf4fa, 0xf531, 0xf568, 0xf59f, 0xf64b, 0xf682, 0xf6b9, 0xf6f0, 0xf727,
            ];
            for site in catalog_reads {
                image[site + 1..site + 3].copy_from_slice(&0x6000u16.to_le_bytes());
            }
            let trampoline = image.len() as u32;
            let original = image[0x1362c..0x13631].to_vec();
            image.extend_from_slice(&[0x02, 0, 0x60, 0x02, 0x1b, 0x33, 0x02, 0xd0, 0x02, 0xad]);
            image.extend_from_slice(&original);
            image.extend_from_slice(&[0x3b, 0x31, 0x36, 0x01]);
            image[0x1362c] = 0x3b;
            image[0x1362d..0x13630].copy_from_slice(&trampoline.to_le_bytes()[..3]);
            image[0x13630] = 0;
            // Keep defeated party members at zero HP in both replacement menus.
            for (entry, active, party) in
                [(0x112a8, 0x37adu16, 0x32d9u16), (0x1133e, 0x37a1, 0x3297)]
            {
                let trampoline = image.len() as u32;
                let original = image[entry..entry + 5].to_vec();
                let [active_low, active_high] = active.to_le_bytes();
                let [party_low, party_high] = party.to_le_bytes();
                image.extend_from_slice(&[
                    0x05,
                    active_low,
                    active_high,
                    0x47,
                    0x16,
                    0,
                    0x45,
                    0x10,
                    0,
                    0x0b,
                    party_low,
                    party_high,
                    0x01,
                    0,
                    0x35,
                    0x38,
                ]);
                image.extend_from_slice(&original);
                image.push(0x3b);
                image.extend_from_slice(&((entry + 5) as u32).to_le_bytes()[..3]);
                image[entry] = 0x3b;
                image[entry + 1..entry + 4].copy_from_slice(&trampoline.to_le_bytes()[..3]);
                image[entry + 4] = 0;
            }
        }
        if image.len() == 14_140
            && crc32fast::hash(&image) == 0x0044_87e2
            && image[0x1940..0x1946] == [0x0f, 0x2e, 0, 0x4f, 0x0a, 0]
            && image[0x2e2d..0x2e31] == [0x3d, 0xac, 1, 0]
            && image[0x2ee6..0x2eea] == [0x3d, 0x3b, 0x26, 0]
        {
            // Stop this release's enemy targeting after a party wipe.
            let original = image[0x1940..0x1946].to_vec();
            let mut defeat = image[0x1b09..0x1b2d].to_vec();
            defeat[0x11..0x14].copy_from_slice(&[0x02, 0x4b, 0x42]);
            let result_offset = defeat.len() - 2;
            defeat[result_offset] = 2;
            let battle_guard = image.len() as u32;
            for (index, address) in [0x425fu16, 0x428f, 0x42bf, 0x42ef, 0x431f]
                .into_iter()
                .enumerate()
            {
                image.push(0x05);
                image.extend_from_slice(&address.to_le_bytes());
                if index != 0 {
                    image.push(0x21);
                }
            }
            image.extend_from_slice(&[0x51, 0, 0, 0x38, 0x39, 0, 0, 0]);
            let healthy_jump = image.len() - 3;
            image.extend_from_slice(&[0x03, 0x10, 0x20, 0x02, 0, 0x01, 5, 0x35, 0x38]);
            image.extend_from_slice(&defeat);
            let healthy = image.len() as u32;
            image[healthy_jump..healthy_jump + 3].copy_from_slice(&healthy.to_le_bytes()[..3]);
            image.extend_from_slice(&original);
            image.extend_from_slice(&[0x3b, 0x46, 0x19, 0]);
            image[0x1940] = 0x3b;
            image[0x1941..0x1944].copy_from_slice(&battle_guard.to_le_bytes()[..3]);
            image[0x1944..0x1946].fill(0);

            // Return to the existing title after defeat or an Escape request.
            let field_guard = image.len() as u32;
            let next_input = field_guard + 21;
            image.extend_from_slice(&[0x05, 0x10, 0x20, 0x4c, 5, 0, 0x38, 0x39]);
            image.extend_from_slice(&next_input.to_le_bytes()[..3]);
            image.extend_from_slice(&[0x03, 0x10, 0x20, 0x02, 0, 0x01, 0, 0x35, 0x38]);
            image.push(0x3f);
            image.extend_from_slice(&[
                0x3d, 0xac, 1, 0, 0x05, 0x10, 0x20, 0x4c, 4, 0, 0x38, 0x39, 0x31, 0x2e, 0, 0x03,
                0x10, 0x20, 0x02, 0, 0x01, 0, 0x35, 0x38, 0x3f,
            ]);
            image[0x2e2d] = 0x3b;
            image[0x2e2e..0x2e31].copy_from_slice(&field_guard.to_le_bytes()[..3]);

            // Remove menu overlays before the field waits for another input.
            let redraw = image.len() as u32;
            image.extend_from_slice(&[
                0x05, 0x10, 0x20, 0x4c, 2, 0, 0x05, 0x10, 0x20, 0x4c, 4, 0, 0x28, 0x38, 0x39,
            ]);
            let redraw_jump = image.len();
            image.extend_from_slice(&[0, 0, 0]);
            image.extend_from_slice(&[0x3d, 0xad, 0x24, 0, 0x01, 0x11, 0x3d, 0xb2, 0x22, 0, 0x89]);
            let resume = image.len() as u32;
            image[redraw_jump..redraw_jump + 3].copy_from_slice(&resume.to_le_bytes()[..3]);
            image.extend_from_slice(&[0x3d, 0x3b, 0x26, 0, 0x3b, 0xea, 0x2e, 0]);
            image[0x2ee6] = 0x3b;
            image[0x2ee7..0x2eea].copy_from_slice(&redraw.to_le_bytes()[..3]);
        }
        if image.len() == 33_999
            && crc32fast::hash(&image) == 0xdf90_d7c3
            && image[0x1e71..0x1e75] == [0x3d, 0x14, 0x17, 0]
        {
            // Reserve both native save slots before this release seeks to either one.
            let trampoline = image.len() as u32;
            image.extend_from_slice(&[0x0e, 9, 0, 0x01, 0, 0x01, 2, 0xb2, 0x38]);
            let pad_loop = image.len() as u32;
            image.extend_from_slice(&[0x0e, 9, 0, 0xb3, 0x4f, 0xa0, 0x0f, 0x38, 0x39]);
            let done_branch = image.len();
            image.extend_from_slice(&[0, 0, 0]);
            image.extend_from_slice(&[0x01, 0, 0x0e, 9, 0, 0xb7, 0x38, 0x3b]);
            image.extend_from_slice(&pad_loop.to_le_bytes()[..3]);
            let done = image.len() as u32;
            image[done_branch..done_branch + 3].copy_from_slice(&done.to_le_bytes()[..3]);
            image.extend_from_slice(&[0x3d, 0x14, 0x17, 0, 0x3b, 0x75, 0x1e, 0]);
            image[0x1e71] = 0x3b;
            image[0x1e72..0x1e75].copy_from_slice(&trampoline.to_le_bytes()[..3]);
        }
        if image.len() == 9_748
            && crc32fast::hash(&image) == 0x420e_640b
            && image[0x2441..0x2445] == [0x3d, 0xa4, 0x1a, 0]
            && image[0x1fc0..0x1fc4] == [1, 0, 1, 0]
        {
            // End the hand before another player draws from an empty wall.
            let draw_guard = image.len() as u32;
            image.extend_from_slice(&[0x04, 0x45, 0x21, 0x38, 0x3a]);
            let draw_branch = image.len();
            image.extend_from_slice(&[0, 0, 0, 0x38, 0x3b, 0xdd, 0x25, 0]);
            let draw = image.len() as u32;
            image[draw_branch..draw_branch + 3].copy_from_slice(&draw.to_le_bytes()[..3]);
            image.extend_from_slice(&[0x3d, 0xa4, 0x1a, 0, 0x3b, 0x45, 0x24, 0]);
            image[0x2441] = 0x3b;
            image[0x2442..0x2445].copy_from_slice(&draw_guard.to_le_bytes()[..3]);

            // A kong requires a replacement tile, so reject it before changing the hand.
            let kong_guard = image.len() as u32;
            image.extend_from_slice(&[0x04, 0x45, 0x21, 0x38, 0x3a]);
            let kong_branch = image.len();
            image.extend_from_slice(&[0, 0, 0, 1, 0, 0x3f]);
            let kong = image.len() as u32;
            image[kong_branch..kong_branch + 3].copy_from_slice(&kong.to_le_bytes()[..3]);
            image.extend_from_slice(&[1, 0, 1, 0, 0x3b, 0xc4, 0x1f, 0]);
            image[0x1fc0] = 0x3b;
            image[0x1fc1..0x1fc4].copy_from_slice(&kong_guard.to_le_bytes()[..3]);
        }
        if image.len() == 11_981
            && crc32fast::hash(&image) == 0xe940_01a5
            && image[0x2abb..0x2ac0] == [0x22, 0x4c, 0, 0, 0x38]
        {
            // Input waits can resume after the exact chest deadline has passed.
            image[0x2abc] = 0x51;
        }
        if image.len() == 4_952
            && crc32fast::hash(&image) == 0xaaed_b282
            && [0x12f7, 0x130e, 0x1322, 0x1338, 0x133c]
                .into_iter()
                .all(|pc| image[pc..pc + 4] == [0x3b, 0xbd, 0x12, 0])
        {
            // Pace each unthrottled battle turn before polling the next input.
            let paced_turn = image.len() as u32;
            image.extend_from_slice(&[1, 80, 0x87, 0x3b, 0xbd, 0x12, 0]);
            for pc in [0x12f7, 0x130e, 0x1322, 0x1338, 0x133c] {
                image[pc + 1..pc + 4].copy_from_slice(&paced_turn.to_le_bytes()[..3]);
            }
        }
        Ok(Self { header, image })
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
    fn leaves_unrecognized_images_and_caller_data_unchanged() {
        let mut original = image(0, 0, 0);
        original.resize(23_739, 0);
        for (address, byte) in [(0x573a, 0x31), (0x5742, 0x50), (0x57c7, 0x39)] {
            original[address] = byte;
        }
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_name_entry_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(18_430, 0);
        original[0x0ff7..0x1000]
            .copy_from_slice(&[0x03, 0x15, 0x00, 0x81, 0x00, 0x01, 0x01, 0x35, 0x38]);
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_five_stage_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(8_201, 0);
        original[0x185a..0x185e].copy_from_slice(&[0x3b, 0x99, 0x1a, 0x00]);
        original[0x1aed] = 0x27;
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_sports_record_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(32_559, 0);
        original[0x33e5..0x33eb].copy_from_slice(&[0x04, 0x7a, 0x4f, 0xaf, 0x8e, 0x89]);
        original[0x3098..0x309d].copy_from_slice(&[0x04, 0x47, 0x4f, 0xb6, 0x38]);
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_ski_ending_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(12_883, 0);
        original[0x2dfb] = 0x01;
        original[0x2e11] = 0x01;
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_party_selection_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(80_098, 0);
        original[0x11116..0x11119].copy_from_slice(&[0x08, 0x97, 0x32]);
        original[0x113b3..0x113b8].copy_from_slice(&[0x03, 0xa1, 0x37, 0x02, 0x00]);
        original[0x112a8..0x112ad].copy_from_slice(&[0x01, 0x01, 0x05, 0xad, 0x37]);
        original[0x1133e..0x11343].copy_from_slice(&[0x01, 0x01, 0x05, 0xa1, 0x37]);
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_party_defeat_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(14_140, 0);
        original[0x1940..0x1946].copy_from_slice(&[0x0f, 0x2e, 0, 0x4f, 0x0a, 0]);
        original[0x2e2d..0x2e31].copy_from_slice(&[0x3d, 0xac, 1, 0]);
        original[0x2ee6..0x2eea].copy_from_slice(&[0x3d, 0x3b, 0x26, 0]);
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_two_slot_save_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(33_999, 0);
        original[0x1e71..0x1e75].copy_from_slice(&[0x3d, 0x14, 0x17, 0]);
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_four_player_tile_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(9_748, 0);
        original[0x2441..0x2445].copy_from_slice(&[0x3d, 0xa4, 0x1a, 0]);
        original[0x1fc0..0x1fc4].copy_from_slice(&[1, 0, 1, 0]);
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_unthrottled_battle_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(4_952, 0);
        for pc in [0x12f7, 0x130e, 0x1322, 0x1338, 0x133c] {
            original[pc..pc + 4].copy_from_slice(&[0x3b, 0xbd, 0x12, 0]);
        }
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
    }

    #[test]
    fn leaves_similar_chest_timer_programs_unmodified() {
        let mut original = image(0, 0, 0);
        original.resize(11_981, 0);
        original[0x2abb..0x2ac0].copy_from_slice(&[0x22, 0x4c, 0, 0, 0x38]);
        let saved = original.clone();
        let program = Program::load(&original).unwrap();
        assert_eq!(original, saved);
        assert_eq!(program.image(), original);
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
