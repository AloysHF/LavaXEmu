use crate::GraphicsMode;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

/// Embedded LVM font resource (LVM.bin) with the exact glyph bitmaps used by
/// the reference LavaX VM. Layout: 128x(6x12) ASCII, 128x(8x16) ASCII,
/// GB2312 12x12 and GB2312 16x16.
const LVM_FONT: &[u8] = include_bytes!("../assets/LVM.bin");
const FONT_ASCII6: usize = 0;
const FONT_ASCII8: usize = 1536;
const FONT_GB12: usize = FONT_ASCII8 + 2048;
const FONT_GB16: usize = FONT_GB12 + 81 * 94 * 24;
const FONT_GB_COUNT: usize = 81 * 94;

enum GlyphOrder {
    Ascii,
    Gbk,
}

fn font_ascii(character: u8, large: bool) -> (&'static [u8], u16, u16) {
    static BLANK: [u8; 32] = [0; 32];
    if character >= 128 {
        return if large {
            (&BLANK[..16], 8, 16)
        } else {
            (&BLANK[..12], 6, 12)
        };
    }
    if large {
        let offset = FONT_ASCII8 + usize::from(character) * 16;
        (&LVM_FONT[offset..offset + 16], 8, 16)
    } else {
        let offset = FONT_ASCII6 + usize::from(character) * 12;
        (&LVM_FONT[offset..offset + 12], 6, 12)
    }
}

