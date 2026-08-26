use std::{cmp::Ordering, ops::Range};

use encoding_rs::GBK;

use crate::{
    AddressWidth, BufferTarget, DrawOperation, Emulator, Error, GraphicsMode, Result, Transform,
    emulator::HostAction,
};

const LAVA_TRUE: i32 = -1;

impl Emulator {
    pub(crate) fn dispatch_system_call(&mut self, call: u8) -> Result<HostAction> {
        match call {
            0 => self.system_putchar(),
            1 => self.system_getchar(),
            2 => self.system_printf(),
            3 => self.system_strcpy(),
            4 => self.system_strlen(),
            5 => self.system_setscreen(),
            6 => self.system_update_lcd(),
            7 => self.system_delay(),
            8 => self.system_write_block(),
            9 => {
                self.display.present();
                Ok(HostAction::Continue)
            }
            10 => self.system_textout(),
            11 => self.system_rectangle(true),
            12 => self.system_rectangle(false),
            13 => self.system_exit(),
            14 => {
                self.display.clear(BufferTarget::Back);
                Ok(HostAction::Continue)
            }
            15 => self.system_abs(),
            16 => self.system_rand(),
            17 => self.system_srand(),
            18 => self.system_locate(),
            19 => self.system_inkey(),
            20 => self.system_point(),
            21 => self.system_getpoint(),
            22 => self.system_line(),
            23 => self.system_box(),
            24 => self.system_circle(),
            25 => self.system_ellipse(),
            26 => Ok(HostAction::Continue),
            27..=37 => self.system_ctype(call),
            38 => self.system_strcat(),
            39 => self.system_strchr(),
            40 => self.system_strcmp(),
            41 => self.system_strstr(),
            42 => self.system_case(false),
            43 => self.system_case(true),
            44 => self.system_memset(),
            45 => self.system_memcpy(false),
            46 => self.system_fopen(),
            47 => self.system_fclose(),
            48 => self.system_fread(),
            49 => self.system_fwrite(),
            50 => self.system_fseek(),
            51 => self.system_ftell(),
            52 => self.system_feof(),
            53 => self.system_rewind(),
            54 => self.system_getc(),
            55 => self.system_putc(),
            56 => self.system_sprintf(),
            57 => self.system_makedir(),
            58 => self.system_delete(),
            59 => {
                let value = (((self.frame_index * 1_000 / 60) % 1_000) * 256 / 1_000) as i32;
                self.vm.push_value(value)?;
                Ok(HostAction::Continue)
            }
            60 => self.system_checkkey(),
            61 => self.system_memcpy(true),
            62 => self.system_crc16(),
            63 => self.system_xor(),
            64 => self.system_chdir(),
            65 => self.system_filelist(),
            66 => self.system_gettime(),
            67 => self.system_settime(),
            68 => self.system_getword(),
            69 => self.system_transform(),
            70 => self.system_releasekey(),
            71 => self.system_getblock(),
            72 => self.system_integer_trig(true),
            73 => self.system_integer_trig(false),
            74 => {
                self.pop_values(3)?;
                Ok(HostAction::Continue)
            }
            75 => self.system_set_graphics_mode(),
            76 => {
                let color = self.vm.pop_value()? as u8;
                self.display.set_background(color);
                Ok(HostAction::Continue)
            }
            77 => {
                let color = self.vm.pop_value()? as u8;
                self.display.set_foreground(color);
                Ok(HostAction::Continue)
            }
            78 => {
                self.pop_values(2)?;
                Ok(HostAction::Continue)
            }
            79 => {
                let amount = self.vm.pop_value()? as u8;
                self.display.fade(amount);
                Ok(HostAction::Continue)
            }
            80 => self.system_exec(),
            81 => self.system_findfile(),
            82 => self.system_getfilenum(),
            83 => self.system_extended(),
            84 => self.system_math(),
            85 => self.system_setpalette(),
            86 => self.system_getcmdline(),
            _ => unreachable!("the VM validates the system call range"),
        }
    }

    fn system_putchar(&mut self) -> Result<HostAction> {
        let value = self.vm.pop_value()? as u8;
        self.console.write(value);
        self.console.render(&mut self.display);
        Ok(HostAction::Continue)
    }

    fn system_getchar(&mut self) -> Result<HostAction> {
        if let Some(value) = self.pending_input_value() {
            self.vm.push_value(value)?;
            Ok(HostAction::Continue)
        } else {
            self.waiting_for_key = true;
            Ok(HostAction::WaitForInput)
        }
    }

    fn system_printf(&mut self) -> Result<HostAction> {
        let arguments = self.pop_variadic_arguments()?;
        if arguments.is_empty() {
            return Ok(HostAction::Continue);
        }
        let output = self.format_arguments(&arguments)?;
        for byte in output {
            self.console.write(byte);
        }
        self.console.render(&mut self.display);
        Ok(HostAction::Continue)
    }

    fn system_strcpy(&mut self) -> Result<HostAction> {
        let source = self.pop_guest_address()?;
        let destination = self.pop_guest_address()?;
        let mut data = self.read_c_bytes(source)?;
        data.push(0);
        self.write_memory(destination, &data)?;
        Ok(HostAction::Continue)
    }

    fn system_strlen(&mut self) -> Result<HostAction> {
        let address = self.pop_guest_address()?;
        let length = self.read_c_bytes(address)?.len();
        self.vm.push_value(length as i32)?;
        Ok(HostAction::Continue)
    }

    fn system_setscreen(&mut self) -> Result<HostAction> {
        let small = self.vm.pop_value()? & 0xff != 0;
        self.console
            .set_mode(self.display.width(), self.display.height(), small);
        Ok(HostAction::Continue)
    }

