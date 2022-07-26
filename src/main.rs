extern crate sdl2;
use chip_8::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use std::time::{Duration, Instant};
mod chip_8;
pub fn main() {
    let mut args = std::env::args();
    if args.len() < 2 {
        usage();
        return;
    }
    let rom_path = args.nth(1).unwrap();
    let rom_path = std::path::Path::new(&rom_path);
    let mut chip8 = Chip8::init();
    match chip8.load(&rom_path) {
        Ok(_) => (),
        Err(why) => {
            println!("Could not load rom: {}", why);
            return;
        }
    }
    let sdl_context = match sdl2::init() {
        Ok(sdl) => sdl,
        Err(why) => {
            println!("{}", why);
            return;
        }
    };
    let video_subsystem = match sdl_context.video() {
        Ok(video) => video,
        Err(why) => {
            println!("{}", why);
            return;
        }
    };

    let window = match video_subsystem
        .window("Fry-8", 1280, 720)
        .position_centered()
        .resizable()
        .build()
    {
        Ok(window) => window,
        Err(why) => {
            println!("Failed to create window:{}", why);
            return;
        }
    };

    let mut canvas = window.into_canvas().software().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.present();
    //60hz , 16.67 ms
    let frametime = Duration::new(0, ((1.0 / 60.0) * 1000000000.0) as u32);
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        //display scaling
        let (x, y) = canvas.output_size().unwrap();
        let scaled_x: f32 = x as f32 / 64.0;
        let scaled_y: f32 = y as f32 / 32.0;
        canvas.set_scale(scaled_x, scaled_y);
        let current_frametime = Instant::now();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => chip8.keyboard.set_key(key),
                _ => {}
            }
        }
        match chip8.step() {
            Ok(i) => match i {
                SDLDo::Draw(rects) => {
                    canvas.set_draw_color(Color::RGB(255, 255, 255));
                    canvas.fill_rects(&rects);
                    canvas.present();
                }
                SDLDo::None => (),
                SDLDo::ClearScreen => {
                    canvas.set_draw_color(Color::RGB(0, 0, 0));
                    canvas.clear();
                    canvas.present();
                }
            },
            Err(why) => {
                println!("{}", why);
                break 'running;
            }
        }

        let elapsed = current_frametime.elapsed();
        if elapsed < frametime {
            std::thread::sleep(frametime - elapsed);
        }
    }
}

fn usage() {
    println!("USAGE: fry-8 [PATH TO ROM]");
}
