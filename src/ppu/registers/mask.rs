pub enum MaskFlags {
    Grayscale = (1 << 0),
    Leftmost8PixelBackground = (1 << 1),
    Leftmost8PixelSprite = (1 << 2),
    ShowBackground = (1 << 3),
    ShowSprites = (1 << 4),
    EmphasiseRed = (1 << 5),
    EmphasiseGreen = (1 << 6),
    EmphasiseBlue = (1 << 7)
}

pub enum Color { Red, Green, Blue }

pub trait MaskArithmetic {
    fn get_mask(&self, flag: MaskFlags) -> bool;
    fn set_mask(&mut self, flag: MaskFlags, value: bool);
    fn is_grayscale(&self) -> bool { self.get_mask(MaskFlags::Grayscale) }
    fn emphasise(&self) -> Vec<Color> {
        let mut result = vec![];
        if self.get_mask(MaskFlags::EmphasiseRed) { result.push(Color::Red); }
        if self.get_mask(MaskFlags::EmphasiseGreen) { result.push(Color::Green); }
        if self.get_mask(MaskFlags::EmphasiseBlue) { result.push(Color::Blue); }
        result
    }
}