    fn system_update_lcd(&mut self) -> Result<HostAction> {
        self.vm.pop_value()?;
        self.console.render(&mut self.display);
        Ok(HostAction::Present)
    }

    fn system_delay(&mut self) -> Result<HostAction> {
        let milliseconds = (self.vm.pop_value()? & 0x7fff) as u32;
        self.delay_remaining_ticks = milliseconds * 256 / 1_000;
        if self.delay_remaining_ticks == 0 {
            Ok(HostAction::Continue)
        } else {
            Ok(HostAction::Delay)
        }
    }

    fn system_write_block(&mut self) -> Result<HostAction> {
        let source = self.pop_guest_address()?;
        let mode = self.vm.pop_value()? as u8;
        let height = self.vm.pop_value()? as u16;
        let width = self.vm.pop_value()? as u16;
        let y = coordinate(self.vm.pop_value()?);
        let x = coordinate(self.vm.pop_value()?);
        let length = packed_length(self.display.graphics_mode(), width, height);
        let range = self.memory_range(source, length)?;
        let data = self.vm.memory()[range].to_vec();
        self.display.blit(
            target_from_flag(mode, true),
            x,
            y,
            width,
            height,
            &data,
            mode & 0x0f,
            mode & 0x20 != 0,
        );
        Ok(HostAction::Continue)
    }

    fn system_textout(&mut self) -> Result<HostAction> {
        let mode = self.vm.pop_value()? as u8;
        let address = self.pop_guest_address()?;
        let y = coordinate(self.vm.pop_value()?);
        let x = coordinate(self.vm.pop_value()?);
        let text = self.read_c_bytes(address)?;
        self.display.draw_text(
            target_from_flag(mode, true),
            x,
            y,
            &text,
            mode & 0x80 != 0,
            if mode & 0x0f == 2 {
                DrawOperation::Invert
            } else {
                DrawOperation::Set
            },
        );
        Ok(HostAction::Continue)
    }

    fn system_rectangle(&mut self, filled: bool) -> Result<HostAction> {
        let mode = self.vm.pop_value()?;
        let y1 = block_clamp(coordinate(self.vm.pop_value()?), i32::from(self.display.height()));
        let x1 = block_clamp(coordinate(self.vm.pop_value()?), i32::from(self.display.width()));
        let y0 = block_clamp(coordinate(self.vm.pop_value()?), i32::from(self.display.height()));
        let x0 = block_clamp(coordinate(self.vm.pop_value()?), i32::from(self.display.width()));
        self.display.draw_rectangle(
            target_from_flag(mode as u8, true),
            x0,
            y0,
            x1,
            y1,
            filled,
            DrawOperation::from_lava(mode),
        );
        Ok(HostAction::Continue)
    }

    fn system_exit(&mut self) -> Result<HostAction> {
        let code = self.vm.pop_value()?;
        self.vm.halt(code);
        Ok(HostAction::Halt(code))
    }

    fn system_abs(&mut self) -> Result<HostAction> {
        let value = self.vm.pop_value()?.wrapping_abs();
        self.vm.push_value(value)?;
        Ok(HostAction::Continue)
    }

    fn system_rand(&mut self) -> Result<HostAction> {
        self.random_seed = self.random_seed.wrapping_mul(0x015a_4e35).wrapping_add(1);
        let value = (self.random_seed >> 16) & 0x7fff;
        self.vm.push_value(value)?;
        Ok(HostAction::Continue)
    }

    fn system_srand(&mut self) -> Result<HostAction> {
        self.random_seed = self.vm.pop_value()?;
        Ok(HostAction::Continue)
    }

    fn system_locate(&mut self) -> Result<HostAction> {
        let x = self.vm.pop_value()? as usize;
        let y = self.vm.pop_value()? as usize;
        self.console.locate(x, y);
        Ok(HostAction::Continue)
    }

    fn system_inkey(&mut self) -> Result<HostAction> {
        let key = self.input.pop_key().map_or(0, i32::from);
        self.vm.push_value(key)?;
        Ok(HostAction::Continue)
    }

    fn system_point(&mut self) -> Result<HostAction> {
        let mode = self.vm.pop_value()?;
        let y = coordinate(self.vm.pop_value()?);
        let x = coordinate(self.vm.pop_value()?);
        self.display.draw_pixel(
            target_from_flag(mode as u8, false),
            x,
            y,
            DrawOperation::from_lava(mode),
        );
        Ok(HostAction::Continue)
    }

    fn system_getpoint(&mut self) -> Result<HostAction> {
        let y = coordinate(self.vm.pop_value()?);
        let x = coordinate(self.vm.pop_value()?);
        let value = self
            .display
            .get_pixel(BufferTarget::Front, x, y)
            .map_or(0, i32::from);
        self.vm.push_value(value)?;
        Ok(HostAction::Continue)
    }

    fn system_line(&mut self) -> Result<HostAction> {
        let mode = self.vm.pop_value()?;
        let y1 = coordinate(self.vm.pop_value()?);
        let x1 = coordinate(self.vm.pop_value()?);
        let y0 = coordinate(self.vm.pop_value()?);
        let x0 = coordinate(self.vm.pop_value()?);
        self.display.draw_line(
            target_from_flag(mode as u8, false),
            x0,
            y0,
            x1,
            y1,
            DrawOperation::from_lava(mode),
        );
        Ok(HostAction::Continue)
    }

    fn system_box(&mut self) -> Result<HostAction> {
        let mode = self.vm.pop_value()?;
        let filled = self.vm.pop_value()? != 0;
        let y1 = block_clamp(coordinate(self.vm.pop_value()?), i32::from(self.display.height()));
        let x1 = block_clamp(coordinate(self.vm.pop_value()?), i32::from(self.display.width()));
        let y0 = block_clamp(coordinate(self.vm.pop_value()?), i32::from(self.display.height()));
        let x0 = block_clamp(coordinate(self.vm.pop_value()?), i32::from(self.display.width()));
        self.display.draw_rectangle(
            BufferTarget::Front,
            x0,
            y0,
            x1,
            y1,
            filled,
            DrawOperation::from_lava(mode),
        );
        Ok(HostAction::Continue)
    }

