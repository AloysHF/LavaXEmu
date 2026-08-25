use crate::{AddressWidth, Error, Program, Result};

pub const GUEST_MEMORY_SIZE: usize = 0x0100_0000;
pub const EVALUATION_STACK_ADDRESS: usize = 0x1b00;
const EVALUATION_STACK_SIZE: usize = 0x100;
const STRING_STACK_ADDRESS: u32 = 0x1c00;
const STRING_STACK_SIZE: u32 = 0x300;
const MAX_SYSTEM_OPCODE: u8 = 0xd6;
const LAVA_TRUE: i32 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Continue,
    SystemCall(u8),
    Halted(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    BudgetExhausted,
    SystemCall(u8),
    Halted(i32),
}

#[derive(Debug, Clone)]
pub struct Vm {
    program: Program,
    memory: Vec<u8>,
    pc: usize,
    evaluation_top: usize,
    local_sp: u32,
    local_bp: u32,
    result: i32,
    exit_code: i32,
    secret: u8,
    string_ptr: u32,
    current_line: Option<u32>,
    current_function: u32,
    function_stack: Vec<u32>,
    running: bool,
    instructions_executed: u64,
}

impl Vm {
    pub fn new(program: Program) -> Self {
        Self {
            program,
            memory: vec![0; GUEST_MEMORY_SIZE],
            pc: crate::LAV_HEADER_SIZE,
            evaluation_top: 0,
            local_sp: 0,
            local_bp: 0,
            result: 0,
            exit_code: 0,
            secret: 0,
            string_ptr: STRING_STACK_ADDRESS,
            current_line: None,
            current_function: 0,
            function_stack: Vec::new(),
            running: true,
            instructions_executed: 0,
        }
    }

    pub fn reset(&mut self) {
        self.memory.fill(0);
        self.pc = crate::LAV_HEADER_SIZE;
        self.evaluation_top = 0;
        self.local_sp = 0;
        self.local_bp = 0;
        self.result = 0;
        self.exit_code = 0;
        self.secret = 0;
        self.string_ptr = STRING_STACK_ADDRESS;
        self.current_line = None;
        self.current_function = 0;
        self.function_stack.clear();
        self.running = true;
        self.instructions_executed = 0;
    }

    pub const fn program(&self) -> &Program {
        &self.program
    }

