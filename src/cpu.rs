use std::collections::HashMap;
use crate::{opcodes, bus::Bus, trace};


const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

pub enum StatusFlag {
    Carry = (1 << 0),
    Zero = (1 << 1),
    InterruptDisable = (1 << 2),
    DecimalMode = (1 << 3),
    Break = (1 << 4),
    Break2 = (1 << 5),
    Overflow = (1 << 6),
    Negative = (1 << 7),
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

pub trait Mem {
    fn mem_read(&mut self, addr: u16) -> u8; 
    fn mem_write(&mut self, addr: u16, data: u8);
    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo: u16 = self.mem_read(pos) as u16;
        let hi: u16 = self.mem_read(pos + 1) as u16;
        (hi << 8) | (lo as u16)
    }
    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi: u8 = (data >> 8) as u8;
        let lo: u8 = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
}

trait Stack {
    fn stack_pop(&mut self) -> u8;
    fn stack_push(&mut self, data: u8);
    fn stack_push_u16(&mut self, data: u16);
    fn stack_pop_u16(&mut self) -> u16;
}

trait FlagArithmetic {
    fn get_flag(&self, flag: StatusFlag) -> bool;
    fn set_flag(&mut self, flag: StatusFlag, value: bool);
    fn update_zero_and_negative_flags(&mut self, result: u8);
}

trait Instructions {
    fn lda(&mut self, mode: &AddressingMode);
    fn sta(&mut self, mode: &AddressingMode);
    fn adc(&mut self, mode: &AddressingMode);
    fn and(&mut self, mode: &AddressingMode);
    fn asl(&mut self, mode: &AddressingMode) -> u8;
    fn asl_accumulator(&mut self);
    fn cmp(&mut self, mode: &AddressingMode);
    fn cpx(&mut self, mode: &AddressingMode);
    fn cpy(&mut self, mode: &AddressingMode);
    fn dec(&mut self, mode: &AddressingMode) -> u8;
    fn dex(&mut self);
    fn dey(&mut self);
    fn eor(&mut self, mode: &AddressingMode);
    fn inc(&mut self, mode: &AddressingMode) -> u8;
    fn inx(&mut self);
    fn iny(&mut self);
    fn ldx(&mut self, mode: &AddressingMode);
    fn ldy(&mut self, mode: &AddressingMode);
    fn lsr(&mut self, mode: &AddressingMode) -> u8;
    fn lsr_accumulator(&mut self);
    fn ora(&mut self, mode: &AddressingMode);
    fn rol(&mut self, mode: &AddressingMode) -> u8;
    fn rol_accumulator(&mut self);
    fn ror(&mut self, mode: &AddressingMode) -> u8;
    fn ror_accumulator(&mut self);
    fn sbc(&mut self, mode: &AddressingMode);
    fn stx(&mut self, mode: &AddressingMode);
    fn sty(&mut self, mode: &AddressingMode);
    fn tax(&mut self);
    fn tay(&mut self);
    fn tsx(&mut self);
    fn txa(&mut self);
    fn txs(&mut self);
    fn tya(&mut self);
    // Unofficial opcodes
    fn lax(&mut self, mode: &AddressingMode);
    fn sax(&mut self, mode: &AddressingMode);
    fn dcp(&mut self, mode: &AddressingMode);
    fn isb(&mut self, mode: &AddressingMode);
    fn slo(&mut self, mode: &AddressingMode);
    fn rla(&mut self, mode: &AddressingMode);
    fn sre(&mut self, mode: &AddressingMode);
    fn rra(&mut self, mode: &AddressingMode);
    fn brk(&mut self);
}

pub struct CPU<'a> {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub bus: Bus<'a>,
    // memory: [u8; 0xFFFF],
}

impl Mem for CPU<'_> {
    fn mem_read(&mut self, addr: u16) -> u8 { self.bus.mem_read(addr) }
    fn mem_write(&mut self, addr: u16, data: u8) { self.bus.mem_write(addr, data); /*self.memory[addr as usize] = data;*/ }
    fn mem_read_u16(&mut self, pos: u16) -> u16 { self.bus.mem_read_u16(pos) }
    fn mem_write_u16(&mut self, pos: u16, data: u16) { self.bus.mem_write_u16(pos, data); }
}
impl Stack for CPU<'_> {
    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read((STACK as u16) + self.stack_pointer as u16)
    }
    fn stack_push(&mut self, data: u8) {
        self.mem_write((STACK as u16) + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1)
    }
    fn stack_push_u16(&mut self, data: u16) {
        let hi: u8 = (data >> 8) as u8;
        let lo: u8 = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }
    fn stack_pop_u16(&mut self) -> u16 {
        let lo: u16 = self.stack_pop() as u16;
        let hi: u16 = self.stack_pop() as u16;
        hi << 8 | lo
    }
}