    fn system_circle(&mut self) -> Result<HostAction> {
        let mode = self.vm.pop_value()?;
        let filled = self.vm.pop_value()? != 0;
        let radius = self.vm.pop_value()? as u16;
        let y = coordinate(self.vm.pop_value()?);
        let x = coordinate(self.vm.pop_value()?);
        self.display.draw_ellipse(
            target_from_flag(mode as u8, false),
            x,
            y,
            radius,
            radius,
            filled,
            DrawOperation::from_lava(mode),
        );
        Ok(HostAction::Continue)
    }

    fn system_ellipse(&mut self) -> Result<HostAction> {
        let mode = self.vm.pop_value()?;
        let filled = self.vm.pop_value()? != 0;
        let radius_y = self.vm.pop_value()? as u16;
        let radius_x = self.vm.pop_value()? as u16;
        let y = coordinate(self.vm.pop_value()?);
        let x = coordinate(self.vm.pop_value()?);
        self.display.draw_ellipse(
            target_from_flag(mode as u8, false),
            x,
            y,
            radius_x,
            radius_y,
            filled,
            DrawOperation::from_lava(mode),
        );
        Ok(HostAction::Continue)
    }

    fn system_ctype(&mut self, call: u8) -> Result<HostAction> {
        let value = self.vm.pop_value()? as u8;
        let matches = match call {
            27 => value.is_ascii_alphanumeric(),
            28 => value.is_ascii_alphabetic(),
            29 => value.is_ascii_control(),
            30 => value.is_ascii_digit(),
            31 => value.is_ascii_graphic(),
            32 => value.is_ascii_lowercase(),
            33 => value.is_ascii_graphic() || value == b' ',
            34 => value.is_ascii_punctuation(),
            35 => value.is_ascii_whitespace(),
            36 => value.is_ascii_uppercase(),
            37 => value.is_ascii_hexdigit(),
            _ => unreachable!(),
        };
        self.vm.push_value(bool_value(matches))?;
        Ok(HostAction::Continue)
    }

    fn system_strcat(&mut self) -> Result<HostAction> {
        let source = self.pop_guest_address()?;
        let destination = self.pop_guest_address()?;
        let source = self.read_c_bytes(source)?;
        let mut destination_data = self.read_c_bytes(destination)?;
        destination_data.extend(source);
        destination_data.push(0);
        self.write_memory(destination, &destination_data)?;
        Ok(HostAction::Continue)
    }

    fn system_strchr(&mut self) -> Result<HostAction> {
        let needle = self.vm.pop_value()? as u8;
        let address = self.pop_guest_address()?;
        let data = self.read_c_bytes(address)?;
        let offset = if needle == 0 {
            Some(data.len())
        } else {
            data.iter().position(|&byte| byte == needle)
        };
        self.vm
            .push_value(offset.map_or(0, |offset| address as i32 + offset as i32))?;
        Ok(HostAction::Continue)
    }

    fn system_strcmp(&mut self) -> Result<HostAction> {
        let right = self.pop_guest_address()?;
        let left = self.pop_guest_address()?;
        let ordering = self.read_c_bytes(left)?.cmp(&self.read_c_bytes(right)?);
        let value = match ordering {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };
        self.vm.push_value(value)?;
        Ok(HostAction::Continue)
    }

    fn system_strstr(&mut self) -> Result<HostAction> {
        let needle = self.pop_guest_address()?;
        let haystack_address = self.pop_guest_address()?;
        let needle = self.read_c_bytes(needle)?;
        let haystack = self.read_c_bytes(haystack_address)?;
        let offset = if needle.is_empty() {
            Some(0)
        } else {
            haystack
                .windows(needle.len())
                .position(|window| window == needle)
        };
        self.vm
            .push_value(offset.map_or(0, |offset| haystack_address as i32 + offset as i32))?;
        Ok(HostAction::Continue)
    }

    fn system_case(&mut self, upper: bool) -> Result<HostAction> {
        let value = self.vm.pop_value()? as u8;
        let converted = if upper {
            value.to_ascii_uppercase()
        } else {
            value.to_ascii_lowercase()
        };
        self.vm.push_value(i32::from(converted))?;
        Ok(HostAction::Continue)
    }

    fn system_memset(&mut self) -> Result<HostAction> {
        let length = self.pop_guest_length()?;
        let value = self.vm.pop_value()? as u8;
        let destination = self.pop_guest_address()?;
        let range = self.memory_range(destination, length)?;
        self.vm.memory_mut()[range].fill(value);
        Ok(HostAction::Continue)
    }

    fn system_memcpy(&mut self, overlapping: bool) -> Result<HostAction> {
        let length = self.pop_guest_length()?;
        let source = self.pop_guest_address()?;
        let destination = self.pop_guest_address()?;
        let source_range = self.memory_range(source, length)?;
        let destination_range = self.memory_range(destination, length)?;
        if overlapping {
            self.vm
                .memory_mut()
                .copy_within(source_range, destination_range.start);
        } else {
            let data = self.vm.memory()[source_range].to_vec();
            self.vm.memory_mut()[destination_range].copy_from_slice(&data);
        }
        Ok(HostAction::Continue)
    }

