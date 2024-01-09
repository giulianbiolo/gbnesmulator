pub mod registers;
use crate::cartridge::Mirroring;
use registers::mask::{MaskFlags, MaskArithmetic};
use registers::control::{ControlFlags, FlagArithmetic};
use registers::addr::AddrRegister;
use registers::status::{StatusFlags, StatusArithmetic};
use registers::scroll::ScrollRegister;


pub trait PPU {
    fn write_to_ctrl(&mut self, value: u8);
    fn write_to_mask(&mut self, value: u8);
    fn read_status(&mut self) -> u8; 
    fn write_to_oam_addr(&mut self, value: u8);
    fn write_to_oam_data(&mut self, value: u8);
    fn read_oam_data(&self) -> u8;
    fn write_to_scroll(&mut self, value: u8);
    fn write_to_ppu_addr(&mut self, value: u8);
    fn write_to_data(&mut self, value: u8);
    fn read_data(&mut self) -> u8;
    fn write_oam_dma(&mut self, value: &[u8; 256]);
}
pub struct NesPPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub vram: [u8; 2048],
    pub oam_addr: u8,
    pub oam_data: [u8; 64 * 4],
    pub addr: AddrRegister,
    pub mirroring: Mirroring,
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub scroll: ScrollRegister,
    pub scanline: u16,
    pub cycles: usize,
    internal_data_buf: u8,
    pub nmi_interrupt: Option<u8>,
}
impl NesPPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        NesPPU {
            chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],
            oam_addr: 0,
            oam_data: [0; 64 * 4],
            addr: AddrRegister::new(),
            mirroring,
            ctrl: 0,
            mask: 0,
            status: 0,
            scroll: ScrollRegister::new(),
            internal_data_buf: 0,
            scanline: 0,
            cycles: 0,
            nmi_interrupt: None,
        }
    }
    pub fn new_empty_rom() -> Self { NesPPU::new(vec![0; 2048], Mirroring::HORIZONTAL) }
    fn vram_addr_increment(&self) -> u8 { if !self.get_flag(ControlFlags::VramAddIncrement) { 1 } else { 32 } }
    fn increment_vram_addr(&mut self) { self.addr.increment(self.vram_addr_increment()); }
    fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram: u16 = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index: u16 = mirrored_vram - 0x2000; // to vram vector
        let name_table: u16 = vram_index / 0x400; // to the name table index
        match (&self.mirroring, name_table) {
            (Mirroring::VERTICAL, 2) | (Mirroring::VERTICAL, 3) => vram_index - 0x800,
            (Mirroring::HORIZONTAL, 2) => vram_index - 0x400,
            (Mirroring::HORIZONTAL, 1) => vram_index - 0x400,
            (Mirroring::HORIZONTAL, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }
    pub fn tick(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as usize;
        if self.cycles >= 341 {
            self.cycles = self.cycles - 341;
            self.scanline += 1;
            if self.scanline == 241 {
                self.set_status(StatusFlags::VBlankStarted, true);
                self.set_status(StatusFlags::SpriteZeroHit, false);
                if self.get_flag(ControlFlags::GenerateNMI) {
                    self.nmi_interrupt = Some(1);
                    // todo!("Should trigger NMI interrupt")
                }
            }
            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = None;
                self.set_status(StatusFlags::SpriteZeroHit, false);
                self.set_status(StatusFlags::VBlankStarted, false);
                return true;
            }
        }
        return false;
    }
    pub fn poll_nmi_interrupt(&mut self) -> Option<u8> { self.nmi_interrupt.take() }
}
impl PPU for NesPPU {
    fn write_to_ctrl(&mut self, value: u8) {
        let before_nmi: bool = self.get_flag(ControlFlags::GenerateNMI);
        self.ctrl = value;
        if !before_nmi && self.get_flag(ControlFlags::GenerateNMI) && self.get_status(StatusFlags::VBlankStarted) { self.nmi_interrupt = Some(1); }
    }
    fn write_to_mask(&mut self, value: u8) { self.mask = value; }
    fn read_status(&mut self) -> u8 {
        let result: u8 = self.status;
        self.set_status(StatusFlags::VBlankStarted, false);
        self.addr.reset_latch();
        self.scroll.reset_latch();
        result
    }
    fn write_to_oam_addr(&mut self, value: u8) { self.oam_addr = value; }
    fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }
    fn read_oam_data(&self) -> u8 { self.oam_data[self.oam_addr as usize] }
    fn write_to_scroll(&mut self, value: u8) { self.scroll.write(value); }
    fn write_to_ppu_addr(&mut self, value: u8) { self.addr.update(value); }
    fn write_to_data(&mut self, value: u8) {
        let addr: u16 = self.addr.get();
        match addr {
            0..=0x1fff => println!("attempt to write to chr rom space {}", addr), 
            0x2000..=0x2fff => { self.vram[self.mirror_vram_addr(addr) as usize] = value; },
            0x3000..=0x3eff => panic!("addr {} shouldn't be used in reality", addr),
            //Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize] = value;
            },
            0x3f00..=0x3fff => { self.palette_table[(addr - 0x3f00) as usize] = value; },
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }
    fn read_data(&mut self) -> u8 {
        let addr: u16 = self.addr.get();
        self.increment_vram_addr();
        match addr {
            0..=0x1fff => {
                let result: u8 = self.internal_data_buf;
                self.internal_data_buf = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2fff => {
                let result: u8 = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            0x3000..=0x3eff => panic!("Addr space 0x3000..=0x3eff is not supposed to be used, requested = {}", addr),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let addr_mirror: u16 = addr - 0x10;
                self.palette_table[(addr_mirror - 0x3f00) as usize]
            }
            0x3f00..=0x3fff => { self.palette_table[(addr - 0x3f00) as usize] } // ! need to check this
            _ => panic!("Unexpected access to mirrored space: {}", addr)
        }
    }
    fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }
}
impl FlagArithmetic for NesPPU {
    fn get_flag(&self, flag: ControlFlags) -> bool { (self.ctrl & (flag as u8)) > 0 }
    fn set_flag(&mut self, flag: ControlFlags, value: bool) {
        if value { self.ctrl |= flag as u8; }
        else { self.ctrl &= !(flag as u8); }
    }
    fn nametable_addr(&self) -> u16 {
        match self.ctrl & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2c00,
            _ => panic!("not possible"),
        }
    }
    fn sprt_pattern_addr(&self) -> u16 { if !self.get_flag(ControlFlags::SpritePatternAddr) { 0 } else { 0x1000 } }
    fn bknd_pattern_addr(&self) -> u16 { if !self.get_flag(ControlFlags::BackgroundPatternAddr) { 0 } else { 0x1000 } }
    fn sprite_size(&self) -> u8 { if !self.get_flag(ControlFlags::SpriteSize) { 8 } else { 16 } }
    fn master_slave_select(&self) -> u8 { if !self.get_flag(ControlFlags::SpriteSize) { 0 } else { 1 } }
}
impl MaskArithmetic for NesPPU {
    fn get_mask(&self, flag: MaskFlags) -> bool { (self.ctrl & (flag as u8)) > 0 }
    fn set_mask(&mut self, flag: MaskFlags, value: bool) {
        if value { self.ctrl |= flag as u8; }
        else { self.ctrl &= !(flag as u8); }
    }
}
impl StatusArithmetic for NesPPU {
    fn get_status(&self, flag: StatusFlags) -> bool { (self.status & (flag as u8)) > 0 }
    fn set_status(&mut self, flag: StatusFlags, value: bool) {
        if value { self.status |= flag as u8; }
        else { self.status &= !(flag as u8); }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);
        ppu.write_to_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x66); //write to a

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x77); //write to B

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from b
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = NesPPU::new(vec![0; 2048], Mirroring::VERTICAL);

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x66); //write to A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x77); //write to b

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from a

        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from B
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x63); //0x6305 -> 0x2305
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        // assert_eq!(ppu.addr.read(), 0x0306)
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.set_status(StatusFlags::VBlankStarted, true);
        let status: u8 = ppu.read_status();
        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_oam_addr(0x10);
        ppu.write_to_oam_data(0x66);
        ppu.write_to_oam_data(0x77);

        ppu.write_to_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_to_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut ppu = NesPPU::new_empty_rom();

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_to_oam_addr(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_to_oam_addr(0xf); //wrap around
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_to_oam_addr(0x10);
        ppu.write_to_oam_addr(0x77);
        ppu.write_to_oam_addr(0x11);
        ppu.write_to_oam_addr(0x66);
    }
}
