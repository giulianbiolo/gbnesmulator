use crate::apu::APU;
use crate::cpu::Mem;
use crate::cartridge::Rom;
use crate::ppu::{NesPPU, PPU};
use crate::joypad::Joypad;

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGISTERS_MIRRORS_END: u16 = 0x3FFF;

pub struct Bus<'call> {
    pub cpu_vram: [u8; 2048],
    prg_rom: Vec<u8>,
    ppu: NesPPU,
    apu: APU,
    pub cycles: usize,
    gameloop_callback: Box<dyn FnMut(&NesPPU, &mut APU, &mut Joypad) + 'call>,
    joypad1: Joypad,
}
impl<'a> Bus<'a> {
    pub fn new<'call, F>(rom: Rom, gameloop_callback: F) -> Bus<'call> where F: FnMut(&NesPPU, &mut APU, &mut Joypad) + 'call {
        let ppu: NesPPU = NesPPU::new(rom.chr_rom, rom.screen_mirroring);
        let apu: APU = APU::new();
        Bus { cpu_vram: [0; 2048], prg_rom: rom.prg_rom, ppu, apu, cycles: 0, gameloop_callback: Box::new(gameloop_callback), joypad1: Joypad::new() }
    }
    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        // Mirror down if ROM is 16KB instead of 32KB
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000 { addr = addr % 0x4000; }
        self.prg_rom[addr as usize]
    }
    pub fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        self.apu.tick(self.cycles as u64, cycles);
        let new_frame: bool = self.ppu.tick(cycles * 3);
        if new_frame { (self.gameloop_callback)(&self.ppu, &mut self.apu, &mut self.joypad1); }
    }
    pub fn reset_cycles(&mut self) { self.cycles = 0; }
    pub fn poll_nmi_status(&mut self) -> Option<u8> { self.ppu.poll_nmi_interrupt().take() }
}
impl Mem for Bus<'_> {
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00000111_11111111;
                self.cpu_vram[mirror_down_addr as usize]
            },
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => { panic!("Attempt to read from write-only PPU address {:x}", addr); }
            0x2002 => { self.ppu.read_status() },
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),
            0x2008..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_down_addr = addr & 0b00100000_00000111;
                self.mem_read(mirror_down_addr)
            },
            0x4015 => self.apu.read_register(),
            0x4000..=0x4015 => {
                // APU and I/O registers
                //println!("Read from APU at {:2X}", addr);
                0
            },
            0x4016 => self.joypad1.read(),
            0x4017 => { 0 }, // TODO: Implement joypad 2
            0x8000..=0xFFFF => self.read_prg_rom(addr),
            _ => { 0 } // { println!("Ignoring mem access at {:2X}", addr); 0 }
        }
    }
    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM..=RAM_MIRRORS_END => {
                let mirror_down_addr: u16 = addr & 0b111_1111_1111;
                self.cpu_vram[mirror_down_addr as usize] = data;
            },
            0x2000 => self.ppu.write_to_ctrl(data),
            0x2001 => self.ppu.write_to_mask(data),
            0x2002 => { panic!("Attempt to write to PPU status register") },
            0x2003 => self.ppu.write_to_oam_addr(data),
            0x2004 => self.ppu.write_to_oam_data(data),
            0x2005 => self.ppu.write_to_scroll(data),
            0x2006 => self.ppu.write_to_ppu_addr(data),
            0x2007 => self.ppu.write_to_data(data),
            0x2008..=PPU_REGISTERS_MIRRORS_END => {
                let mirror_down_addr: u16 = addr & 0b00100000_00000111;
                self.mem_write(mirror_down_addr, data);
            },
            0x4014 => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (data as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.mem_read(hi + i);
                }
                self.ppu.write_oam_dma(&buffer);

                // todo: handle this eventually
                //let add_cycles: u16 = if self.cycles % 2 == 1 { 514 } else { 513 };
                //self.tick(add_cycles as u8); //todo this will cause weird effects as PPU will have 513/514 * 3 ticks
            },
            0x4000..=0x4015 => self.apu.write_register(addr, data, self.cycles as u64),
            0x4016 => self.joypad1.write(data),
            0x4017 => self.apu.write_register(addr, data, self.cycles as u64),
            // 0x4017 => { } // TODO: Frame Counter of APU
            0x8000..=0xFFFF => { panic!("Attempt to write to Cartridge ROM space") },
            _ => {} //println!("Ignoring mem write-access at {:2X}", addr)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::cartridge::test;

    #[test]
    fn test_mem_read_write_to_ram() {
        let mut bus = Bus::new(test::test_rom(), |_, _, _| {});
        bus.mem_write(0x01, 0x55);
        assert_eq!(bus.mem_read(0x01), 0x55);
    }
}
