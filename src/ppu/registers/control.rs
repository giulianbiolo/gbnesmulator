pub enum ControlFlags {
    NameTable1 = (1 << 0),
    NameTable2 = (1 << 1),
    VramAddIncrement = (1 << 2),
    SpritePatternAddr = (1 << 3),
    BackgroundPatternAddr = (1 << 4),
    SpriteSize = (1 << 5),
    MasterSlaveSelect = (1 << 6),
    GenerateNMI = (1 << 7),
}

pub trait FlagArithmetic {
    fn get_flag(&self, flag: ControlFlags) -> bool;
    fn set_flag(&mut self, flag: ControlFlags, value: bool);
    fn nametable_addr(&self) -> u16;
    fn sprt_pattern_addr(&self) -> u16;
    fn bknd_pattern_addr(&self) -> u16;
    fn sprite_size(&self) -> u8;
    fn master_slave_select(&self) -> u8;
}

