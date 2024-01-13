use std::collections::HashMap;

use apu::APU;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

use rodio::{OutputStream, source::Source, Sink};

pub mod cpu;
pub mod bus;
pub mod opcodes;
pub mod cartridge;
pub mod trace;
pub mod ppu;
pub mod render;
pub mod snake;
pub mod joypad;
pub mod apu;

use cpu::CPU;
use bus::Bus;
use ppu::NesPPU;
use cartridge::Rom;
use render::frame::Frame;
use joypad::JoypadButton;

struct NesSound { buffer: Vec<f32> }
impl Iterator for NesSound {
    type Item = f32;
    fn next(&mut self) -> Option<f32> { self.buffer.pop() }
}
impl Source for NesSound {
    fn current_frame_len(&self) -> Option<usize> { Some(self.buffer.len()) }
    fn channels(&self) -> u16 { 1 }
    fn sample_rate(&self) -> u32 { 44100 }
    fn total_duration(&self) -> Option<std::time::Duration> { None }
}

fn main() {
    let sdl_context: sdl2::Sdl = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("GBNesmulator", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas: sdl2::render::Canvas<sdl2::video::Window> = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump: EventPump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    //load the game
    let bytes: Vec<u8> = std::fs::read("games/apu_test.nes").unwrap();
    let rom: Rom = Rom::new(&bytes).unwrap();
    
    let mut frame: Frame = Frame::new();
    let mut key_map = HashMap::new();
    key_map.insert(Keycode::S, JoypadButton::Down);
    key_map.insert(Keycode::W, JoypadButton::Up);
    key_map.insert(Keycode::D, JoypadButton::Right);
    key_map.insert(Keycode::A, JoypadButton::Left);
    key_map.insert(Keycode::Backspace, JoypadButton::Select);
    key_map.insert(Keycode::Return, JoypadButton::Start);
    key_map.insert(Keycode::Space, JoypadButton::ButtonA);
    key_map.insert(Keycode::Q, JoypadButton::ButtonB);

    // Get handle to physical audio device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink: Sink = Sink::try_new(&stream_handle).unwrap();

    // * Save to a file the audio buffer for debugging
    //let filename = "audio.bin";
    //let mut file = std::fs::File::create(filename).unwrap();
    //let mut timing: std::time::Instant = std::time::Instant::now();

    let bus: Bus<'_> = Bus::new(rom, move |ppu: &NesPPU, apu: &mut APU, joypad: &mut joypad::Joypad| {
        render::render(ppu, &mut frame);
        texture.update(None, &frame.data, 256 * 3).unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                Event::KeyDown { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, true);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, false);
                    }
                }
                _ => { /* do nothing */ }
            }
        }
        // * Code for saving buffer to file for debugging
        /*****************
        let last_timing: std::time::Duration = timing.elapsed();
        let audio_buffer_size: usize = apu.buffer.len();
        println!("Time: {:?}ms\t|FPS: {:2.2}\t|BufSize: {}", last_timing.as_millis(), 1.0_f32 / timing.elapsed().as_secs_f32(), audio_buffer_size);
        timing = std::time::Instant::now();
        for f in apu.buffer.iter() {
            let bytes: [u8; 4] = unsafe { std::mem::transmute(*f) };
            file.write_all(&bytes).unwrap();
        }
        ****************/

        let sound: NesSound = NesSound { buffer: apu.buffer.clone() };
        sink.append(sound.amplify(0.1));
        //let _ = stream_handle.play_raw(source);
        apu.buffer.clear();
    });
    let mut cpu: CPU = CPU::new(bus);
    cpu.reset();
    cpu.run();
}