    fn system_fopen(&mut self) -> Result<HostAction> {
        let mode_address = self.pop_guest_address()?;
        let path_address = self.pop_guest_address()?;
        let mode = String::from_utf8_lossy(&self.read_c_bytes(mode_address)?).into_owned();
        let path = self.decode_guest_string(path_address)?;
        let handle = self.files.open(&path, &mode).map_or(0, i32::from);
        self.vm.push_value(handle)?;
        Ok(HostAction::Continue)
    }

    fn system_fclose(&mut self) -> Result<HostAction> {
        let handle = self.vm.pop_value()? as u8;
        self.files.close(handle);
        Ok(HostAction::Continue)
    }

    fn system_fread(&mut self) -> Result<HostAction> {
        let handle = self.vm.pop_value()? as u8;
        let length = self.pop_guest_length()?;
        self.vm.pop_value()?;
        let destination = self.pop_guest_address()?;
        let data = self.files.read(handle, length).unwrap_or_default();
        self.write_memory(destination, &data)?;
        self.vm.push_value(data.len() as i32)?;
        Ok(HostAction::Continue)
    }

    fn system_fwrite(&mut self) -> Result<HostAction> {
        let handle = self.vm.pop_value()? as u8;
        let length = self.pop_guest_length()?;
        self.vm.pop_value()?;
        let source = self.pop_guest_address()?;
        let range = self.memory_range(source, length)?;
        let data = self.vm.memory()[range].to_vec();
        let written = self.files.write(handle, &data).unwrap_or(0);
        self.vm.push_value(written as i32)?;
        Ok(HostAction::Continue)
    }

    fn system_fseek(&mut self) -> Result<HostAction> {
        let origin = self.vm.pop_value()? as u8;
        let offset = self.vm.pop_value()?;
        let handle = self.vm.pop_value()? as u8;
        let position = self
            .files
            .seek(handle, offset, origin)
            .map_or(-1, |value| value as i32);
        self.vm.push_value(position)?;
        Ok(HostAction::Continue)
    }

    fn system_ftell(&mut self) -> Result<HostAction> {
        let handle = self.vm.pop_value()? as u8;
        let position = self.files.tell(handle).map_or(-1, |value| value as i32);
        self.vm.push_value(position)?;
        Ok(HostAction::Continue)
    }

    fn system_feof(&mut self) -> Result<HostAction> {
        let handle = self.vm.pop_value()? as u8;
        let eof = self.files.eof(handle).map_or(-1, bool_value);
        self.vm.push_value(eof)?;
        Ok(HostAction::Continue)
    }

    fn system_rewind(&mut self) -> Result<HostAction> {
        let handle = self.vm.pop_value()? as u8;
        self.files.rewind(handle);
        Ok(HostAction::Continue)
    }

    fn system_getc(&mut self) -> Result<HostAction> {
        let handle = self.vm.pop_value()? as u8;
        let value = self
            .files
            .read(handle, 1)
            .and_then(|data| data.first().copied())
            .map_or(-1, i32::from);
        self.vm.push_value(value)?;
        Ok(HostAction::Continue)
    }

    fn system_putc(&mut self) -> Result<HostAction> {
        let handle = self.vm.pop_value()? as u8;
        let value = self.vm.pop_value()? as u8;
        let result = if self.files.write(handle, &[value]) == Some(1) {
            i32::from(value)
        } else {
            -1
        };
        self.vm.push_value(result)?;
        Ok(HostAction::Continue)
    }

    fn system_sprintf(&mut self) -> Result<HostAction> {
        let arguments = self.pop_variadic_arguments()?;
        if arguments.len() < 2 {
            return Ok(HostAction::Continue);
        }
        let destination = self.guest_address(arguments[0]);
        let output = self.format_arguments(&arguments[1..])?;
        self.write_memory(destination, &output)?;
        self.write_memory(destination + output.len() as u32, &[0])?;
        Ok(HostAction::Continue)
    }

    fn system_makedir(&mut self) -> Result<HostAction> {
        let path_address = self.pop_guest_address()?;
        let path = self.decode_guest_string(path_address)?;
        let result = self.files.create_directory(&path);
        self.vm.push_value(bool_value(result))?;
        Ok(HostAction::Continue)
    }

    fn system_delete(&mut self) -> Result<HostAction> {
        let path_address = self.pop_guest_address()?;
        let path = self.decode_guest_string(path_address)?;
        let result = self.files.delete(&path);
        self.vm.push_value(bool_value(result))?;
        Ok(HostAction::Continue)
    }

    fn system_checkkey(&mut self) -> Result<HostAction> {
        let key = self.vm.pop_value()? as u8;
        let value = if key < 0x80 {
            bool_value(self.input.is_pressed(key))
        } else {
            // LVM scans physical key codes in ascending order and reports the
            // first one that is pressed (mapped back to a LavaX key code).
            (0..256u16)
                .find_map(|vk| {
                    let lava = lava_key_from_vk(vk);
                    (lava != 0 && self.input.is_pressed(lava)).then_some(i32::from(lava))
                })
                .unwrap_or(0)
        };
        self.vm.push_value(value)?;
        Ok(HostAction::Continue)
    }

    fn system_crc16(&mut self) -> Result<HostAction> {
        let length = self.pop_guest_length()?;
        let address = self.pop_guest_address()?;
        let range = self.memory_range(address, length)?;
        let mut crc = 0u16;
        for &byte in &self.vm.memory()[range] {
            crc ^= u16::from(byte) << 8;
            for _ in 0..8 {
                crc = if crc & 0x8000 != 0 {
                    (crc << 1) ^ 0x1021
                } else {
                    crc << 1
                };
            }
        }
        self.vm.push_value(i32::from(crc))?;
        Ok(HostAction::Continue)
    }