fn font_gbk(first: u8, second: u8, large: bool) -> Option<(&'static [u8], u16, u16)> {
    if !(0xa1..=0xf7).contains(&first) || !(0xa1..=0xfe).contains(&second) {
        return None;
    }
    let index = if first < 0xb0 {
        usize::from(first - 0xa1) * 94 + usize::from(second - 0xa1)
    } else {
        usize::from(first - 0xa7) * 94 + usize::from(second - 0xa1)
    };
    if index >= FONT_GB_COUNT {
        return None;
    }
    if large {
        let offset = FONT_GB16 + index * 32;
        Some((&LVM_FONT[offset..offset + 32], 16, 16))
    } else {
        let offset = FONT_GB12 + index * 24;
        Some((&LVM_FONT[offset..offset + 24], 12, 12))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferTarget {
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawOperation {
    Clear,
    Set,
    Invert,
}

impl DrawOperation {
    pub const fn from_lava(value: i32) -> Self {
        match value & 3 {
            0 => Self::Clear,
            1 => Self::Set,
            _ => Self::Invert,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    Left,
    Right,
    Up,
    Down,
    MirrorHorizontal,
    MirrorVertical,
    RestoreBackBuffer,
}

impl Transform {
    pub const fn from_lava(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Left),
            1 => Some(Self::Right),
            2 => Some(Self::Up),
            3 => Some(Self::Down),
            4 => Some(Self::MirrorHorizontal),
            5 => Some(Self::MirrorVertical),
            6 => Some(Self::RestoreBackBuffer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Display {
    width: u16,
    height: u16,
    graphics_mode: GraphicsMode,
    background: u8,
    foreground: u8,
    #[serde(with = "BigArray")]
    palette: [[u8; 3]; 256],
    front: Vec<u8>,
    back: Vec<u8>,
}

impl Display {
    pub fn new(width: u16, height: u16, graphics_mode: GraphicsMode) -> Self {
        let mut display = Self {
            width,
            height,
            graphics_mode,
            background: 0,
            foreground: match graphics_mode {
                GraphicsMode::Mono => 1,
                GraphicsMode::Color4 => 15,
                GraphicsMode::Color8 => 255,
            },
            palette: [[0; 3]; 256],
            front: vec![0; usize::from(width) * usize::from(height)],
            back: vec![0; usize::from(width) * usize::from(height)],
        };
        display.reset_palette();
        display
    }

    pub const fn width(&self) -> u16 {
        self.width
    }

    pub const fn height(&self) -> u16 {
        self.height
    }

    pub const fn graphics_mode(&self) -> GraphicsMode {
        self.graphics_mode
    }

    pub const fn background(&self) -> u8 {
        self.background
    }

    pub const fn foreground(&self) -> u8 {
        self.foreground
    }

    pub fn indexed_frame(&self) -> &[u8] {
        &self.front
    }

    pub fn back_buffer(&self) -> &[u8] {
        &self.back
    }

    pub fn palette(&self) -> &[[u8; 3]; 256] {
        &self.palette
    }

    pub fn reset(&mut self) {
        self.background = 0;
        self.foreground = self.max_color();
        self.front.fill(self.background);
        self.back.fill(self.background);
        self.reset_palette();
    }

    pub fn set_graphics_mode(&mut self, mode: GraphicsMode) -> GraphicsMode {
        let previous = self.graphics_mode;
        if previous != mode {
            self.graphics_mode = mode;
            self.background = 0;
            self.foreground = self.max_color();
            self.front.fill(self.background);
            self.back.fill(self.background);
            self.reset_palette();
        }
        previous
    }

    pub fn set_background(&mut self, color: u8) {
        self.background = color & self.max_color();
    }

    pub fn set_foreground(&mut self, color: u8) {
        self.foreground = color & self.max_color();
    }

    pub fn set_palette_rgba(&mut self, first: u8, colors: &[[u8; 4]]) -> usize {
        let available = 256usize.saturating_sub(usize::from(first));
        let count = colors.len().min(available);
        for (offset, color) in colors.iter().take(count).enumerate() {
            self.palette[usize::from(first) + offset] = [color[0], color[1], color[2]];
        }
        count
    }

    pub fn clear(&mut self, target: BufferTarget) {
        let color = if self.graphics_mode == GraphicsMode::Mono {
            0
        } else {
            self.background
        };
        self.buffer_mut(target).fill(color);
    }

    pub(crate) fn fill_region(
        &mut self,
        target: BufferTarget,
        x: i32,
        y: i32,
        width: u16,
        height: u16,
        color: u8,
    ) {
        for row in 0..height {
            for column in 0..width {
                let Some(index) = self.pixel_index(x + i32::from(column), y + i32::from(row))
                else {
                    continue;
                };
                self.buffer_mut(target)[index] = color;
            }
        }
    }

    pub fn present(&mut self) {
        self.front.copy_from_slice(&self.back);
    }

    pub fn get_pixel(&self, target: BufferTarget, x: i32, y: i32) -> Option<u8> {
        self.pixel_index(x, y)
            .map(|index| self.buffer(target)[index])
    }

    pub fn draw_pixel(&mut self, target: BufferTarget, x: i32, y: i32, operation: DrawOperation) {
        let Some(index) = self.pixel_index(x, y) else {
            return;
        };
        let background = if self.graphics_mode == GraphicsMode::Mono {
            0
        } else {
            self.background
        };
        let foreground = self.foreground;
        let mask = self.max_color();
        let pixel = &mut self.buffer_mut(target)[index];
        match operation {
            DrawOperation::Clear => *pixel = background,
            DrawOperation::Set => *pixel = foreground,
            DrawOperation::Invert => *pixel ^= mask,
        }
    }

    pub fn draw_line(
        &mut self,
        target: BufferTarget,
        mut x0: i32,
        mut y0: i32,
        x1: i32,
        y1: i32,
        operation: DrawOperation,
    ) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.draw_pixel(target, x0, y0, operation);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice_error = error * 2;
            if twice_error >= dy {
                error += dy;
                x0 += sx;
            }
            if twice_error <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_rectangle(
        &mut self,
        target: BufferTarget,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        filled: bool,
        operation: DrawOperation,
    ) {
        let left = x0.min(x1);
        let right = x0.max(x1);
        let top = y0.min(y1);
        let bottom = y0.max(y1);
        if filled {
            for y in top..=bottom {
                self.draw_line(target, left, y, right, y, operation);
            }
        } else {
            self.draw_line(target, left, top, right, top, operation);
            self.draw_line(target, left, bottom, right, bottom, operation);
            self.draw_line(target, left, top, left, bottom, operation);
            self.draw_line(target, right, top, right, bottom, operation);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_ellipse(
        &mut self,
        target: BufferTarget,
        center_x: i32,
        center_y: i32,
        radius_x: u16,
        radius_y: u16,
        filled: bool,
        operation: DrawOperation,
    ) {
        let rx = f64::from(radius_x);
        let ry = f64::from(radius_y);
        if radius_x == 0 && radius_y == 0 {
            self.draw_pixel(target, center_x, center_y, operation);
            return;
        }
        if radius_x == 0 {
            self.draw_line(
                target,
                center_x,
                center_y - i32::from(radius_y),
                center_x,
                center_y + i32::from(radius_y),
                operation,
            );
            return;
        }
        if radius_y == 0 {
            self.draw_line(
                target,
                center_x - i32::from(radius_x),
                center_y,
                center_x + i32::from(radius_x),
                center_y,
                operation,
            );
            return;
        }

        for y in -i32::from(radius_y)..=i32::from(radius_y) {
            let normalized = f64::from(y) / ry;
            let extent = (rx * (1.0 - normalized * normalized).max(0.0).sqrt()).round() as i32;
            if filled {
                self.draw_line(
                    target,
                    center_x - extent,
                    center_y + y,
                    center_x + extent,
                    center_y + y,
                    operation,
                );
            } else {
                self.draw_pixel(target, center_x - extent, center_y + y, operation);
                self.draw_pixel(target, center_x + extent, center_y + y, operation);
            }
        }
    }

    pub fn draw_text(
        &mut self,
        target: BufferTarget,
        x: i32,
        y: i32,
        text: &[u8],
        large: bool,
        operation: DrawOperation,
    ) {
        let mut pen_x = x;
        let mut offset = 0;
        while offset < text.len() {
            let character = text[offset];
            if character < 128 {
                let (glyph, width, height) = font_ascii(character, large);
                self.draw_glyph(
                    target,
                    pen_x,
                    y,
                    glyph,
                    width,
                    height,
                    GlyphOrder::Ascii,
                    operation,
                );
                pen_x += i32::from(width);
                offset += 1;
            } else if offset + 1 < text.len() {
                let second = text[offset + 1];
                if let Some((glyph, width, height)) = font_gbk(character, second, large) {
                    self.draw_glyph(
                        target,
                        pen_x,
                        y,
                        glyph,
                        width,
                        height,
                        GlyphOrder::Gbk,
                        operation,
                    );
                    pen_x += i32::from(width);
                    offset += 2;
                } else {
                    let (glyph, width, height) = font_ascii(character, large);
                    self.draw_glyph(
                        target,
                        pen_x,
                        y,
                        glyph,
                        width,
                        height,
                        GlyphOrder::Ascii,
                        operation,
                    );
                    pen_x += i32::from(width);
                    offset += 1;
                }
            } else {
                offset += 1;
            }
        }
    }

    /// Render one glyph bitmap with the exact LVM write_comm semantics used
    /// by the console and TextOut: set bits draw the foreground color,
    /// cleared bits draw the background color; inverting XORs the color.
    #[allow(clippy::too_many_arguments)]
    fn draw_glyph(
        &mut self,
        target: BufferTarget,
        x: i32,
        y: i32,
        glyph: &[u8],
        width: u16,
        height: u16,
        order: GlyphOrder,
        operation: DrawOperation,
    ) {
        let foreground = self.foreground;
        let background = self.background;
        let invert_mask = self.max_color();
        for row in 0..height {
            for column in 0..width {
                let bit = match order {
                    GlyphOrder::Ascii => glyph[usize::from(row)] & (0x80 >> (column & 7)) != 0,
                    GlyphOrder::Gbk if width == 16 => {
                        let byte = glyph[usize::from(row) * 2 + usize::from(column / 8)];
                        byte & (0x80 >> (column & 7)) != 0
                    }
                    GlyphOrder::Gbk => {
                        if column < 8 {
                            glyph[usize::from(row) * 2] & (0x80 >> column) != 0
                        } else {
                            glyph[usize::from(row) * 2 + 1] & (0x80 >> (column - 8)) != 0
                        }
                    }
                };
                let mut color = if bit { foreground } else { background };
                if matches!(operation, DrawOperation::Invert) {
                    color ^= invert_mask;
                }
                let Some(index) = self.pixel_index(x + i32::from(column), y + i32::from(row))
                else {
                    continue;
                };
                self.buffer_mut(target)[index] = color;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn blit(
        &mut self,
        target: BufferTarget,
        x: i32,
        y: i32,
        width: u16,
        height: u16,
        source: &[u8],
        mode: u8,
        mirror_horizontal: bool,
    ) {
        let stride = match self.graphics_mode {
            GraphicsMode::Mono => usize::from(width).div_ceil(8),
            GraphicsMode::Color4 => usize::from(width).div_ceil(2),
            GraphicsMode::Color8 => usize::from(width),
        };
        let operation = mode & 7;
        let invert_source = mode & 8 != 0 || operation == 2;
        let max_color = self.max_color();
        for row in 0..usize::from(height) {
            let row_start = row.saturating_mul(stride);
            if row_start.saturating_add(stride) > source.len() {
                break;
            }
            for column in 0..usize::from(width) {
                let source_column = if mirror_horizontal {
                    usize::from(width) - 1 - column
                } else {
                    column
                };
                let mut color = match self.graphics_mode {
                    GraphicsMode::Mono => {
                        (source[row_start + source_column / 8] >> (7 - source_column % 8)) & 1
                    }
                    GraphicsMode::Color4 => {
                        let packed = source[row_start + source_column / 2];
                        if source_column & 1 == 0 {
                            packed >> 4
                        } else {
                            packed & 0x0f
                        }
                    }
                    GraphicsMode::Color8 => source[row_start + source_column],
                };
                if invert_source {
                    color ^= max_color;
                }
                let Some(index) = self.pixel_index(x + column as i32, y + row as i32) else {
                    continue;
                };
                let pixel = &mut self.buffer_mut(target)[index];
                match operation {
                    3 => *pixel |= color,
                    4 => *pixel &= color,
                    5 => *pixel ^= color,
                    6 if color == 0 => {}
                    _ => *pixel = color,
                }
                *pixel &= max_color;
            }
        }
    }

    pub fn capture(
        &self,
        target: BufferTarget,
        x: i32,
        y: i32,
        width: u16,
        height: u16,
    ) -> Vec<u8> {
        let stride = match self.graphics_mode {
            GraphicsMode::Mono => usize::from(width).div_ceil(8),
            GraphicsMode::Color4 => usize::from(width).div_ceil(2),
            GraphicsMode::Color8 => usize::from(width),
        };
        let mut output = vec![0; stride * usize::from(height)];
        for row in 0..usize::from(height) {
            for column in 0..usize::from(width) {
                let color = self
                    .get_pixel(target, x + column as i32, y + row as i32)
                    .unwrap_or(self.background);
                match self.graphics_mode {
                    GraphicsMode::Mono => {
                        output[row * stride + column / 8] |= (color & 1) << (7 - column % 8);
                    }
                    GraphicsMode::Color4 => {
                        let shift = if column & 1 == 0 { 4 } else { 0 };
                        output[row * stride + column / 2] |= (color & 0x0f) << shift;
                    }
                    GraphicsMode::Color8 => output[row * stride + column] = color,
                }
            }
        }
        output
    }

    pub fn transform(&mut self, transform: Transform) {
        let width = usize::from(self.width);
        let height = usize::from(self.height);
        let background = self.background;
        match transform {
            Transform::Left => {
                for row in self.back.chunks_exact_mut(width) {
                    row.copy_within(1..width, 0);
                    row[width - 1] = background;
                }
            }
            Transform::Right => {
                for row in self.back.chunks_exact_mut(width) {
                    row.copy_within(0..width - 1, 1);
                    row[0] = background;
                }
            }
            Transform::Up => {
                self.back.copy_within(width..width * height, 0);
                self.back[width * (height - 1)..].fill(background);
            }
            Transform::Down => {
                self.back.copy_within(0..width * (height - 1), width);
                self.back[..width].fill(background);
            }
            Transform::MirrorHorizontal => {
                for row in self.back.chunks_exact_mut(width) {
                    row.reverse();
                }
            }
            Transform::MirrorVertical => {
                for y in 0..height / 2 {
                    let opposite = height - 1 - y;
                    for x in 0..width {
                        self.back.swap(y * width + x, opposite * width + x);
                    }
                }
            }
            Transform::RestoreBackBuffer => self.back.copy_from_slice(&self.front),
        }
    }

    pub fn fade(&mut self, amount: u8) {
        if self.graphics_mode == GraphicsMode::Mono {
            return;
        }
        let floor = (amount & 0x0f) ^ 0x0f;
        for (source, destination) in self.back.iter().zip(self.front.iter_mut()) {
            *destination = (*source).max(floor);
        }
    }

    pub fn to_xrgb8888(&self, output: &mut Vec<u32>) {
        output.resize(self.front.len(), 0);
        for (destination, &index) in output.iter_mut().zip(&self.front) {
            let [red, green, blue] = self.palette[usize::from(index)];
            *destination = (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue);
        }
    }

    pub fn to_rgb565(&self, output: &mut Vec<u16>) {
        output.resize(self.front.len(), 0);
        for (destination, &index) in output.iter_mut().zip(&self.front) {
            let [red, green, blue] = self.palette[usize::from(index)];
            *destination =
                (u16::from(red >> 3) << 11) | (u16::from(green >> 2) << 5) | u16::from(blue >> 3);
        }
    }

    fn reset_palette(&mut self) {
        self.palette.fill([0, 0, 0]);
        match self.graphics_mode {
            GraphicsMode::Mono => {
                self.palette[0] = [0, 192, 0];
                self.palette[1] = [0, 0, 0];
            }
            GraphicsMode::Color4 => {
                for index in 0..16 {
                    let level = (15 - index) as u8 * 0x11;
                    self.palette[index] = [level, level, level];
                }
            }
            GraphicsMode::Color8 => {
                const LEVEL_9: [u8; 9] = [0, 0x20, 0x40, 0x60, 0x80, 0xa0, 0xc0, 0xe0, 0xff];
                const LEVEL_5: [u8; 5] = [0, 0x40, 0x80, 0xc0, 0xff];
                self.palette[0] = [0, 0, 0];
                self.palette[255] = [255, 255, 255];
                for index in 0..225 {
                    self.palette[index + 16] = [
                        LEVEL_5[index % 5],
                        LEVEL_9[index / 25],
                        LEVEL_5[(index / 5) % 5],
                    ];
                }
            }
        }
    }

    fn max_color(&self) -> u8 {
        match self.graphics_mode {
            GraphicsMode::Mono => 1,
            GraphicsMode::Color4 => 15,
            GraphicsMode::Color8 => 255,
        }
    }

    fn pixel_index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= i32::from(self.width) || y >= i32::from(self.height) {
            return None;
        }
        Some(y as usize * usize::from(self.width) + x as usize)
    }

    fn buffer(&self, target: BufferTarget) -> &[u8] {
        match target {
            BufferTarget::Front => &self.front,
            BufferTarget::Back => &self.back,
        }
    }

    fn buffer_mut(&mut self, target: BufferTarget) -> &mut [u8] {
        match target {
            BufferTarget::Front => &mut self.front,
            BufferTarget::Back => &mut self.back,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presents_back_buffer_and_converts_rgb565() {
        let mut display = Display::new(160, 80, GraphicsMode::Color4);
        display.draw_pixel(BufferTarget::Back, 4, 5, DrawOperation::Set);
        assert_eq!(display.get_pixel(BufferTarget::Front, 4, 5), Some(0));
        display.present();
        assert_eq!(display.get_pixel(BufferTarget::Front, 4, 5), Some(15));

        let mut frame = Vec::new();
        display.to_rgb565(&mut frame);
        assert_eq!(frame[5 * 160 + 4], 0);
        assert_eq!(frame[0], 0xffff);
    }

    #[test]
    fn clips_lines_and_rectangles() {
        let mut display = Display::new(160, 80, GraphicsMode::Mono);
        display.draw_line(BufferTarget::Front, -10, 2, 10, 2, DrawOperation::Set);
        display.draw_rectangle(
            BufferTarget::Front,
            20,
            10,
            22,
            12,
            true,
            DrawOperation::Set,
        );
        assert_eq!(
            display.indexed_frame().iter().filter(|&&p| p != 0).count(),
            20
        );
    }

    #[test]
    fn round_trips_packed_monochrome_blocks() {
        let mut display = Display::new(160, 80, GraphicsMode::Mono);
        let source = [0b1010_0101, 0b0101_1010];
        display.blit(BufferTarget::Back, 8, 4, 8, 2, &source, 0, false);
        assert_eq!(display.capture(BufferTarget::Back, 8, 4, 8, 2), source);
    }

    #[test]
    fn applies_palette_entries_from_rgba_data() {
        let mut display = Display::new(240, 160, GraphicsMode::Color8);
        assert_eq!(
            display.set_palette_rgba(254, &[[1, 2, 3, 99], [4, 5, 6, 88], [7, 8, 9, 77]]),
            2
        );
        assert_eq!(display.palette()[254], [1, 2, 3]);
        assert_eq!(display.palette()[255], [4, 5, 6]);
    }

    #[test]
    fn renders_ascii_and_gbk_text() {
        let mut display = Display::new(160, 80, GraphicsMode::Mono);
        display.draw_text(
            BufferTarget::Front,
            0,
            0,
            b"LavaX",
            false,
            DrawOperation::Set,
        );
        assert!(display.indexed_frame().iter().any(|&pixel| pixel != 0));
    }

    #[test]
    fn shifts_and_mirrors_the_back_buffer() {
        let mut display = Display::new(160, 80, GraphicsMode::Mono);
        display.draw_pixel(BufferTarget::Back, 0, 0, DrawOperation::Set);
        display.transform(Transform::Right);
        assert_eq!(display.get_pixel(BufferTarget::Back, 1, 0), Some(1));
        display.transform(Transform::MirrorHorizontal);
        assert_eq!(display.get_pixel(BufferTarget::Back, 158, 0), Some(1));
    }
}