    pub fn memory(&self) -> &[u8] {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut [u8] {
        &mut self.memory
    }

    pub const fn pc(&self) -> usize {
        self.pc
    }

    pub const fn local_sp(&self) -> u32 {
        self.local_sp
    }

    pub const fn local_bp(&self) -> u32 {
        self.local_bp
    }

    pub const fn result(&self) -> i32 {
        self.result
    }

    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub const fn current_line(&self) -> Option<u32> {
        self.current_line
    }

    pub const fn current_function(&self) -> u32 {
        self.current_function
    }

    pub const fn is_running(&self) -> bool {
        self.running
    }

    pub fn halt(&mut self, exit_code: i32) {
        self.exit_code = exit_code;
        self.running = false;
    }

    pub const fn instructions_executed(&self) -> u64 {
        self.instructions_executed
    }

    pub const fn stack_depth(&self) -> usize {
        self.evaluation_top / 4
    }

    pub fn run(&mut self, instruction_budget: usize) -> Result<RunOutcome> {
        for _ in 0..instruction_budget {
            match self.step()? {
                StepOutcome::Continue => {}
                StepOutcome::SystemCall(call) => return Ok(RunOutcome::SystemCall(call)),
                StepOutcome::Halted(code) => return Ok(RunOutcome::Halted(code)),
            }
        }
        Ok(RunOutcome::BudgetExhausted)
    }

    pub fn step(&mut self) -> Result<StepOutcome> {
        if !self.running {
            return Ok(StepOutcome::Halted(self.exit_code));
        }

        let opcode_pc = self.pc;
        let opcode = self.fetch_u8()?;
        self.instructions_executed = self.instructions_executed.wrapping_add(1);

        if opcode & 0x80 != 0 {
            if opcode > MAX_SYSTEM_OPCODE {
                return Err(Error::InvalidOpcode {
                    opcode,
                    pc: opcode_pc,
                });
            }
            return Ok(StepOutcome::SystemCall(opcode & 0x7f));
        }

        match opcode {
            0x00 | 0x75..=0x7f => {
                return Err(Error::InvalidOpcode {
                    opcode,
                    pc: opcode_pc,
                });
            }
            0x01 => {
                let value = i32::from(self.fetch_u8()?);
                self.push_value(value)?;
            }
            0x02 => {
                let value = i32::from(self.fetch_i16()?);
                self.push_value(value)?;
            }
            0x03 => {
                let value = self.fetch_i32()?;
                self.push_value(value)?;
            }
            0x04..=0x0c => self.execute_global_push(opcode)?,
            0x0d => self.push_string()?,
            0x0e..=0x19 => self.execute_local_push(opcode)?,
            0x1a => return Err(Error::UnsupportedFeature("_TEXT")),
            0x1b => return Err(Error::UnsupportedFeature("_GRAPH")),
            0x1c => {
                let value = self.pop_value()?.wrapping_neg();
                self.push_value(value)?;
            }
            0x1d..=0x20 => self.execute_increment(opcode)?,
            0x21..=0x34 => self.execute_integer_operation(opcode, opcode_pc)?,
            0x35 => self.assign_descriptor()?,
            0x36 => {
                let pointer = self.pop_value()? as u32;
                let address = self.mask_address(pointer);
                let value = i32::from(self.read_u8(address)?);
                self.push_value(value)?;
            }
            0x37 => {
                let pointer = self.pop_value()? as u32;
                let value = self.pointer_descriptor(pointer, 1);
                self.push_value(value as i32)?;
            }
            0x38 => self.result = self.pop_value()?,
            0x39 => self.conditional_jump(self.result == 0)?,
            0x3a => self.conditional_jump(self.result != 0)?,
            0x3b => {
                let target = self.fetch_u24()? as usize;
                self.jump(target)?;
            }
            0x3c => self.local_sp = self.fetch_address()?,
            0x3d => self.call()?,
            0x3e => self.enter_function()?,
            0x3f => self.leave_function()?,
            0x40 => {
                self.exit_code = 0;
                self.running = false;
                return Ok(StepOutcome::Halted(0));
            }
            0x41 => self.preset_memory()?,
            0x42 => return Err(Error::UnsupportedFeature("_GBUF")),
            0x43 => self.secret = self.fetch_u8()?,
            0x44 => {}
            0x45..=0x51 => self.execute_quick_operation(opcode, opcode_pc)?,
            0x52 => {
                let pointer = self.pop_value()? as u32;
                let address = self.mask_address(pointer);
                let value = i32::from(self.read_i16(address)?);
                self.push_value(value)?;
            }
            0x53 => {
                let pointer = self.pop_value()? as u32;
                let address = self.mask_address(pointer);
                let value = self.read_i32(address)?;
                self.push_value(value)?;
            }
            0x54..=0x69 => self.execute_float_operation(opcode, opcode_pc)?,
            0x6a => {
                let pointer = self.pop_value()? as u32;
                let value = self.pointer_descriptor(pointer, 2);
                self.push_value(value as i32)?;
            }
            0x6b => {
                let pointer = self.pop_value()? as u32;
                let value = self.pointer_descriptor(pointer, 4);
                self.push_value(value as i32)?;
            }
            0x6c => {
                let value = self.pop_value()? & 0xff;
                self.push_value(value)?;
            }
            0x6d => {
                let value = i32::from(self.pop_value()? as i16);
                self.push_value(value)?;
            }
            0x6e => self.assign_indirect()?,
            0x6f => {
                let address = self.indexed_address(false)?;
                self.push_value(address as i32)?;
            }
            0x70 => self.execute_indexed_increment()?,
            0x71 => {
                self.fetch_u8()?;
            }
            0x72 => {}
            0x73 => self.current_line = Some(self.fetch_u24()?),
            0x74 => {
                self.function_stack.push(self.current_function);
                self.current_function = self.fetch_u24()?;
            }
            _ => unreachable!("all byte opcodes are covered"),
        }

        Ok(StepOutcome::Continue)
    }

    pub fn pop_value(&mut self) -> Result<i32> {
        if self.evaluation_top < 4 {
            return Err(Error::StackUnderflow);
        }
        self.evaluation_top -= 4;
        let start = EVALUATION_STACK_ADDRESS + self.evaluation_top;
        Ok(i32::from_le_bytes(
            self.memory[start..start + 4]
                .try_into()
                .expect("evaluation slot has a fixed size"),
        ))
    }

    pub fn push_value(&mut self, value: i32) -> Result<()> {
        if self.evaluation_top + 4 > EVALUATION_STACK_SIZE {
            return Err(Error::StackOverflow);
        }
        let start = EVALUATION_STACK_ADDRESS + self.evaluation_top;
        self.memory[start..start + 4].copy_from_slice(&value.to_le_bytes());
        self.evaluation_top += 4;
        Ok(())
    }

    fn execute_global_push(&mut self, opcode: u8) -> Result<()> {
        let indexed = opcode >= 0x07;
        let address = if indexed {
            self.indexed_address(false)?
        } else {
            self.fetch_address()?
        };
        match opcode {
            0x04 | 0x07 => {
                let value = i32::from(self.read_u8(address)?);
                self.push_value(value)
            }
            0x05 | 0x08 => {
                let value = i32::from(self.read_i16(address)?);
                self.push_value(value)
            }
            0x06 | 0x09 => {
                let value = self.read_i32(address)?;
                self.push_value(value)
            }
            0x0a => self.push_value(self.pointer_descriptor(address, 1) as i32),
            0x0b => self.push_value(self.pointer_descriptor(address, 2) as i32),
            0x0c => self.push_value(self.pointer_descriptor(address, 0) as i32),
            _ => unreachable!(),
        }
    }

    fn execute_local_push(&mut self, opcode: u8) -> Result<()> {
        let address = match opcode {
            0x0e..=0x10 | 0x19 => self.fetch_address()?.wrapping_add(self.local_bp),
            0x17 => self.indexed_address(false)?,
            _ => self.indexed_address(true)?,
        };
        match opcode {
            0x0e | 0x11 => {
                let value = i32::from(self.read_u8(address)?);
                self.push_value(value)
            }
            0x0f | 0x12 => {
                let value = i32::from(self.read_i16(address)?);
                self.push_value(value)
            }
            0x10 | 0x13 => {
                let value = self.read_i32(address)?;
                self.push_value(value)
            }
            0x14 => self.push_value(self.pointer_descriptor(address, 1) as i32),
            0x15 => self.push_value(self.pointer_descriptor(address, 2) as i32),
            0x16..=0x19 => self.push_value(self.pointer_descriptor(address, 0) as i32),
            _ => unreachable!(),
        }
    }

    fn execute_increment(&mut self, opcode: u8) -> Result<()> {
        let descriptor = self.pop_value()? as u32;
        let (address, size) = self.decode_descriptor(descriptor);
        let original = self.read_sized(address, size)?;
        let updated = if opcode == 0x1d || opcode == 0x1f {
            original.wrapping_add(1)
        } else {
            original.wrapping_sub(1)
        };
        let pushed = if opcode == 0x1d || opcode == 0x1e {
            updated
        } else {
            original
        };
        self.push_value(pushed)?;
        self.write_sized(address, size, updated)
    }

    fn execute_integer_operation(&mut self, opcode: u8, opcode_pc: usize) -> Result<()> {
        if opcode == 0x25 || opcode == 0x29 {
            let value = self.pop_value()?;
            let result = if opcode == 0x25 {
                !value
            } else {
                bool_value(value == 0)
            };
            return self.push_value(result);
        }

        let rhs = self.pop_value()?;
        let lhs = self.pop_value()?;
        let value = match opcode {
            0x21 => lhs.wrapping_add(rhs),
            0x22 => lhs.wrapping_sub(rhs),
            0x23 => lhs & rhs,
            0x24 => lhs | rhs,
            0x26 => lhs ^ rhs,
            0x27 => bool_value(lhs != 0 && rhs != 0),
            0x28 => bool_value(lhs != 0 || rhs != 0),
            0x2a => lhs.wrapping_mul(rhs),
            0x2b => checked_div(lhs, rhs, opcode_pc)?,
            0x2c => checked_rem(lhs, rhs, opcode_pc)?,
            0x2d => {
                if rhs < 0 {
                    0
                } else {
                    lhs.wrapping_shl(rhs as u32)
                }
            }
            0x2e => {
                if rhs < 0 {
                    0
                } else {
                    ((lhs as u32).wrapping_shr(rhs as u32)) as i32
                }
            }
            0x2f => bool_value(lhs == rhs),
            0x30 => bool_value(lhs != rhs),
            0x31 => bool_value(lhs <= rhs),
            0x32 => bool_value(lhs >= rhs),
            0x33 => bool_value(lhs > rhs),
            0x34 => bool_value(lhs < rhs),
            _ => unreachable!(),
        };
        self.push_value(value)
    }

    fn assign_descriptor(&mut self) -> Result<()> {
        let value = self.pop_value()?;
        let descriptor = self.pop_value()? as u32;
        let (address, size) = self.decode_descriptor(descriptor);
        self.write_sized(address, size, value)?;
        self.push_value(value)
    }

    fn assign_indirect(&mut self) -> Result<()> {
        let value = self.pop_value()?;
        let mut address = self.pop_value()? as u32;
        let kind = self.fetch_u8()?;
        if kind & 0x80 != 0 {
            address = address.wrapping_add(self.local_bp);
        }
        self.write_sized(address, kind & 0x7f, value)?;
        self.push_value(value)
    }

    fn conditional_jump(&mut self, take: bool) -> Result<()> {
        let target = self.fetch_u24()? as usize;
        if take {
            self.jump(target)?;
        }
        Ok(())
    }

    fn call(&mut self) -> Result<()> {
        let return_address = self
            .pc
            .checked_add(3)
            .ok_or(Error::UnexpectedEnd { pc: self.pc })?;
        self.write_u24(self.local_sp, return_address as u32)?;
        let target = self.fetch_u24()? as usize;
        self.jump(target)
    }

    fn enter_function(&mut self) -> Result<()> {
        let previous_bp = self.local_bp;
        self.local_bp = self.local_sp;
        match self.address_width() {
            AddressWidth::Bits16 => self.write_u16(self.local_bp + 3, previous_bp as u16)?,
            AddressWidth::Bits24 | AddressWidth::Bits32 => {
                self.write_u24(self.local_bp + 3, previous_bp)?;
            }
        }
        let frame_size = self.fetch_address()?;
        self.local_sp = self.local_bp.wrapping_add(frame_size & 0x00ff_ffff);
        let argument_bytes = usize::from(self.fetch_u8()?) * 4;
        if argument_bytes > self.evaluation_top {
            return Err(Error::StackUnderflow);
        }
        self.evaluation_top -= argument_bytes;
        let source = EVALUATION_STACK_ADDRESS + self.evaluation_top;
        let destination = self.local_bp
            + match self.address_width() {
                AddressWidth::Bits32 => 8,
                AddressWidth::Bits24 => 6,
                AddressWidth::Bits16 => 5,
            };
        let bytes = self.memory[source..source + argument_bytes].to_vec();
        self.write_bytes(destination, &bytes)
    }

    fn leave_function(&mut self) -> Result<()> {
        self.local_sp = self.local_bp;
        let target = self.read_u24(self.local_bp)? as usize;
        let previous_bp = match self.address_width() {
            AddressWidth::Bits16 => u32::from(self.read_u16(self.local_bp + 3)?),
            AddressWidth::Bits24 | AddressWidth::Bits32 => self.read_u24(self.local_bp + 3)?,
        };
        self.local_bp = previous_bp & 0x00ff_ffff;
        self.jump(target)?;
        if let Some(function) = self.function_stack.pop() {
            self.current_function = function;
        }
        Ok(())
    }

    fn preset_memory(&mut self) -> Result<()> {
        let address = self.fetch_address()?;
        let length = usize::from(self.fetch_u16()?);
        let end = self
            .pc
            .checked_add(length)
            .ok_or(Error::UnexpectedEnd { pc: self.pc })?;
        let bytes = self
            .program
            .image()
            .get(self.pc..end)
            .ok_or(Error::UnexpectedEnd { pc: self.pc })?
            .to_vec();
        self.pc = end;
        self.write_bytes(address, &bytes)
    }

    fn execute_quick_operation(&mut self, opcode: u8, opcode_pc: usize) -> Result<()> {
        let lhs = self.pop_value()?;
        let rhs = i32::from(self.fetch_i16()?);
        let value = match opcode {
            0x45 => lhs.wrapping_add(rhs),
            0x46 => lhs.wrapping_sub(rhs),
            0x47 => lhs.wrapping_mul(rhs),
            0x48 => checked_div(lhs, rhs, opcode_pc)?,
            0x49 => checked_rem(lhs, rhs, opcode_pc)?,
            0x4a => lhs.wrapping_shl(rhs as u32),
            0x4b => (lhs as u32).wrapping_shr(rhs as u32) as i32,
            0x4c => bool_value(lhs == rhs),
            0x4d => bool_value(lhs != rhs),
            0x4e => bool_value(lhs > rhs),
            0x4f => bool_value(lhs < rhs),
            0x50 => bool_value(lhs >= rhs),
            0x51 => bool_value(lhs <= rhs),
            _ => unreachable!(),
        };
        self.push_value(value)
    }

    fn execute_float_operation(&mut self, opcode: u8, opcode_pc: usize) -> Result<()> {
        if matches!(opcode, 0x54 | 0x55 | 0x62 | 0x69) {
            let value = self.pop_value()?;
            let converted = match opcode {
                0x54 => (value as f32).to_bits() as i32,
                0x55 => f32::from_bits(value as u32) as i32,
                0x62 => (-f32::from_bits(value as u32)).to_bits() as i32,
                0x69 => value & 0x7fff_ffff,
                _ => unreachable!(),
            };
            return self.push_value(converted);
        }

        let rhs = self.pop_value()?;
        let lhs = self.pop_value()?;
        let lhs_float = f32::from_bits(lhs as u32);
        let rhs_float = f32::from_bits(rhs as u32);
        let value = match opcode {
            0x56 => (lhs_float + rhs_float).to_bits() as i32,
            0x57 => (lhs_float + rhs as f32).to_bits() as i32,
            0x58 => (lhs as f32 + rhs_float).to_bits() as i32,
            0x59 => (lhs_float - rhs_float).to_bits() as i32,
            0x5a => (lhs_float - rhs as f32).to_bits() as i32,
            0x5b => (lhs as f32 - rhs_float).to_bits() as i32,
            0x5c => (lhs_float * rhs_float).to_bits() as i32,
            0x5d => (lhs_float * rhs as f32).to_bits() as i32,
            0x5e => (lhs as f32 * rhs_float).to_bits() as i32,
            0x5f => {
                if rhs == 0 {
                    return Err(Error::DivisionByZero { pc: opcode_pc });
                }
                (lhs_float / rhs_float).to_bits() as i32
            }
            0x60 => {
                if rhs == 0 {
                    return Err(Error::DivisionByZero { pc: opcode_pc });
                }
                (lhs_float / rhs as f32).to_bits() as i32
            }
            0x61 => {
                if rhs == 0 {
                    return Err(Error::DivisionByZero { pc: opcode_pc });
                }
                (lhs as f32 / rhs_float).to_bits() as i32
            }
            0x63 => bool_value(lhs_float < rhs_float),
            0x64 => bool_value(lhs_float > rhs_float),
            0x65 => bool_value(lhs_float == rhs_float),
            0x66 => bool_value(lhs_float != rhs_float),
            0x67 => bool_value(lhs_float <= rhs_float),
            0x68 => bool_value(lhs_float >= rhs_float),
            _ => unreachable!(),
        };
        self.push_value(value)
    }

    fn execute_indexed_increment(&mut self) -> Result<()> {
        let mut address = self.pop_value()? as u32;
        let mode = self.fetch_u8()?;
        if mode & 0x80 != 0 {
            address = address.wrapping_add(self.local_bp);
        }
        let size = mode & 0x1f;
        let original = self.read_sized(address, size)?;
        let operation = (mode >> 5) & 3;
        let updated = if operation == 0 || operation == 2 {
            original.wrapping_add(1)
        } else {
            original.wrapping_sub(1)
        };
        let pushed = if operation < 2 { updated } else { original };
        self.push_value(pushed)?;
        self.write_sized(address, size, updated)
    }

    fn push_string(&mut self) -> Result<()> {
        self.push_value(self.string_ptr as i32)?;
        loop {
            let value = self.fetch_u8()? ^ self.secret;
            self.write_u8(self.string_ptr, value)?;
            self.string_ptr = self.string_ptr.wrapping_add(1);
            if value == 0 {
                break;
            }
        }
        if self.string_ptr >= STRING_STACK_ADDRESS + STRING_STACK_SIZE {
            self.string_ptr = STRING_STACK_ADDRESS;
        }
        Ok(())
    }

    fn indexed_address(&mut self, local: bool) -> Result<u32> {
        let mut address = self.fetch_address()?;
        if local {
            address = address.wrapping_add(self.local_bp);
        }
        let index = self.pop_value()? as u32;
        Ok(address.wrapping_add(index & self.address_mask()))
    }

    fn pointer_descriptor(&self, address: u32, size: u8) -> u32 {
        match self.address_width() {
            AddressWidth::Bits16 => (address & 0xffff) | (u32::from(size) << 16),
            AddressWidth::Bits24 | AddressWidth::Bits32 => {
                (address & 0x00ff_ffff) | (u32::from(size) << 24)
            }
        }
    }

    fn decode_descriptor(&self, descriptor: u32) -> (u32, u8) {
        match self.address_width() {
            AddressWidth::Bits16 => {
                let adjusted = if descriptor & 0x0080_0000 != 0 {
                    descriptor.wrapping_add(self.local_bp)
                } else {
                    descriptor
                };
                (adjusted & 0xffff, ((descriptor >> 16) & 0x7f) as u8)
            }
            AddressWidth::Bits24 | AddressWidth::Bits32 => {
                let adjusted = if descriptor & 0x8000_0000 != 0 {
                    descriptor.wrapping_add(self.local_bp)
                } else {
                    descriptor
                };
                (adjusted & 0x00ff_ffff, ((descriptor >> 24) & 0x7f) as u8)
            }
        }
    }

    fn address_width(&self) -> AddressWidth {
        self.program.header().address_width
    }

    fn address_mask(&self) -> u32 {
        match self.address_width() {
            AddressWidth::Bits16 => 0xffff,
            AddressWidth::Bits24 | AddressWidth::Bits32 => 0x00ff_ffff,
        }
    }

    fn mask_address(&self, address: u32) -> u32 {
        address & self.address_mask()
    }

    fn fetch_address(&mut self) -> Result<u32> {
        let low = u32::from(self.fetch_u16()?);
        if self.address_width() == AddressWidth::Bits16 {
            Ok(low)
        } else {
            Ok(low | (u32::from(self.fetch_u8()?) << 16))
        }
    }

    fn fetch_u8(&mut self) -> Result<u8> {
        let value = self
            .program
            .image()
            .get(self.pc)
            .copied()
            .ok_or(Error::UnexpectedEnd { pc: self.pc })?;
        self.pc += 1;
        Ok(value)
    }

    fn fetch_u16(&mut self) -> Result<u16> {
        let low = self.fetch_u8()?;
        let high = self.fetch_u8()?;
        Ok(u16::from_le_bytes([low, high]))
    }

    fn fetch_i16(&mut self) -> Result<i16> {
        Ok(self.fetch_u16()? as i16)
    }

    fn fetch_u24(&mut self) -> Result<u32> {
        let low = u32::from(self.fetch_u16()?);
        Ok(low | (u32::from(self.fetch_u8()?) << 16))
    }

    fn fetch_i32(&mut self) -> Result<i32> {
        let low = self.fetch_u16()?;
        let high = self.fetch_u16()?;
        Ok(i32::from_le_bytes([
            low as u8,
            (low >> 8) as u8,
            high as u8,
            (high >> 8) as u8,
        ]))
    }

    fn jump(&mut self, target: usize) -> Result<()> {
        if target >= self.program.image().len() {
            return Err(Error::InvalidJump { target });
        }
        self.pc = target;
        Ok(())
    }

    fn checked_range(&self, address: u32, length: usize) -> Result<std::ops::Range<usize>> {
        let start = address as usize;
        let end = start.checked_add(length).ok_or(Error::MemoryOutOfBounds {
            address,
            end: u32::MAX,
        })?;
        if end > self.memory.len() {
            return Err(Error::MemoryOutOfBounds {
                address,
                end: end.saturating_sub(1).min(u32::MAX as usize) as u32,
            });
        }
        Ok(start..end)
    }

    fn read_u8(&self, address: u32) -> Result<u8> {
        let range = self.checked_range(address, 1)?;
        Ok(self.memory[range.start])
    }

    fn read_u16(&self, address: u32) -> Result<u16> {
        let range = self.checked_range(address, 2)?;
        Ok(u16::from_le_bytes(
            self.memory[range]
                .try_into()
                .expect("checked two-byte range"),
        ))
    }

    fn read_i16(&self, address: u32) -> Result<i16> {
        Ok(self.read_u16(address)? as i16)
    }

    fn read_u24(&self, address: u32) -> Result<u32> {
        let range = self.checked_range(address, 3)?;
        let bytes = &self.memory[range];
        Ok(u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16))
    }