    fn system_xor(&mut self) -> Result<HostAction> {
        let key_address = self.pop_guest_address()?;
        let length = self.pop_guest_length()?;
        let destination = self.pop_guest_address()?;
        let key = self.read_c_bytes(key_address)?;
        if key.is_empty() {
            return Ok(HostAction::Continue);
        }
        let range = self.memory_range(destination, length)?;
        for (index, byte) in self.vm.memory_mut()[range].iter_mut().enumerate() {
            *byte ^= key[index % key.len()];
        }
        Ok(HostAction::Continue)
    }

    fn system_chdir(&mut self) -> Result<HostAction> {
        let address = self.pop_guest_address()?;
        let path = self.decode_guest_string(address)?;
        let changed = self.files.change_directory(&path);
        self.vm.push_value(bool_value(changed))?;
        Ok(HostAction::Continue)
    }

    fn system_filelist(&mut self) -> Result<HostAction> {
        self.vm.pop_value()?;
        self.vm.push_value(0)?;
        Ok(HostAction::Continue)
    }

    fn system_gettime(&mut self) -> Result<HostAction> {
        let address = self.pop_guest_address()?;
        let calendar = self.calendar;
        self.write_memory(address, &calendar)?;
        Ok(HostAction::Continue)
    }

    fn system_settime(&mut self) -> Result<HostAction> {
        let address = self.pop_guest_address()?;
        let range = self.memory_range(address, 8)?;
        self.calendar.copy_from_slice(&self.vm.memory()[range]);
        Ok(HostAction::Continue)
    }

    fn system_getword(&mut self) -> Result<HostAction> {
        self.vm.pop_value()?;
        self.system_getchar()
    }

    fn system_transform(&mut self) -> Result<HostAction> {
        let value = self.vm.pop_value()? as u8;
        if let Some(transform) = Transform::from_lava(value) {
            self.display.transform(transform);
        }
        Ok(HostAction::Continue)
    }

    fn system_releasekey(&mut self) -> Result<HostAction> {
        let key = self.vm.pop_value()? as u8;
        if key < 0x80 {
            self.input.release(key);
        } else {
            self.input.release_all();
        }
        Ok(HostAction::Continue)
    }

    fn system_getblock(&mut self) -> Result<HostAction> {
        let destination = self.pop_guest_address()?;
        let mode = self.vm.pop_value()? as u8;
        let height = self.vm.pop_value()? as u16;
        let width = self.vm.pop_value()? as u16;
        let y = coordinate(self.vm.pop_value()?);
        let x = coordinate(self.vm.pop_value()?);
        let data = self
            .display
            .capture(target_from_flag(mode, true), x, y, width, height);
        self.write_memory(destination, &data)?;
        Ok(HostAction::Continue)
    }

    fn system_integer_trig(&mut self, sine: bool) -> Result<HostAction> {
        let degrees = i32::from(self.vm.pop_value()? as u16) % 360;
        let value = if sine {
            sin_lookup(degrees)
        } else {
            let adjusted = if degrees >= 270 {
                degrees - 270
            } else {
                degrees + 90
            };
            sin_lookup(adjusted)
        };
        self.vm.push_value(value)?;
        Ok(HostAction::Continue)
    }

    fn system_set_graphics_mode(&mut self) -> Result<HostAction> {
        let request = self.vm.pop_value()? as u8;
        let previous = self.display.graphics_mode().bits_per_pixel();
        let result = match request {
            0 => previous,
            1 => {
                self.display.set_graphics_mode(GraphicsMode::Mono);
                previous
            }
            4 => {
                self.display.set_graphics_mode(GraphicsMode::Color4);
                previous
            }
            8 => {
                self.display.set_graphics_mode(GraphicsMode::Color8);
                previous
            }
            _ => 0,
        };
        self.vm.push_value(i32::from(result))?;
        Ok(HostAction::Continue)
    }

    fn system_exec(&mut self) -> Result<HostAction> {
        self.pop_values(3)?;
        self.vm.push_value(-1)?;
        Ok(HostAction::Continue)
    }

    fn system_findfile(&mut self) -> Result<HostAction> {
        let destination = self.pop_guest_address()?;
        let count = self.vm.pop_value()?.max(0) as usize;
        let index = self.vm.pop_value()?.max(0) as usize;
        let mut entries = Vec::with_capacity(count);
        let listing = self.files.list(self.files.current_directory());
        for offset in 0..count {
            let name = if index + offset == 0 {
                Some("..")
            } else {
                listing.get(index + offset - 1).map(String::as_str)
            };
            let Some(name) = name else {
                break;
            };
            let mut slot = [0u8; 16];
            let (encoded, _, _) = GBK.encode(name);
            let length = encoded.len().min(15);
            slot[..length].copy_from_slice(&encoded[..length]);
            entries.extend_from_slice(&slot);
        }
        let found = entries.len() / 16;
        self.write_memory(destination, &entries)?;
        self.vm.push_value(found as i32)?;
        Ok(HostAction::Continue)
    }

    fn system_getfilenum(&mut self) -> Result<HostAction> {
        let address = self.pop_guest_address()?;
        let path = self.decode_guest_string(address)?;
        self.vm.push_value(self.files.count(&path))?;
        Ok(HostAction::Continue)
    }

    fn system_extended(&mut self) -> Result<HostAction> {
        let function = self.vm.pop_value()?;
        let result = match function {
            0 | 2..=5 | 7 | 13 | 14 => 0,
            1 | 6 | 8 => {
                self.vm.pop_value()?;
                0
            }
            9..=11 => {
                self.pop_values(2)?;
                0
            }
            12 => {
                self.pop_values(3)?;
                0
            }
            15 => {
                self.system_flm_decode()?;
                0
            }
            20 => {
                self.pop_values(3)?;
                0
            }
            29 => return self.system_findfile_extended(),
            30 => return self.system_getfilenum_extended(),
            31 => (self.frame_index * 1_000 / 60 * 256 / 1_000) as i32,
            32 => {
                self.pop_values(2)?;
                0
            }
            33 => return self.system_file_attributes(),
            _ => 0,
        };
        self.vm.push_value(result)?;
        Ok(HostAction::Continue)
    }

