use std::collections::HashMap;

use ppu::NesPPU;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;

pub mod cpu;
pub mod bus;
pub mod opcodes;
pub mod cartridge;
pub mod trace;
pub mod ppu;
pub mod render;
pub mod snake;
pub mod joypad;
use cpu::{CPU, Mem};
use bus::Bus;
use cartridge::Rom;
use render::frame::Frame;
use joypad::JoypadButton;

fn main() {
    let sdl_context: sdl2::Sdl = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Tile viewer", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
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
    let bytes: Vec<u8> = std::fs::read("games/donkey_kong.nes").unwrap();
    let rom: Rom = Rom::new(&bytes).unwrap();

    
    let mut frame: Frame = Frame::new();
    let mut key_map = HashMap::new();
    key_map.insert(Keycode::Down, JoypadButton::DOWN);
    key_map.insert(Keycode::Up, JoypadButton::UP);
    key_map.insert(Keycode::Right, JoypadButton::RIGHT);
    key_map.insert(Keycode::Left, JoypadButton::LEFT);
    key_map.insert(Keycode::Space, JoypadButton::SELECT);
    key_map.insert(Keycode::Return, JoypadButton::START);
    key_map.insert(Keycode::A, JoypadButton::BUTTON_A);
    key_map.insert(Keycode::S, JoypadButton::BUTTON_B);



    let bus: Bus<'_> = Bus::new(rom, move |ppu: &NesPPU, joypad: &mut joypad::Joypad| {
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
    });
    let mut cpu: CPU = CPU::new(bus);
    cpu.reset();
    cpu.run();
    /* * OLD CODE FUNCTIONING
    let bus = Bus::new(rom.clone(), move |ppu: &NesPPU| { });
    let mut screen_state = [0 as u8; 32 * 3 * 32];
    let mut cpu: CPU = CPU::new(bus);
    cpu.reset();
    //cpu.print_rom();
    //println!("");
    //cpu.load(rom.prg_rom);
    cpu.run_with_callback(|cpu: &mut CPU| {
        //println!("{}", trace(cpu));
        handle_user_input(cpu, &mut event_pump);
        cpu.mem_write(0xfe, rng.gen_range(1..16));
        if read_screen_state(cpu, &mut screen_state) {
            texture.update(None, &screen_state, 32 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }
    });
    */
}

fn handle_user_input<'a>(cpu: &mut CPU<'a>, event_pump: &mut EventPump) where CPU<'a>: Mem {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => { std::process::exit(0) },
            Event::KeyDown { keycode: Some(Keycode::W), .. } => { cpu.mem_write(0xff, 0x77); },
            Event::KeyDown { keycode: Some(Keycode::S), .. } => { cpu.mem_write(0xff, 0x73); },
            Event::KeyDown { keycode: Some(Keycode::A), .. } => { cpu.mem_write(0xff, 0x61); },
            Event::KeyDown { keycode: Some(Keycode::D), .. } => { cpu.mem_write(0xff, 0x64); },
            _ => {/* do nothing */}
        }
    }
}

fn color(byte: u8) -> Color {
    match byte {
        0 => sdl2::pixels::Color::BLACK,
        1 => sdl2::pixels::Color::WHITE,
        2 | 9 => sdl2::pixels::Color::GREY,
        3 | 10 => sdl2::pixels::Color::RED,
        4 | 11 => sdl2::pixels::Color::GREEN,
        5 | 12 => sdl2::pixels::Color::BLUE,
        6 | 13 => sdl2::pixels::Color::MAGENTA,
        7 | 14 => sdl2::pixels::Color::YELLOW,
        _ => sdl2::pixels::Color::CYAN,
    }
}

fn read_screen_state(cpu: &mut CPU, frame: &mut [u8; 32 * 3 * 32]) -> bool {
    let mut frame_idx = 0;
    let mut update = false;
    for i in 0x0200..0x600 {
        let color_idx = cpu.mem_read(i as u16);
        let (b1, b2, b3) = color(color_idx).rgb();
        if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
            frame[frame_idx] = b1;
            frame[frame_idx + 1] = b2;
            frame[frame_idx + 2] = b3;
            update = true;
        }
        frame_idx += 3;
    }
    update
}

