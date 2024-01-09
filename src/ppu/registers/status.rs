pub enum StatusFlags {
    NotUsed = (1 << 0),
    NotUsed2 = (1 << 1),
    NotUsed3 = (1 << 2),
    NotUsed4 = (1 << 3),
    NotUsed5 = (1 << 4),
    SpriteOverflow = (1 << 5),
    SpriteZeroHit = (1 << 6),
    VBlankStarted = (1 << 7)
}

pub trait StatusArithmetic {
    fn get_status(&self, flag: StatusFlags) -> bool;
    fn set_status(&mut self, flag: StatusFlags, value: bool);
    fn is_in_vblank(&self) -> bool { self.get_status(StatusFlags::VBlankStarted) }
}