    fn system_math(&mut self) -> Result<HostAction> {
        let function = self.vm.pop_value()?;
        let result = match function {
            0 => 0x100,
            7..=15 | 19 => {
                let input = f32::from_bits(self.vm.pop_value()? as u32);
                let output = match function {
                    7 => input.sin(),
                    8 => input.cos(),
                    9 => input.tan(),
                    10 => input.asin(),
                    11 => input.acos(),
                    12 => input.atan(),
                    13 => input.sqrt(),
                    14 => input.exp(),
                    15 => input.ln(),
                    19 => input.abs(),
                    _ => unreachable!(),
                };
                output.to_bits() as i32
            }
            _ => 0,
        };
        self.vm.push_value(result)?;
        Ok(HostAction::Continue)
    }

    fn system_setpalette(&mut self) -> Result<HostAction> {
        let address = self.pop_guest_address()?;
        let count = self.vm.pop_value()?.clamp(0, 256) as usize;
        let first = self.vm.pop_value()?.clamp(0, 255) as u8;
        let available = 256usize.saturating_sub(usize::from(first));
        let count = count.min(available);
        let range = self.memory_range(address, count * 4)?;
        let data = &self.vm.memory()[range];
        let colors: Vec<[u8; 4]> = data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|color| [color[0], color[1], color[2], color[3]])
            .collect();
        let written = self.display.set_palette_rgba(first, &colors);
        self.vm.push_value(written as i32)?;
        Ok(HostAction::Continue)
    }

    fn system_getcmdline(&mut self) -> Result<HostAction> {
        let destination = self.pop_guest_address()?;
        let command_line = self.command_line.clone();
        self.write_memory(destination, &command_line)?;
        self.write_memory(destination + command_line.len() as u32, &[0])?;
        Ok(HostAction::Continue)
    }

    fn system_findfile_extended(&mut self) -> Result<HostAction> {
        let extension_address = self.pop_guest_address()?;
        let slot_length = self.vm.pop_value()?.max(1) as usize;
        let destination = self.pop_guest_address()?;
        let count = self.vm.pop_value()?.max(0) as usize;
        let index = self.vm.pop_value()?.max(0) as usize;
        let extensions = self.read_c_bytes(extension_address)?;
        let extensions = parse_extensions(&extensions);
        let listing: Vec<String> = self
            .files
            .list(self.files.current_directory())
            .into_iter()
            .filter(|name| extension_matches(name, &extensions))
            .collect();
        let mut output = Vec::new();
        for name in listing.iter().skip(index).take(count) {
            let mut slot = vec![0; slot_length];
            let (encoded, _, _) = GBK.encode(name);
            let length = encoded.len().min(slot_length.saturating_sub(1));
            slot[..length].copy_from_slice(&encoded[..length]);
            output.extend(slot);
        }
        let found = output.len() / slot_length;
        self.write_memory(destination, &output)?;
        self.vm.push_value(found as i32)?;
        Ok(HostAction::Continue)
    }

    fn system_getfilenum_extended(&mut self) -> Result<HostAction> {
        let extension_address = self.pop_guest_address()?;
        let path_address = self.pop_guest_address()?;
        let extensions = parse_extensions(&self.read_c_bytes(extension_address)?);
        let path = self.decode_guest_string(path_address)?;
        let count = self
            .files
            .list(&path)
            .iter()
            .filter(|name| extension_matches(name, &extensions))
            .count() as i32;
        self.vm.push_value(count)?;
        Ok(HostAction::Continue)
    }

    fn system_file_attributes(&mut self) -> Result<HostAction> {
        let output = self.pop_guest_address()?;
        let path_address = self.pop_guest_address()?;
        let path = self.decode_guest_string(path_address)?;
        let Some(file) = self.files.file(&path) else {
            self.vm.push_value(0)?;
            return Ok(HostAction::Continue);
        };
        let mut attributes = vec![0; 29];
        attributes[1..5].copy_from_slice(&(file.len() as u32).to_le_bytes());
        for offset in [5, 13, 21] {
            attributes[offset..offset + 8].copy_from_slice(&self.calendar);
        }
        self.write_memory(output, &attributes)?;
        self.vm.push_value(1)?;
        Ok(HostAction::Continue)
    }

    fn system_flm_decode(&mut self) -> Result<()> {
        let destination_value = self.vm.pop_value()?;
        let source_value = self.vm.pop_value()?;
        let destination = self.guest_address(destination_value);
        let source = self.guest_address(source_value);
        let header = self.read_u16(source)?;
        let kind = header >> 13;
        let length = usize::from(header & 0x1fff);
        if length < 2 {
            return Ok(());
        }
        let range = self.memory_range(source + 2, length - 2)?;
        let encoded = self.vm.memory()[range].to_vec();
        if kind == 0 {
            self.write_memory(destination, &encoded)?;
            return Ok(());
        }
        let mut input = 0;
        let mut output = destination;
        while input < encoded.len() {
            let command = encoded[input];
            input += 1;
            let run = usize::from(command & 0x3f).max(64 * usize::from(command & 0x3f == 0));
            let class = command >> 6;
            let repeated = if class == 3 {
                let value = encoded.get(input).copied().unwrap_or(0);
                input = input.saturating_add(1);
                Some(value)
            } else {
                None
            };
            for _ in 0..run {
                let delta = match class {
                    0 => 0,
                    1 => 0xff,
                    2 => {
                        let value = encoded.get(input).copied().unwrap_or(0);
                        input += 1;
                        value
                    }
                    _ => repeated.unwrap_or(0),
                };
                let value = if kind == 2 {
                    self.read_u8(output)?.wrapping_add(delta)
                } else {
                    delta
                };
                self.write_memory(output, &[value])?;
                output += 1;
            }
        }
        Ok(())
    }

    fn pop_values(&mut self, count: usize) -> Result<Vec<i32>> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.vm.pop_value()?);
        }
        values.reverse();
        Ok(values)
    }

    fn pop_variadic_arguments(&mut self) -> Result<Vec<i32>> {
        let count = self.vm.pop_value()?.max(0) as usize;
        self.pop_values(count)
    }

    fn format_arguments(&self, arguments: &[i32]) -> Result<Vec<u8>> {
        let Some((&format_address, values)) = arguments.split_first() else {
            return Ok(Vec::new());
        };
        let format = self.read_c_bytes(self.guest_address(format_address))?;
        let mut output = Vec::new();
        let mut value_index = 0;
        let mut index = 0;
        while index < format.len() {
            if format[index] != b'%' {
                output.push(format[index]);
                index += 1;
                continue;
            }
            index += 1;
            if format.get(index) == Some(&b'%') {
                output.push(b'%');
                index += 1;
                continue;
            }
            let left_align = format.get(index) == Some(&b'-');
            if left_align {
                index += 1;
            }
            let zero_pad = format.get(index) == Some(&b'0');
            if zero_pad {
                index += 1;
            }
            let mut width = 0usize;
            while let Some(digit) = format.get(index).filter(|digit| digit.is_ascii_digit()) {
                width = width.saturating_mul(10) + usize::from(*digit - b'0');
                index += 1;
            }
            let specifier = format.get(index).copied().unwrap_or(0);
            index += usize::from(specifier != 0);
            let value = values.get(value_index).copied().unwrap_or(0);
            if specifier != b'%' {
                value_index += 1;
            }
            let mut rendered = match specifier {
                b'd' => value.to_string().into_bytes(),
                b'f' => format_float(value),
                b'c' => vec![value as u8],
                b's' => self.read_c_bytes(self.guest_address(value))?,
                0 => break,
                other => vec![other],
            };
            if width > rendered.len() {
                let padding = vec![if zero_pad { b'0' } else { b' ' }; width - rendered.len()];
                if left_align {
                    rendered.extend(padding);
                } else {
                    let mut padded = padding;
                    padded.extend(rendered);
                    rendered = padded;
                }
            }
            output.extend(rendered);
        }
        Ok(output)
    }

    fn pop_guest_address(&mut self) -> Result<u32> {
        let value = self.vm.pop_value()?;
        Ok(self.guest_address(value))
    }

    fn pop_guest_length(&mut self) -> Result<usize> {
        let value = self.vm.pop_value()?;
        Ok(self.guest_length(value))
    }

    fn guest_address(&self, value: i32) -> u32 {
        match self.vm.program().header().address_width {
            AddressWidth::Bits16 => value as u32 & 0xffff,
            AddressWidth::Bits24 | AddressWidth::Bits32 => value as u32 & 0x00ff_ffff,
        }
    }

    fn guest_length(&self, value: i32) -> usize {
        self.guest_address(value) as usize
    }

    fn memory_range(&self, address: u32, length: usize) -> Result<Range<usize>> {
        let start = address as usize;
        let end = start.checked_add(length).ok_or(Error::MemoryOutOfBounds {
            address,
            end: u32::MAX,
        })?;
        if end > self.vm.memory().len() {
            return Err(Error::MemoryOutOfBounds {
                address,
                end: end.saturating_sub(1).min(u32::MAX as usize) as u32,
            });
        }
        Ok(start..end)
    }

    fn read_u8(&self, address: u32) -> Result<u8> {
        let range = self.memory_range(address, 1)?;
        Ok(self.vm.memory()[range.start])
    }

    fn read_u16(&self, address: u32) -> Result<u16> {
        let range = self.memory_range(address, 2)?;
        Ok(u16::from_le_bytes(
            self.vm.memory()[range]
                .try_into()
                .expect("checked two-byte range"),
        ))
    }

    fn read_c_bytes(&self, address: u32) -> Result<Vec<u8>> {
        let start = self.memory_range(address, 1)?.start;
        let memory = self.vm.memory();
        let length =
            memory[start..]
                .iter()
                .position(|&byte| byte == 0)
                .ok_or(Error::MemoryOutOfBounds {
                    address,
                    end: memory.len().saturating_sub(1) as u32,
                })?;
        Ok(memory[start..start + length].to_vec())
    }

    fn decode_guest_string(&self, address: u32) -> Result<String> {
        let bytes = self.read_c_bytes(address)?;
        Ok(GBK.decode(&bytes).0.into_owned())
    }

    fn write_memory(&mut self, address: u32, data: &[u8]) -> Result<()> {
        let range = self.memory_range(address, data.len())?;
        self.vm.memory_mut()[range].copy_from_slice(data);
        Ok(())
    }
}

