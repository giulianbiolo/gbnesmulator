use std::collections::HashMap;
use crate::opcodes;

pub enum StatusFlag {
    Carry = (1 << 0),
    Zero = (1 << 1),
    InterruptDisable = (1 << 2),
    DecimalMode = (1 << 3),
    Break = (1 << 4),
    Unused = (1 << 5),
    Overflow = (1 << 6),
    Negative = (1 << 7),
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
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

trait Mem {
    fn mem_read(&self, addr: u16) -> u8; 
    fn mem_write(&mut self, addr: u16, data: u8);
    fn mem_read_u16(&self, pos: u16) -> u16 {
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

trait FlagArithmetic {
    fn get_flag(&self, flag: StatusFlag) -> bool;
    fn set_flag(&mut self, flag: StatusFlag, value: bool);
    fn update_zero_and_negative_flags(&mut self, result: u8);
}

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    memory: [u8; 0xFFFF],
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8 { self.memory[addr as usize] }
    fn mem_write(&mut self, addr: u16, data: u8) { self.memory[addr as usize] = data; }
}

impl FlagArithmetic for CPU {
    fn get_flag(&self, flag: StatusFlag) -> bool { (self.status & (flag as u8)) > 0 }
    fn set_flag(&mut self, flag: StatusFlag, value: bool) {
        if value { self.status |= flag as u8; }
        else { self.status &= !(flag as u8); }
    }
    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 { self.set_flag(StatusFlag::Zero, true); }
        else { self.set_flag(StatusFlag::Zero, false); }
        if result & 0b1000_0000 != 0 { self.set_flag(StatusFlag::Negative, true); }
        else { self.set_flag(StatusFlag::Negative, false); }
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
            memory: [0; 0xFFFF],
        }
    }

    fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
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
            }
            AddressingMode::NoneAddressing => { panic!("mode {:?} is not supported", mode); }
        }
    }

    pub fn reset(&mut self) {
        // Reset registers and program counter
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0;
        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    // Instructions
    fn lda(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let value: u8 = self.mem_read(addr);
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run()
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000 .. (0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;
        loop {
            let code: u8 = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state: u16 = self.program_counter;
            let opcode: &&opcodes::OpCode = opcodes.get(&code).expect(&format!("OpCode {:x} is not recognized", code));

            match code {
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => self.lda(&opcode.mode), // LDA
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode), // STA
                0xAA =>  self.tax(), // TAX
                0xE8 => self.inx(), // INX
                0x00 => return, // BRK
                _ => todo!()
            }
            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }
}

#[cfg(test)]
mod test {
   use super::*;
 
    #[test]
    fn test_0xa9_lda_immediate_load_data() {
       let mut cpu = CPU::new();
       cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
       assert_eq!(cpu.register_a, 5);
       assert!(cpu.status & 0b0000_0010 == 0);
       assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
       let mut cpu = CPU::new();
       cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
       assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
       let mut cpu = CPU::new();
       cpu.load_and_run(vec![0xa9, 0x0A,0xaa, 0x00]);
       assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
       let mut cpu = CPU::new();
       cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
       assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
       let mut cpu = CPU::new();
       cpu.load_and_run(vec![0xa9, 0xff, 0xaa,0xe8, 0xe8, 0x00]);
       assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
       let mut cpu = CPU::new();
       cpu.mem_write(0x10, 0x55);
       cpu.load_and_run(vec![0xa5, 0x10, 0x00]);
       assert_eq!(cpu.register_a, 0x55);
    }
}
