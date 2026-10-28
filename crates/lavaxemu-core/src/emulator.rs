use crate::filelist::FileListState;
use crate::{
    BufferTarget, Display, DrawOperation, GraphicsMode, InputState, Program, Result, RunOutcome,
    VirtualFileSystem, Vm,
};
use serde::{Deserialize, Serialize};

const VM_SLICE_INSTRUCTIONS: usize = 1_001;
const DEFAULT_FRAME_BUDGET: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStatus {
    Running,
    Presented,
    WaitingForInput,
    Delayed,
    Halted(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameResult {
    pub status: FrameStatus,
    pub instructions: usize,
    pub system_calls: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostAction {
    Continue,
    Present,
    WaitForInput,
    Delay,
    Halt(i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TextConsole {
    cells: Vec<u8>,
    columns: usize,
    rows: usize,
    x: usize,
    y: usize,
    small: bool,
    small_left: usize,
    small_up: usize,
    small_down: usize,
}

impl TextConsole {
    fn new(width: u16, height: u16) -> Self {
        let mut console = Self {
            cells: Vec::new(),
            columns: 0,
            rows: 0,
            x: 0,
            y: 0,
            small: false,
            small_left: 0,
            small_up: 0,
            small_down: 0,
        };
        console.set_mode(width, height, false);
        console
    }

    pub(crate) fn set_mode(&mut self, width: u16, height: u16, small: bool) {
        self.small = small;
        if small {
            self.columns = (usize::from(width).saturating_sub(2) / 6) & !1;
            self.rows = usize::from(height).saturating_sub(1) / 13;
            self.small_left = (usize::from(width) - self.columns * 6) / 2;
            self.small_up = (usize::from(height) - (self.rows * 13 - 1)) / 2;
            self.small_down = usize::from(height) - (self.rows * 13 - 1) - self.small_up;
        } else {
            self.columns = usize::from(width) / 8;
            self.rows = usize::from(height) / 16;
            self.small_left = 0;
            self.small_up = 0;
            self.small_down = 0;
        }
        self.columns = self.columns.max(1);
        self.rows = self.rows.max(1);
        self.cells = vec![b' '; self.columns * self.rows];
        self.x = 0;
        self.y = 0;
    }

    pub(crate) fn locate(&mut self, x: usize, y: usize) {
        self.x = x.min(self.columns - 1);
        self.y = y.min(self.rows - 1);
    }

    pub(crate) fn write(&mut self, byte: u8) {
        match byte {
            b'\r' => return,
            b'\n' => {
                self.x = 0;
                self.y += 1;
            }
            b'\t' => self.write(b' '),
            value => {
                self.scroll_if_needed();
                let index = self.y * self.columns + self.x;
                self.cells[index] = value;
                self.x += 1;
                if self.x >= self.columns {
                    self.x = 0;
                    self.y += 1;
                }
            }
        }
        self.scroll_if_needed();
    }

    fn scroll_if_needed(&mut self) {
        if self.y < self.rows {
            return;
        }
        self.cells.copy_within(self.columns.., 0);
        let last_row = self.columns * (self.rows - 1);
        self.cells[last_row..].fill(b' ');
        self.y = self.rows - 1;
        self.x = 0;
    }

    pub(crate) fn render(&self, display: &mut Display) {
        if self.small {
            let background = if display.graphics_mode() == GraphicsMode::Mono {
                0
            } else {
                display.background()
            };
            let width = display.width();
            let height = display.height();
            // Top and bottom margins, plus separator rows between lines.
            display.fill_region(
                BufferTarget::Front,
                0,
                0,
                width,
                self.small_up as u16,
                background,
            );
            display.fill_region(
                BufferTarget::Front,
                0,
                i32::from(height) - self.small_down as i32,
                width,
                self.small_down as u16,
                background,
            );
            for row in 1..self.rows {
                let y = (row * 13 + self.small_up - 1) as i32;
                display.fill_region(BufferTarget::Front, 0, y, width, 1, background);
            }
            // Left and right margins.
            for column in 0..self.small_left {
                display.fill_region(BufferTarget::Front, column as i32, 0, 1, height, background);
                display.fill_region(
                    BufferTarget::Front,
                    i32::from(width) - 1 - column as i32,
                    0,
                    1,
                    height,
                    background,
                );
            }
            for row in 0..self.rows {
                let start = row * self.columns;
                let end = start + self.columns;
                display.draw_text(
                    BufferTarget::Front,
                    self.small_left as i32,
                    (row * 13 + self.small_up) as i32,
                    &self.cells[start..end],
                    false,
                    DrawOperation::Set,
                );
            }
        } else {
            for row in 0..self.rows {
                let start = row * self.columns;
                let end = start + self.columns;
                display.draw_text(
                    BufferTarget::Front,
                    0,
                    (row * 16) as i32,
                    &self.cells[start..end],
                    true,
                    DrawOperation::Set,
                );
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emulator {
    pub(crate) vm: Vm,
    pub(crate) display: Display,
    pub(crate) input: InputState,
    pub(crate) files: VirtualFileSystem,
    pub(crate) console: TextConsole,
    pub(crate) elapsed_ms: u64,
    pub(crate) time_remainder: u32,
    pub(crate) frame_index: u64,
    pub(crate) delay_remaining_ticks: u32,
    pub(crate) delay_tick_carry: u32,
    pub(crate) waiting_for_key: bool,
    pub(crate) random_seed: i32,
    pub(crate) calendar: [u8; 8],
    pub(crate) command_line: Vec<u8>,
    pub(crate) file_list: FileListState,
    pub(crate) legacy_graphics_memory: bool,
}

impl Emulator {
    pub fn new(program: Program) -> Self {
        let header = program.header().clone();
        Self {
            vm: Vm::new(program),
            display: Display::new(header.width, header.height, header.graphics_mode),
            input: InputState::default(),
            files: VirtualFileSystem::default(),
            console: TextConsole::new(header.width, header.height),
            elapsed_ms: 0,
            time_remainder: 0,
            frame_index: 0,
            delay_remaining_ticks: 0,
            delay_tick_carry: 0,
            waiting_for_key: false,
            random_seed: 1,
            calendar: [0xd0, 0x07, 1, 1, 0, 0, 0, 6],
            command_line: Vec::new(),
            file_list: FileListState::default(),
            legacy_graphics_memory: false,
        }
    }

    pub fn reset(&mut self) {
        self.vm.reset();
        self.display.reset();
        self.console
            .set_mode(self.display.width(), self.display.height(), false);
        self.input.release_all();
        self.elapsed_ms = 0;
        self.time_remainder = 0;
        self.frame_index = 0;
        self.delay_remaining_ticks = 0;
        self.delay_tick_carry = 0;
        self.waiting_for_key = false;
        self.random_seed = 1;
        self.file_list = FileListState::default();
        self.legacy_graphics_memory = false;
    }

    pub const fn vm(&self) -> &Vm {
        &self.vm
    }

    pub fn vm_mut(&mut self) -> &mut Vm {
        &mut self.vm
    }

    pub const fn display(&self) -> &Display {
        &self.display
    }

    pub fn input_mut(&mut self) -> &mut InputState {
        &mut self.input
    }

    pub const fn files(&self) -> &VirtualFileSystem {
        &self.files
    }

    pub fn files_mut(&mut self) -> &mut VirtualFileSystem {
        &mut self.files
    }

    pub const fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }

    pub fn set_command_line(&mut self, command_line: impl Into<Vec<u8>>) {
        self.command_line = command_line.into();
    }

    pub fn run_frame(&mut self) -> Result<FrameResult> {
        self.run_frame_with_budget(DEFAULT_FRAME_BUDGET)
    }

    pub fn run_frame_with_budget(&mut self, budget: usize) -> Result<FrameResult> {
        let result = self.run_frame_inner(budget);
        self.frame_index += 1;
        result
    }

    fn run_frame_inner(&mut self, budget: usize) -> Result<FrameResult> {
        self.advance_frame_clock();
        if !self.vm.is_running() {
            return Ok(self.frame_result(FrameStatus::Halted(self.vm.exit_code()), 0, 0));
        }
        if self.file_list.destination.is_some() && self.poll_file_list()? {
            return Ok(self.frame_result(FrameStatus::WaitingForInput, 0, 0));
        }
        if self.waiting_for_key {
            if let Some(value) = self.pending_input_value() {
                self.vm.push_value(value)?;
                self.waiting_for_key = false;
            } else {
                return Ok(self.frame_result(FrameStatus::WaitingForInput, 0, 0));
            }
        }
        self.advance_delay_ticks();
        if self.delay_remaining_ticks != 0 {
            return Ok(self.frame_result(FrameStatus::Delayed, 0, 0));
        }

        let start = self.vm.instructions_executed();
        let mut system_calls = 0;
        loop {
            let executed = (self.vm.instructions_executed() - start) as usize;
            if executed >= budget {
                return Ok(self.frame_result(FrameStatus::Running, executed, system_calls));
            }
            let slice = (budget - executed).min(VM_SLICE_INSTRUCTIONS);
            let outcome = self.vm.run(slice)?;
            self.read_graphics_memory();
            match outcome {
                RunOutcome::GraphicsBuffer { back } => {
                    self.request_graphics_memory(back)?;
                }
                RunOutcome::BudgetExhausted => {}
                RunOutcome::Halted(code) => {
                    let executed = (self.vm.instructions_executed() - start) as usize;
                    return Ok(self.frame_result(
                        FrameStatus::Halted(code),
                        executed,
                        system_calls,
                    ));
                }
                RunOutcome::SystemCall(call) => {
                    system_calls += 1;
                    let action = self.dispatch_system_call(call)?;
                    self.sync_graphics_system_call(call);
                    let status = match action {
                        HostAction::Continue => continue,
                        HostAction::Present => FrameStatus::Presented,
                        HostAction::WaitForInput => FrameStatus::WaitingForInput,
                        HostAction::Delay => FrameStatus::Delayed,
                        HostAction::Halt(code) => FrameStatus::Halted(code),
                    };
                    let executed = (self.vm.instructions_executed() - start) as usize;
                    return Ok(self.frame_result(status, executed, system_calls));
                }
            }
        }
    }

    fn frame_result(
        &self,
        status: FrameStatus,
        instructions: usize,
        system_calls: usize,
    ) -> FrameResult {
        FrameResult {
            status,
            instructions,
            system_calls,
        }
    }

    fn advance_frame_clock(&mut self) {
        let previous_second = self.elapsed_ms / 1_000;
        self.time_remainder += 1_000;
        self.elapsed_ms += u64::from(self.time_remainder / 60);
        self.time_remainder %= 60;
        if self.elapsed_ms / 1_000 != previous_second {
            self.advance_calendar_second();
        }
    }

    fn advance_calendar_second(&mut self) {
        for (index, limit) in [(6, 60), (5, 60), (4, 24)] {
            self.calendar[index] = self.calendar[index].wrapping_add(1);
            if self.calendar[index] < limit {
                return;
            }
            self.calendar[index] = 0;
        }
        self.calendar[7] = (self.calendar[7] + 1) % 7;
        let year = u16::from_le_bytes([self.calendar[0], self.calendar[1]]);
        let days = match self.calendar[2] {
            2 if year.is_multiple_of(4)
                && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
            {
                29
            }
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        self.calendar[3] = self.calendar[3].wrapping_add(1);
        if self.calendar[3] <= days {
            return;
        }
        self.calendar[3] = 1;
        self.calendar[2] = self.calendar[2].wrapping_add(1);
        if self.calendar[2] > 12 {
            self.calendar[2] = 1;
            self.calendar[..2].copy_from_slice(&year.wrapping_add(1).to_le_bytes());
        }
    }

    /// Advance the delay counter using LVM timing: delay is counted in
    /// 1/256-second ticks and the host refreshes at 60 Hz, so one frame
    /// consumes 256/60 ticks.
    fn advance_delay_ticks(&mut self) {
        self.delay_tick_carry += 256;
        while self.delay_tick_carry >= 60 {
            self.delay_tick_carry -= 60;
            if self.delay_remaining_ticks != 0 {
                self.delay_remaining_ticks -= 1;
            }
        }
    }

    pub(crate) fn pending_input_value(&mut self) -> Option<i32> {
        if let Some(key) = self.input.pop_key() {
            return Some(i32::from(key));
        }
        let pointer = self.input.pointer()?;
        if pointer.pressed && pointer.x >= 0 && pointer.y >= 0 {
            return Some(
                ((i32::from(pointer.x) & 0xff) << 8)
                    | ((i32::from(pointer.y) & 0xff) << 16)
                    | 0xff00_0000_u32 as i32,
            );
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emulator() -> Emulator {
        let mut image = vec![0; crate::LAV_HEADER_SIZE];
        image[..4].copy_from_slice(&crate::LAV_MAGIC);
        image.push(0x40);
        Emulator::new(Program::load(&image).unwrap())
    }

    #[test]
    fn advances_calendar_at_sixty_frames_per_second_and_restores_its_phase() {
        let mut emulator = emulator();
        for _ in 0..59 {
            emulator.advance_frame_clock();
        }
        assert_eq!(emulator.calendar[6], 0);
        let state = emulator.save_state().unwrap();
        emulator.advance_frame_clock();
        assert_eq!(emulator.calendar[6], 1);
        emulator.load_state(&state).unwrap();
        emulator.advance_frame_clock();
        assert_eq!(emulator.calendar[6], 1);
        for _ in 0..3540 {
            emulator.advance_frame_clock();
        }
        assert_eq!(&emulator.calendar[4..7], &[0, 1, 0]);
    }

    #[test]
    fn rolls_over_leap_days_months_and_years() {
        let mut emulator = emulator();
        for (year, month, day, expected_year, expected_month, expected_day) in [
            (2000u16, 2, 28, 2000u16, 2, 29),
            (1900, 2, 28, 1900, 3, 1),
            (2024, 2, 29, 2024, 3, 1),
            (2024, 4, 30, 2024, 5, 1),
            (2024, 12, 31, 2025, 1, 1),
        ] {
            let [low, high] = year.to_le_bytes();
            emulator.calendar = [low, high, month, day, 23, 59, 59, 6];
            emulator.advance_calendar_second();
            let [low, high] = expected_year.to_le_bytes();
            assert_eq!(
                emulator.calendar,
                [low, high, expected_month, expected_day, 0, 0, 0, 0]
            );
        }
    }

    #[test]
    fn gettime_reports_elapsed_seconds_after_settime() {
        let mut emulator = emulator();
        let date = [0xe8, 0x07, 12, 31, 23, 59, 59, 2];
        emulator.vm.memory_mut()[0x2000..0x2008].copy_from_slice(&date);
        emulator.vm.push_value(0x2000).unwrap();
        emulator.dispatch_system_call(67).unwrap();
        for _ in 0..60 {
            emulator.advance_frame_clock();
        }
        emulator.vm.push_value(0x2000).unwrap();
        emulator.dispatch_system_call(66).unwrap();
        assert_eq!(
            &emulator.vm.memory()[0x2000..0x2008],
            &[0xe9, 0x07, 1, 1, 0, 0, 0, 3]
        );
    }
}