impl FlagArithmetic for CPU<'_> {
    fn get_flag(&self, flag: StatusFlag) -> bool { (self.status & (flag as u8)) > 0 }
    fn set_flag(&mut self, flag: StatusFlag, value: bool) {
        if value { self.status |= flag as u8; }
        else { self.status &= !(flag as u8); }
    }
    fn update_zero_and_negative_flags(&mut self, result: u8) {
        self.set_flag(StatusFlag::Zero, result == 0);
        self.set_flag(StatusFlag::Negative, (result & 0b1000_0000) != 0);
    }
}

impl Instructions for CPU<'_> {
    fn lda(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode); // Get the operand to read based on the addressing mode
        let value: u8 = self.mem_read(addr); // Read the value from memory
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn sta(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode); // Get the operand to write to based on the addressing mode
        self.mem_write(addr, self.register_a); // Write the value to memory
    }
    fn adc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode); // Get the operand to read based on the addressing mode
        let value: u8 = self.mem_read(addr); // Read the value from memory
        self.add_to_register_a(value);
    }
    fn and(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode); // Get the operand to read based on the addressing mode
        let value: u8 = self.mem_read(addr); // Read the value from memory
        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn asl_accumulator(&mut self) {
        let mut data: u8 = self.register_a;
        if data >> 7 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        data = data << 1;
        self.register_a = data;
        self.update_zero_and_negative_flags(data);
    }
    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let addr: u16 = self.get_operand_address(mode);
        let mut data: u8 = self.mem_read(addr);
        if data >> 7 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        data = data << 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }
    fn cmp(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode); // Get the operand to read based on the addressing mode
        let value: u8 = self.mem_read(addr); // Read the value from memory
        if self.register_a >= value { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        self.update_zero_and_negative_flags(self.register_a.wrapping_sub(value));
    }
    fn cpx(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode); // Get the operand to read based on the addressing mode
        let value: u8 = self.mem_read(addr); // Read the value from memory
        if self.register_x >= value { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        self.update_zero_and_negative_flags(self.register_x.wrapping_sub(value));
    }
    fn cpy(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode); // Get the operand to read based on the addressing mode
        let value: u8 = self.mem_read(addr); // Read the value from memory
        if self.register_y >= value { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        self.update_zero_and_negative_flags(self.register_y.wrapping_sub(value));
    }
    fn dec(&mut self, mode: &AddressingMode) -> u8 {
        let addr: u16 = self.get_operand_address(mode);
        let value: u8 = self.mem_read(addr);
        let result: u8 = value.wrapping_sub(1);
        self.mem_write(addr, result);
        self.update_zero_and_negative_flags(result);
        result
    }
    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }
    fn eor(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let value: u8 = self.mem_read(addr);
        self.register_a ^= value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn inc(&mut self, mode: &AddressingMode) -> u8 {
        let addr: u16 = self.get_operand_address(mode);
        let value: u8 = self.mem_read(addr);
        let result: u8 = value.wrapping_add(1);
        self.mem_write(addr, result);
        self.update_zero_and_negative_flags(result);
        result
    }
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }
    fn ldx(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let value: u8 = self.mem_read(addr);
        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn ldy(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let value: u8 = self.mem_read(addr);
        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
    }
    fn lsr_accumulator(&mut self) {
        let mut data: u8 = self.register_a;
        if data & 1 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        data = data >> 1;
        self.register_a = data;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let addr: u16 = self.get_operand_address(mode);
        let mut data: u8 = self.mem_read(addr);
        if data & 1 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        data = data >> 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }
    fn ora(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let value: u8 = self.mem_read(addr);
        self.register_a |= value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let addr: u16 = self.get_operand_address(mode);
        let mut data: u8 = self.mem_read(addr);
        let old_carry: bool = self.get_flag(StatusFlag::Carry);
        if data >> 7 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        data = data << 1;
        if old_carry { data = data | 1; }
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }
    fn rol_accumulator(&mut self) {
        let mut data: u8 = self.register_a;
        let old_carry: bool = self.get_flag(StatusFlag::Carry);
        if data >> 7 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        data = data << 1;
        if old_carry { data = data | 1; }
        self.register_a = data;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr: u16 = self.get_operand_address(mode);
        let mut data: u8 = self.mem_read(addr);
        let old_carry: bool = self.get_flag(StatusFlag::Carry);
        if data & 1 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        data = data >> 1;
        if old_carry { data = data | 0b10000000; }
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }
    fn ror_accumulator(&mut self) {
        let mut data: u8 = self.register_a;
        let old_carry: bool = self.get_flag(StatusFlag::Carry);
        if data & 1 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        data = data >> 1;
        if old_carry { data = data | 0b10000000; }
        self.register_a = data;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn sbc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        let data: u8 = self.mem_read(addr);
        self.add_to_register_a(((data as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }
    fn stx(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        self.mem_write(addr, self.register_x);
    }
    fn sty(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        self.mem_write(addr, self.register_y);
    }
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_flags(self.register_y);
    }
    fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn txs(&mut self) { self.stack_pointer = self.register_x; }
    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_and_negative_flags(self.register_a);
    }
    // Unofficial opcodes
    fn lax(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        let value: u8 = self.mem_read(addr);
        self.register_a = value;
        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn sax(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        let value: u8 = self.register_a & self.register_x;
        self.mem_write(addr, value);
    }
    fn dcp(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        let value: u8 = self.mem_read(addr).wrapping_sub(1);
        self.mem_write(addr, value);
        if self.register_a >= value { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        self.update_zero_and_negative_flags(self.register_a.wrapping_sub(value));
    }
    fn isb(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        let value: u8 = self.mem_read(addr).wrapping_add(1);
        self.mem_write(addr, value);
        self.add_to_register_a(((value as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }
    fn slo(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        let val = self.mem_read(addr);
        if val >> 7 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        let value: u8 = val << 1;
        self.mem_write(addr, value);
        self.register_a |= value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn rla(&mut self, mode: &AddressingMode) {
        self.rol(mode);
        let addr: u16 = self.get_operand_address(&mode);
        self.register_a &= self.mem_read(addr);
    }
    fn sre(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(&mode);
        let val = self.mem_read(addr);
        if val & 1 == 1 { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        let value: u8 = val >> 1;
        self.mem_write(addr, value);
        self.register_a ^= value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn rra(&mut self, mode: &AddressingMode) {
        self.ror(mode);
        let addr: u16 = self.get_operand_address(&mode);
        let value: u8 = self.mem_read(addr);
        self.add_to_register_a(value);
    }
    fn brk(&mut self) {
        self.program_counter += 1;
        self.stack_push_u16(self.program_counter);
        self.set_flag(StatusFlag::Break, true);
        self.stack_push(self.status);
        self.program_counter = self.mem_read_u16(0xFFFE);
    }
}

mod interrupt {
    #[derive(PartialEq, Eq)]
    pub enum InterruptType { NMI }

    #[derive(PartialEq, Eq)]
    pub(super) struct Interrupt {
        pub(super) itype: InterruptType,
        pub(super) vector_addr: u16,
        pub(super) b_flag_mask: u8,
        pub(super) cpu_cycles: u8,
    }
    pub(super) const NMI: Interrupt = Interrupt {
        itype: InterruptType::NMI,
        vector_addr: 0xfffA,
        b_flag_mask: 0b00100000,
        cpu_cycles: 2,
    };
}


impl<'a> CPU<'a> {
    pub fn new<'b>(bus: Bus<'b>) -> CPU<'b> {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0b100100,
            program_counter: 0,
            stack_pointer: STACK_RESET,
            bus,
            //memory: [0; 0xFFFF],
        }
    }

    pub fn get_absolute_address(&mut self, mode: &AddressingMode, addr: u16) -> u16 {
        match mode {
            AddressingMode::Accumulator => 0,
            AddressingMode::ZeroPage => self.mem_read(addr) as u16,
            AddressingMode::Absolute => self.mem_read_u16(addr),
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(addr);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            }
            AddressingMode::Indirect_X => {
                let base = self.mem_read(addr);
                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(addr);
                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            }
            _ => { panic!("mode {:?} is not supported", mode); }
        }
    }

    fn add_to_register_a(&mut self, data: u8) {
        let sum: u16 = self.register_a as u16 + data as u16 + (if self.get_flag(StatusFlag::Carry) { 1 } else { 0 }) as u16;
        let carry: bool = sum > 0xff;
        if carry { self.set_flag(StatusFlag::Carry, true); }
        else { self.set_flag(StatusFlag::Carry, false); }
        let result: u8 = sum as u8;
        if (data ^ result) & (result ^ self.register_a) & 0x80 != 0 { self.set_flag(StatusFlag::Overflow, true) }
        else { self.set_flag(StatusFlag::Overflow, false) }
        self.register_a = result;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            let offset: i8 = self.mem_read(self.program_counter) as i8;
            self.program_counter = self.program_counter.wrapping_add(1).wrapping_add(offset as u16);
        }
    }

    fn bit_test(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let data: u8 = self.mem_read(addr);
        if (self.register_a & data) == 0 { self.set_flag(StatusFlag::Zero, true); }
        else { self.set_flag(StatusFlag::Zero, false); }
        self.set_flag(StatusFlag::Negative, data & 0b10000000 > 0);
        self.set_flag(StatusFlag::Overflow, data & 0b01000000 > 0);
    }

    fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Accumulator => 0,
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage  => self.mem_read(self.program_counter) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            AddressingMode::ZeroPage_X => {
                let pos: u8 = self.mem_read(self.program_counter);
                let addr: u16 = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos: u8 = self.mem_read(self.program_counter);
                let addr: u16 = pos.wrapping_add(self.register_y) as u16;
                addr
            }
            AddressingMode::Absolute_X => {
                let base: u16 = self.mem_read_u16(self.program_counter);
                let addr: u16 = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::Absolute_Y => {
                let base: u16 = self.mem_read_u16(self.program_counter);
                let addr: u16 = base.wrapping_add(self.register_y as u16);
                addr
            }
            AddressingMode::Indirect_X => {
                let base: u8 = self.mem_read(self.program_counter);
                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo: u8 = self.mem_read(ptr as u16);
                let hi: u8 = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base: u8 = self.mem_read(self.program_counter);
                let lo: u8 = self.mem_read(base as u16);
                let hi: u8 = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base: u16 = (hi as u16) << 8 | (lo as u16);
                let deref: u16 = deref_base.wrapping_add(self.register_y as u16);
                deref
            },
            AddressingMode::NoneAddressing => { panic!("mode {:?} is not supported", mode); }
        }
    }

    pub fn print_rom(&mut self) {
        for i in 0x8000..=0xffff {
            print!("{:02X} ", self.mem_read(i));
            if i % 16 == 15 { println!(); }
        }
    }
    pub fn load(&mut self, program: Vec<u8>) {
        // For Snake Game:
        // self.memory[0x0600..(0x0600 + program.len())].copy_from_slice(&program[..]);
        //for i in 0..(program.len() as u16) {
        //    self.mem_write(0x8600 + i, program[i as usize]);
        //}
        for i in 0..(program.len() as u16) {
            self.mem_write(0x8000 + i, program[i as usize]);
        }
        self.mem_write_u16(0x1FFC, 0x8600); // ! Moved from FFFC to 1FFC to be in RAM and not in ROM space
    }
    fn interrupt(&mut self, interrupt: interrupt::Interrupt) {
        self.stack_push_u16(self.program_counter);
        let mut flag: u8 = self.status.clone();
        if interrupt.b_flag_mask & 0b010000 == 1 { flag |= 0b010000; }
        if interrupt.b_flag_mask & 0b100000 == 1 { flag |= 0b100000; }
        self.stack_push(flag);
        self.set_flag(StatusFlag::InterruptDisable, true);
        self.bus.tick(interrupt.cpu_cycles);
        self.program_counter = self.mem_read_u16(interrupt.vector_addr);
    }
    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = STACK_RESET;
        self.status = 0b100100;
        // self.program_counter = 0xC000; // ! Moved from FFFC to 1FFC to be in RAM and not in ROM space
        self.program_counter = self.mem_read_u16(0xFFFC);
    }
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.program_counter = self.mem_read_u16(0xFFFC);
        self.run()
    }
    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }
    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;
        loop {
            if let Some(_nmi) = self.bus.poll_nmi_status() { self.interrupt(interrupt::NMI); }
            //callback(self);
         //   println!("{}", trace::trace(self));
            let code: u8 = self.mem_read(self.program_counter);
            //if self.program_counter != 0x8150 && self.program_counter != 0x8153 && self.program_counter != 0x8155{
            //println!("code: {:X}, pc:{:X}", code, self.program_counter);
            //}
            self.program_counter += 1;
            let program_counter_state: u16 = self.program_counter;
            let opcode: &&opcodes::OpCode = opcodes.get(&code).expect(&format!("OpCode 0x{:X} is not recognized", code));
            // Print the current state of the CPU
            //let v1 = self.mem_read(self.program_counter + 1);
            //let v2 = self.mem_read(self.program_counter + 2);
            //println!("A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PC:{:04X} ({:?} | {:X} {:X} {:X})",
            //         self.register_a, self.register_x, self.register_y, self.status, self.stack_pointer, self.program_counter, opcode.mnemonic, code, v1, v2);
            match code {
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => self.lda(&opcode.mode), // LDA
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => self.adc(&opcode.mode), // ADC
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => self.and(&opcode.mode), // AND
                0x0a => self.asl_accumulator(), // ASL
                0x06 | 0x16 | 0x0e | 0x1e => { self.asl(&opcode.mode); }, // ASL
                0x90 => self.branch(!self.get_flag(StatusFlag::Carry)), // BCC
                0xB0 => self.branch(self.get_flag(StatusFlag::Carry)), // BCS
                0xF0 => self.branch(self.get_flag(StatusFlag::Zero)), // BEQ
                0x24 | 0x2C => self.bit_test(&opcode.mode), // BIT
                0x30 => self.branch(self.get_flag(StatusFlag::Negative)), // BMI
                0xD0 => self.branch(!self.get_flag(StatusFlag::Zero)), // BNE
                0x10 => self.branch(!self.get_flag(StatusFlag::Negative)), // BPL
                0x50 => self.branch(!self.get_flag(StatusFlag::Overflow)), // BVC
                0x70 => self.branch(self.get_flag(StatusFlag::Overflow)), // BVS
                0x18 => self.set_flag(StatusFlag::Carry, false), // CLC
                0xD8 => self.set_flag(StatusFlag::DecimalMode, false), // CLD
                0x58 => self.set_flag(StatusFlag::InterruptDisable, false), // CLI
                0xB8 => self.set_flag(StatusFlag::Overflow, false), // CLV
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => self.cmp(&opcode.mode), // CMP
                0xE0 | 0xE4 | 0xEC => self.cpx(&opcode.mode), // CPX
                0xC0 | 0xC4 | 0xCC => self.cpy(&opcode.mode), // CPY
                0xC6 | 0xD6 | 0xCE | 0xDE => { self.dec(&opcode.mode); }, // DEC
                0xCA => self.dex(), // DEX
                0x88 => self.dey(), // DEY
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => self.eor(&opcode.mode), // EOR
                0xE6 | 0xF6 | 0xEE | 0xFE => { self.inc(&opcode.mode); }, // INC
                0xE8 => self.inx(), // INX
                0xC8 => self.iny(), // INY
                0x4C => { // JMP
                    let addr: u16 = self.mem_read_u16(self.program_counter);
                    self.program_counter = addr;
                },
                0x6C => { // JMP (indirect) - bug emulation
                    let addr: u16 = self.mem_read_u16(self.program_counter);
                    let indirect_ref: u16 = if addr & 0x00FF == 0x00FF {
                        let lo: u8 = self.mem_read(addr);
                        let hi: u8 = self.mem_read(addr & 0xFF00);
                        (hi as u16) << 8 | (lo as u16)
                    } else { self.mem_read_u16(addr) };
                    self.program_counter = indirect_ref;
                },
                0x20 => { // JSR
                    self.stack_push_u16(self.program_counter + 2 - 1);
                    let target_address: u16 = self.mem_read_u16(self.program_counter);
                    self.program_counter = target_address
                },
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => self.ldx(&opcode.mode), // LDX
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => self.ldy(&opcode.mode), // LDY
                0x4A => self.lsr_accumulator(), // LSR
                0x46 | 0x56 | 0x4E | 0x5E => { self.lsr(&opcode.mode); }, // LSR
                0xEA => { }, // NOP
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => self.ora(&opcode.mode), // ORA
                0x48 => self.stack_push(self.register_a), // PHA
                0x08 => self.stack_push(self.status | 0b0011_0000), // PHP
                0x68 => { self.register_a = self.stack_pop(); self.update_zero_and_negative_flags(self.register_a); }, // PLA
                0x28 => { // PLP
                    self.status = self.stack_pop();
                    self.set_flag(StatusFlag::Break, false);
                    self.set_flag(StatusFlag::Break2, true);
                },
                0x2A => self.rol_accumulator(), // ROL
                0x26 | 0x36 | 0x2E | 0x3E => { self.rol(&opcode.mode); }, // ROL
                0x6A => self.ror_accumulator(), // ROR
                0x66 | 0x76 | 0x6E | 0x7E => { self.ror(&opcode.mode); }, // ROR
                0x40 => { // RTI
                    self.status = self.stack_pop();
                    self.set_flag(StatusFlag::Break, false);
                    self.set_flag(StatusFlag::Break2, true);
                    self.program_counter = self.stack_pop_u16();
                },
                0x60 =>  self.program_counter = self.stack_pop_u16() + 1, // RTS
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => self.sbc(&opcode.mode), // SBC
                0x38 => self.set_flag(StatusFlag::Carry, true), // SEC
                0xF8 => self.set_flag(StatusFlag::DecimalMode, true), // SED
                0x78 => self.set_flag(StatusFlag::InterruptDisable, true), // SEI
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode), // STA
                0x86 | 0x96 | 0x8E => self.stx(&opcode.mode), // STX
                0x84 | 0x94 | 0x8C => self.sty(&opcode.mode), // STY
                0xAA =>  self.tax(), // TAX
                0xA8 =>  self.tay(), // TAY
                0xBA =>  self.tsx(), // TSX
                0x8A =>  self.txa(), // TXA
                0x9A =>  self.txs(), // TXS
                0x98 =>  self.tya(), // TYA
                0x00 => self.brk(), // BRK
                // Unofficial opcodes
                0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 0xf4 | 0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => { }, // *NOP = DOP
                0x0C | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => { }, // *NOP = TOP
                0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => { }, // *NOP = NOP
                0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => self.lax(&opcode.mode), // *LAX
                0x87 | 0x97 | 0x8F | 0x83 => self.sax(&opcode.mode), // *SAX
                0xEB => self.sbc(&AddressingMode::Immediate), // *SBC
                0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xC3 | 0xD3 => self.dcp(&opcode.mode), // *DCP
                0xE7 | 0xF7 | 0xEF | 0xFF | 0xFB | 0xE3 | 0xF3 => self.isb(&opcode.mode), // *ISB
                0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => self.slo(&opcode.mode), // *SLO
                0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x23 | 0x33 => self.rla(&opcode.mode), // *RLA
                0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => self.sre(&opcode.mode), // *SRE
                0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => self.rra(&opcode.mode), // *RRA

                _ => panic!("OpCode 0x{:X} is not recognized", code)
            }
            self.bus.tick(opcode.cycles);
            if program_counter_state == self.program_counter { self.program_counter += (opcode.len - 1) as u16; }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let bus = Bus::new(test::test_rom(), |_, _| {});
        let mut cpu = CPU::new(bus);
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 5);
        assert!(cpu.status & 0b0000_0010 == 0b00);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let bus = Bus::new(test::test_rom(), |_, _| {});
        let mut cpu = CPU::new(bus);
        cpu.register_a = 10;
        cpu.load_and_run(vec![0xa9, 0x0A,0xaa, 0x00]);
        assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let bus = Bus::new(test::test_rom(), |_, _| {});
        let mut cpu = CPU::new(bus);
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let bus = Bus::new(test::test_rom(), |_, _| {});
        let mut cpu = CPU::new(bus);
        cpu.register_x = 0xff;
        cpu.load_and_run(vec![0xe8, 0xe8, 0x00]);
        assert_eq!(cpu.register_x, 2)
    }

    #[test]
    fn test_lda_from_memory() {
        let bus = Bus::new(test::test_rom(), |_, _| {});
        let mut cpu = CPU::new(bus);
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register_a, 0x55);
    }
}