fn coordinate(value: i32) -> i32 {
    i32::from(value as i16)
}

/// Clamp a rectangle corner like LVM's block_check: the VM casts arguments
/// to unsigned 16-bit words, then values at or beyond the screen edge are
/// clamped to the last pixel.
fn block_clamp(value: i32, limit: i32) -> i32 {
    let value = i32::from(value as u16);
    if value >= limit {
        limit - 1
    } else {
        value
    }
}

/// Integer sine table used by the reference VM (1024 = sin 90 degrees).
const SIN90: [i32; 91] = [
    0, 18, 36, 54, 71, 89, 107, 125, 143, 160, 178, 195, 213, 230, 248, 265,
    282, 299, 316, 333, 350, 367, 384, 400, 416, 433, 449, 465, 481, 496, 512,
    527, 543, 558, 573, 587, 602, 616, 630, 644, 658, 672, 685, 698, 711, 724,
    737, 749, 761, 773, 784, 796, 807, 818, 828, 839, 849, 859, 868, 878, 887,
    896, 904, 912, 920, 928, 935, 943, 949, 956, 962, 968, 974, 979, 984, 989,
    994, 998, 1002, 1005, 1008, 1011, 1014, 1016, 1018, 1020, 1022, 1023, 1023,
    1024, 1024,
];

