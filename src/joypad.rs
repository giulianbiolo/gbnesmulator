// use bitflags::bitflags;

/*
bitflags! {
    // https://wiki.nesdev.com/w/index.php/Controller_reading_code
    #[derive(Default, Copy, Clone)]
    pub struct JoypadButton: u8 {
        const RIGHT             = 0b10000000;
        const LEFT              = 0b01000000;
        const DOWN              = 0b00100000;
        const UP                = 0b00010000;
        const START             = 0b00001000;
        const SELECT            = 0b00000100;
        const BUTTON_B          = 0b00000010;
        const BUTTON_A          = 0b00000001;
    }
}
*/
#[derive(Debug, Copy, Clone)]
pub enum JoypadButton {
    ButtonA = (1 << 0),
    ButtonB = (1 << 1),
    Select = (1 << 2),
    Start = (1 << 3),
    Up = (1 << 4),
    Down = (1 << 5),
    Left = (1 << 6),
    Right = (1 << 7),
}

#[derive(Default, Copy, Clone)]
pub struct Joypad {
    strobe: bool,
    button_index: u8,
    button_status: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe: false,
            button_index: 0,
            button_status: 0,
        }
    }

    pub fn write(&mut self, data: u8) {
        self.strobe = data & 1 == 1;
        if self.strobe {
            self.button_index = 0
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }
        let response: u8 = (self.button_status & (1 << self.button_index)) >> self.button_index;
        if !self.strobe && self.button_index <= 7 {
            self.button_index += 1;
        }
        response
    }

    pub fn set_button_pressed_status(&mut self, button: JoypadButton, pressed: bool) {
        if pressed { self.button_status |= button as u8; }
        else { self.button_status &= !(button as u8); }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_strobe_mode() {
        let mut joypad = Joypad::new();
        joypad.write(1);
        joypad.set_button_pressed_status(JoypadButton::ButtonA, true);
        for _x in 0..10 {
            assert_eq!(joypad.read(), 1);
        }
    }

    #[test]
    fn test_strobe_mode_on_off() {
        let mut joypad = Joypad::new();

        joypad.write(0);
        joypad.set_button_pressed_status(JoypadButton::Right, true);
        joypad.set_button_pressed_status(JoypadButton::Left, true);
        joypad.set_button_pressed_status(JoypadButton::Select, true);
        joypad.set_button_pressed_status(JoypadButton::ButtonB, true);

        for _ in 0..=1 {
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 0);
            assert_eq!(joypad.read(), 1);
            assert_eq!(joypad.read(), 1);

            for _x in 0..10 {
                assert_eq!(joypad.read(), 1);
            }
            joypad.write(1);
            joypad.write(0);
        }
    }
}
