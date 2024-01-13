pub struct ScrollRegister {
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub latch: bool,
}

impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            scroll_x: 0,
            scroll_y: 0,
            latch: false,
        }
    }
    pub fn write(&mut self, data: u8) {
        // if data == 0 { return; }
        if !self.latch { self.scroll_x = data; /*println!("Set X to {}", data);*/ }
        else { self.scroll_y = data; /*println!("Set Y to {}", data);*/ }
        self.latch = !self.latch;
    }
    pub fn reset_latch(&mut self) { self.latch = false; }
}