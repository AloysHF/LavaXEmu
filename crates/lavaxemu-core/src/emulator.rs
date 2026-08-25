use crate::{
    BufferTarget, Display, DrawOperation, InputState, Program, Result, RunOutcome,
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
        };
        console.set_mode(width, height, false);
        console
    }

    pub(crate) fn set_mode(&mut self, width: u16, height: u16, small: bool) {
        self.small = small;
        if small {
            self.columns = (usize::from(width).saturating_sub(2) / 6) & !1;
            self.rows = usize::from(height).saturating_sub(1) / 13;
        } else {
            self.columns = usize::from(width) / 8;
            self.rows = usize::from(height) / 16;
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
        display.clear(BufferTarget::Front);
        let character_height = if self.small { 13 } else { 16 };
        let large = !self.small;
        for row in 0..self.rows {
            let start = row * self.columns;
            let mut end = start + self.columns;
            while end > start && self.cells[end - 1] == b' ' {
                end -= 1;
            }
            display.draw_text(
                BufferTarget::Front,
                0,
                (row * character_height) as i32,
                &self.cells[start..end],
                large,
                DrawOperation::Set,
            );
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
    pub(crate) delay_remaining_ms: u32,
    pub(crate) waiting_for_key: bool,
    pub(crate) random_seed: i32,
    pub(crate) calendar: [u8; 8],
    pub(crate) command_line: Vec<u8>,
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
            delay_remaining_ms: 0,
            waiting_for_key: false,
            random_seed: 1,
            calendar: [0xd0, 0x07, 1, 1, 0, 0, 0, 6],
            command_line: Vec::new(),
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
        self.delay_remaining_ms = 0;
        self.waiting_for_key = false;
        self.random_seed = 1;
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
        self.advance_frame_clock();
        if !self.vm.is_running() {
            return Ok(self.frame_result(FrameStatus::Halted(self.vm.exit_code()), 0, 0));
        }
        if self.waiting_for_key {
            if let Some(value) = self.pending_input_value() {
                self.vm.push_value(value)?;
                self.waiting_for_key = false;
            } else {
                return Ok(self.frame_result(FrameStatus::WaitingForInput, 0, 0));
            }
        }
        if self.delay_remaining_ms != 0 {
            let elapsed = self.frame_duration_ms();
            self.delay_remaining_ms = self.delay_remaining_ms.saturating_sub(elapsed);
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
            match self.vm.run(slice)? {
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
        self.time_remainder += 1_000;
        self.elapsed_ms += u64::from(self.time_remainder / 60);
        self.time_remainder %= 60;
    }

    fn frame_duration_ms(&self) -> u32 {
        16 + u32::from(self.time_remainder >= 40)
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
