pub struct EnvelopeControl {
    pub constant_level: u8,
    pub decay_period: u8,
    pub constant: bool,
    pub looping: bool
}

pub struct Envelope {
    control: EnvelopeControl,
    counter: u8,
    level: u8,
    start: bool,
}

impl Envelope {
    pub fn new() -> Self {
        Envelope {
            counter: 0,
            level: 0,
            control: EnvelopeControl {
                constant_level: 0,
                decay_period: 0,
                constant: false,
                looping: false
            },
            start: false,
        }
    }

    pub fn tick(&mut self) {
        if self.start {
            self.start = false;
            self.set_level(0x0f);
        } else {
            if self.counter > 0 {
                self.counter -= 1;
            } else {
                if self.level > 0 {
                    let l: u8 = self.level - 1;
                    self.set_level(l);
                } else if self.control.looping {
                    self.set_level(0x0f);
                }
            }
        }
    }

    fn set_level(&mut self, v: u8) {
        self.level = v & 0x0f;
        self.counter = self.control.decay_period;
    }

    pub fn write_register(&mut self, data: u8) {
        self.control.constant_level = data & 0b0000_1111;
        self.control.decay_period = data & 0b0000_1111;
        self.control.constant = data & 0b0001_0000 != 0;
        self.control.looping = data & 0b0010_0000 != 0;
    }
    pub fn start(&mut self) { self.start = true; }
    pub fn volume(&self) -> u8 { if self.control.constant { self.control.constant_level } else { self.level } }
}