    fn read_i32(&self, address: u32) -> Result<i32> {
        let range = self.checked_range(address, 4)?;
        Ok(i32::from_le_bytes(
            self.memory[range]
                .try_into()
                .expect("checked four-byte range"),
        ))
    }

    fn read_sized(&self, address: u32, size: u8) -> Result<i32> {
        match size {
            1 => Ok(i32::from(self.read_u8(address)?)),
            2 => Ok(i32::from(self.read_i16(address)?)),
            _ => self.read_i32(address),
        }
    }

    fn write_u8(&mut self, address: u32, value: u8) -> Result<()> {
        let range = self.checked_range(address, 1)?;
        self.memory[range.start] = value;
        Ok(())
    }

    fn write_u16(&mut self, address: u32, value: u16) -> Result<()> {
        self.write_bytes(address, &value.to_le_bytes())
    }

    fn write_u24(&mut self, address: u32, value: u32) -> Result<()> {
        self.write_bytes(
            address,
            &[value as u8, (value >> 8) as u8, (value >> 16) as u8],
        )
    }

    fn write_i32(&mut self, address: u32, value: i32) -> Result<()> {
        self.write_bytes(address, &value.to_le_bytes())
    }

    fn write_sized(&mut self, address: u32, size: u8, value: i32) -> Result<()> {
        match size {
            1 => self.write_u8(address, value as u8),
            2 => self.write_u16(address, value as u16),
            _ => self.write_i32(address, value),
        }
    }

