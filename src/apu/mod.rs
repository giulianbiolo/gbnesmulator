mod frame_counter;
mod lenght_counter;
mod pulse_channel;
mod noise_channel;
mod triangle_channel;
mod dmc_channel;
mod filter;
mod sequencer;
mod sweep;
mod envelope;

const SAMPLE_RATE: f64 = 44_100.0;

use frame_counter::{FrameResult, FrameCounter};
use pulse_channel::PulseChannel;
use noise_channel::NoiseChannel;
use dmc_channel::DmcChannel;
use filter::FirstOrderFilter;

use self::{sweep::SweepNegationMode, triangle_channel::TriangleChannel};

pub struct APU {
    pub buffer: Vec<f32>,
    pub executed_cycles: u32,
    pub frame_counter: FrameCounter,
    pub pulse_0: PulseChannel,
    pub pulse_1: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,
    pub dmc: DmcChannel,
    pub filters: [FirstOrderFilter; 3]
}

impl APU {
    pub fn new() -> Self {
        APU {
            buffer: Vec::new(),
            executed_cycles: 0,
            frame_counter: FrameCounter::new(),
            pulse_0: PulseChannel::new(SweepNegationMode::OnesCompliment),
            pulse_1: PulseChannel::new(SweepNegationMode::TwosCompliment),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            filters: [
                FirstOrderFilter::high_pass(SAMPLE_RATE, 90.0),
                FirstOrderFilter::high_pass(SAMPLE_RATE, 440.0),
                FirstOrderFilter::low_pass(SAMPLE_RATE, 14_000.0),
            ],
        }
    }

    pub fn reset(&mut self) {
        self.write_register(0x4017, 0, 0);
        for i in 0..11 { self.tick(i, 0); }
    }

    pub fn read_register(&mut self) -> u8 {
        let mut result = 0;
        if self.dmc.irq_flag { result |= 0b1000_0000; }
        if self.frame_counter.private_irq_flag { result |= 0b0100_0000; }
        if self.dmc.playing() { result |= 0b0001_0000; }
        if self.noise.playing() { result |= 0b0000_1000; }
        if self.triangle.playing() { result |= 0b0000_0100; }
        if self.pulse_1.playing() { result |= 0b0000_0010; }
        if self.pulse_0.playing() { result |= 0b0000_0001; }

        self.frame_counter.private_irq_flag = false;
        self.frame_counter.public_irq_flag = false;
        result
    }

    pub fn write_register(&mut self, address: u16, value: u8, cycles: u64) {
        match address {
            0x4000..=0x4003 => self.pulse_0.write_register(address, value),
            0x4004..=0x4007 => self.pulse_1.write_register(address, value),
            0x4008..=0x400B => self.triangle.write_register(address, value),
            0x400C..=0x400F => self.noise.write_register(address, value),
            0x4010..=0x4013 => self.dmc.write_register(address, value),
            0x4015 => {
                self.pulse_0.set_enabled(value & 0b0000_0001 != 0);
                self.pulse_1.set_enabled(value & 0b0000_0010 != 0);
                self.triangle.set_enabled(value & 0b0000_0100 != 0);
                self.noise.set_enabled(value & 0b0000_1000 != 0);
                self.dmc.set_enabled(value & 0b0001_0000 != 0);
            }
            0x4017 => {
                let r: FrameResult = self.frame_counter.write_register(value, cycles);
                self.handle_frame_result(r);
            }
            _ => {}
            //_ => panic!("Bad APU address: {:04X}", address),
        }
    }

    pub fn tick(&mut self, cpu_cycles: u64, opcode_cycles: u8) {
        // Triangle ticks on each cpu cycle.
        self.triangle.tick_sequencer();

        // Everything else ticks on every other cycle
        if cpu_cycles % 2 == 1 {
            self.pulse_0.tick_sequencer();
            self.pulse_1.tick_sequencer();
            self.noise.tick_sequencer();
            self.dmc.tick_sequencer();
        }

        let r: FrameResult = self.frame_counter.tick();
        self.handle_frame_result(r);

        self.pulse_0.update_pending_length_counter();
        self.pulse_1.update_pending_length_counter();
        self.triangle.update_pending_length_counter();
        self.noise.update_pending_length_counter();

        // We need 730 stereo audio samples per frame for 60 fps.
        // Each frame lasts a minimum of 29,779 CPU cycles. This
        // works out to around 40 CPU cycles per sample.
        self.executed_cycles += opcode_cycles as u32;
        //println!("cycles: {}", self.executed_cycles)
        if self.executed_cycles >= 40 {
            let s: f32 = self.sample();
            self.buffer.push(s);
            //self.buffer.push(s);
            self.executed_cycles = 0;
        }
    }

    fn handle_frame_result(&mut self, result: FrameResult) {
        match result {
            FrameResult::Quarter => {
                self.pulse_0.tick_quarter_frame();
                self.pulse_1.tick_quarter_frame();
                self.triangle.tick_quarter_frame();
            }
            FrameResult::Half => {
                self.pulse_0.tick_quarter_frame();
                self.pulse_0.tick_half_frame();
                self.pulse_1.tick_quarter_frame();
                self.pulse_1.tick_half_frame();
                self.triangle.tick_quarter_frame();
                self.triangle.tick_half_frame();
                self.noise.tick_quarter_frame();
                self.noise.tick_half_frame();
            }
            FrameResult::None => (),
        }
    }

    pub fn irq_flag(&self) -> bool { self.frame_counter.public_irq_flag /*|| self.dmc.irq_flag*/ }
    /*
    fn sample(&mut self) -> (i32, i32) {
        let p0: f64 = self.pulse_0.sample() as f64;
        let p1: f64 = self.pulse_1.sample() as f64;
        let t: f64 = self.triangle.sample() as f64;
        let n: f64 = self.noise.sample() as f64;
        let d: f64 = self.dmc.sample() as f64;
        (p0 as i32, p1 as i32)
    }
    */
    
    fn sample(&mut self) -> f32 {
        let p0: f64 = self.pulse_0.sample() as f64;
        let p1: f64 = self.pulse_1.sample() as f64;
        let t: f64 = self.triangle.sample() as f64;
        let n: f64 = self.noise.sample() as f64;
        let d: f64 = self.dmc.sample() as f64;

        // Combine channels into a single value from 0.0 to 1.0
        // Formula is from http://wiki.nesdev.com/w/index.php/APU_Mixer
        //println!("p0: {}, p1: {}, t: {}, n: {}, d: {}", p0, p1, t, n, d);
        let pulse_out: f64;
        let tnd_out: f64;
        if p0 + p1 < 0.1 { pulse_out = 0.0; } else { pulse_out = 95.88 / ((8128.0 / (p0 + p1)) + 100.0); }
        if t + n + d < 0.1 { tnd_out = 0.0; } else { tnd_out = 159.79 / ((1.0 / (t / 8227.0 + n / 12241.0 + d / 22638.0)) + 100.0); }
        // Linear approximation of the above formula
        //let pulse_out: f64 = 0.00752 * (p0 + p1);
        //let tnd_out: f64 = 0.00851 * t + 0.00494 * n + 0.00335 * d;

        // Scale to 0..65536
        //let mut output = (pulse_out + tnd_out) * 65535.0;
        let mut output: f64 = pulse_out + tnd_out;

        // Apply high pass and low pass filters
        for i in 0..3 { output = self.filters[i].tick(output); }

        // The final range is -32767 to +32767
        output as f32
    }
}