fn sin_lookup(degrees: i32) -> i32 {
    let degrees = degrees % 360;
    if degrees < 90 {
        SIN90[degrees as usize]
    } else if degrees < 180 {
        SIN90[(180 - degrees) as usize]
    } else if degrees < 270 {
        -SIN90[(degrees - 180) as usize]
    } else {
        -SIN90[(360 - degrees) as usize]
    }
}

/// Map a physical Windows virtual-key code to a LavaX key code (c_keyval).
fn lava_key_from_vk(vk: u16) -> u8 {
    let vk = vk as u8;
    match vk {
        b'A'..=b'Z' => vk | 0x20,
        112..=115 => vk - 112 + 0x1c, // F1-F4
        37 => 23,                      // left
        38 => 20,                      // up
        39 => 22,                      // right
        40 => 21,                      // down
        33 => 19,                      // PageUp
        34 => 14,                      // PageDown
        190 => b'.',
        b'0' | b' ' | b'\r' | 27 => vk,
        16 => 26, // Shift
        b'1'..=b'9' => *b"bnmghjtyu".get((vk - b'1') as usize).unwrap_or(&0),
        116 => 25, // F5
        117 => 18, // F6
        _ => 0,
    }
}

fn target_from_flag(mode: u8, set_means_front: bool) -> BufferTarget {
    let flag = mode & 0x40 != 0;
    if flag == set_means_front {
        BufferTarget::Front
    } else {
        BufferTarget::Back
    }
}

fn packed_length(mode: GraphicsMode, width: u16, height: u16) -> usize {
    let stride = match mode {
        GraphicsMode::Mono => usize::from(width).div_ceil(8),
        GraphicsMode::Color4 => usize::from(width).div_ceil(2),
        GraphicsMode::Color8 => usize::from(width),
    };
    stride * usize::from(height)
}

fn bool_value(value: bool) -> i32 {
    if value { LAVA_TRUE } else { 0 }
}

fn format_float(value: i32) -> Vec<u8> {
    let value = f32::from_bits(value as u32);
    if value.is_finite() {
        value.to_string().into_bytes()
    } else {
        b"error".to_vec()
    }
}

fn parse_extensions(value: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(value);
    let mut filters = text.as_ref();
    while filters.len() >= 2 {
        match &filters[..2] {
            "+h" | "+s" | "!d" | "!f" => filters = &filters[2..],
            _ => break,
        }
    }
    if filters == "*" {
        Vec::new()
    } else {
        filters
            .split(';')
            .filter(|value| !value.is_empty())
            .map(|value| value.trim_start_matches('.').to_ascii_lowercase())
            .collect()
    }
}

fn extension_matches(name: &str, extensions: &[String]) -> bool {
    extensions.is_empty()
        || name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extensions.iter().any(|value| value == extension))
}

#[cfg(test)]
mod tests {
    use crate::{FrameStatus, LAV_HEADER_SIZE, LAV_MAGIC, Program};

    use super::*;

    fn emulator(bytecode: &[u8]) -> Emulator {
        let mut image = vec![0; LAV_HEADER_SIZE];
        image[..4].copy_from_slice(&LAV_MAGIC);
        image[8] = 0xf0;
        image[9] = 15;
        image[10] = 10;
        image.extend_from_slice(bytecode);
        Emulator::new(Program::load(&image).unwrap())
    }

    #[test]
    fn runs_drawing_calls_and_presents_a_frame() {
        let mut emulator = emulator(&[
            0x01, 2, // x
            0x01, 3, // y
            0x01, 0x41, // set on the back buffer
            0x94, // point
            0x89, // present
            0x40,
        ]);
        let frame = emulator.run_frame().unwrap();
        assert_eq!(frame.status, FrameStatus::Halted(0));
        assert_eq!(emulator.display().indexed_frame()[3 * 240 + 2], 255);
    }

    #[test]
    fn waits_for_and_delivers_input() {
        let mut emulator = emulator(&[0x81, 0x38, 0x40]);
        assert_eq!(
            emulator.run_frame().unwrap().status,
            FrameStatus::WaitingForInput
        );
        emulator.input_mut().set_key(b'A', true);
        assert_eq!(emulator.run_frame().unwrap().status, FrameStatus::Halted(0));
        assert_eq!(emulator.vm().result(), i32::from(b'A'));
    }

    #[test]
    fn provides_deterministic_virtual_files() {
        let mut emulator = emulator(&[0x40]);
        emulator
            .files_mut()
            .import_file("/data/save.dat", vec![1, 2, 3]);
        let handle = emulator.files_mut().open("/data/save.dat", "rb").unwrap();
        assert_eq!(emulator.files_mut().read(handle, 3).unwrap(), [1, 2, 3]);
    }

    #[test]
    fn crc_matches_the_ccitt_check_value() {
        let mut crc = 0u16;
        for &byte in b"123456789" {
            crc ^= u16::from(byte) << 8;
            for _ in 0..8 {
                crc = if crc & 0x8000 != 0 {
                    (crc << 1) ^ 0x1021
                } else {
                    crc << 1
                };
            }
        }
        assert_eq!(crc, 0x31c3);
    }
}