    fn write_bytes(&mut self, address: u32, bytes: &[u8]) -> Result<()> {
        let range = self.checked_range(address, bytes.len())?;
        self.memory[range].copy_from_slice(bytes);
        Ok(())
    }
}

fn bool_value(value: bool) -> i32 {
    if value { LAVA_TRUE } else { 0 }
}

fn checked_div(lhs: i32, rhs: i32, pc: usize) -> Result<i32> {
    if rhs == 0 {
        Err(Error::DivisionByZero { pc })
    } else if lhs == i32::MIN && rhs == -1 {
        Ok(i32::MIN)
    } else {
        Ok(lhs / rhs)
    }
}

fn checked_rem(lhs: i32, rhs: i32, pc: usize) -> Result<i32> {
    if rhs == 0 {
        Err(Error::DivisionByZero { pc })
    } else if lhs == i32::MIN && rhs == -1 {
        Ok(0)
    } else {
        Ok(lhs % rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LAV_MAGIC;

    fn vm(bytecode: &[u8]) -> Vm {
        let mut image = vec![0; crate::LAV_HEADER_SIZE];
        image[..4].copy_from_slice(&LAV_MAGIC);
        image[8] = 0xf0;
        image[9] = 15;
        image[10] = 10;
        image.extend_from_slice(bytecode);
        Vm::new(Program::load(&image).unwrap())
    }

    #[test]
    fn executes_integer_expression() {
        let mut vm = vm(&[
            0x01, 7, // push_char 7
            0x02, 0xfd, 0xff, // push_int -3
            0x21, // add
            0x38, // pop result
            0x40, // exit
        ]);
        assert_eq!(vm.run(32).unwrap(), RunOutcome::Halted(0));
        assert_eq!(vm.result(), 4);
        assert_eq!(vm.stack_depth(), 0);
    }

    #[test]
    fn assigns_and_loads_guest_memory() {
        let mut vm = vm(&[
            0x01, 0, // array index
            0x0c, 0x00, 0x20, 0x00, // address of long at 0x2000
            0x03, 42, 0, 0, 0,    // value
            0x35, // assignment
            0x38, // discard expression value
            0x06, 0x00, 0x20, 0x00, // load long
            0x38, // save result
            0x40,
        ]);
        assert_eq!(vm.run(64).unwrap(), RunOutcome::Halted(0));
        assert_eq!(vm.result(), 42);
        assert_eq!(&vm.memory()[0x2000..0x2004], &42_i32.to_le_bytes());
    }

    #[test]
    fn yields_system_calls_to_the_host() {
        let mut vm = vm(&[0x01, b'A', 0x80, 0x40]);
        assert_eq!(vm.run(16).unwrap(), RunOutcome::SystemCall(0));
        assert_eq!(vm.pop_value().unwrap(), i32::from(b'A'));
        assert_eq!(vm.run(16).unwrap(), RunOutcome::Halted(0));
    }

    #[test]
    fn calls_and_returns_from_a_function() {
        let mut vm = vm(&[
            0x3c, 0x00, 0x20, 0x00, // stack starts at 0x2000
            0x3d, 0x1a, 0x00, 0x00, // call absolute offset 0x1a
            0x38, // store returned expression
            0x40, // exit
            0x3e, 0x20, 0x00, 0x00, 0, // enter a frame with no arguments
            0x01, 9,    // return expression
            0x3f, // leave function
        ]);
        assert_eq!(vm.run(32).unwrap(), RunOutcome::Halted(0));
        assert_eq!(vm.result(), 9);
        assert_eq!(vm.local_bp(), 0);
        assert_eq!(vm.local_sp(), 0x2000);
    }

    #[test]
    fn copies_preset_data_into_memory() {
        let mut vm = vm(&[0x41, 0x00, 0x20, 0x00, 3, 0, 0xaa, 0xbb, 0xcc, 0x40]);
        assert_eq!(vm.run(16).unwrap(), RunOutcome::Halted(0));
        assert_eq!(&vm.memory()[0x2000..0x2003], &[0xaa, 0xbb, 0xcc]);
    }

    #[test]
    fn reports_division_by_zero() {
        let mut vm = vm(&[0x01, 5, 0x01, 0, 0x2b]);
        assert!(matches!(vm.run(16), Err(Error::DivisionByZero { .. })));
    }
}
