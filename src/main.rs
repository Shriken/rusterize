extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::error;
use std::process;
use std::thread;
use std::time::Duration;
use std::time::Instant;

mod pixel;
mod renderer;
mod screen;
mod texture;

use renderer::Renderer;

macro_rules! main_try {
    ($x:expr) => {{
        let var = $x;
        if let Err(e) = var {
            error(&*e);
            return;
        }
        var.unwrap()
    }}
}


const SCREEN_WIDTH:  u32 = 20;
const SCREEN_HEIGHT: u32 = 20;

const TARGET_FPS: u32 = 60;
const FRAME_LEN_NANOS: u32 = 1_000_000_000 / (TARGET_FPS as u32);

fn main() {
    // Initialize screen.
    let screen = screen::TextScreen::new(
        "softraster",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
    );
    let mut renderer = Renderer::new(screen);

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Main loop.
    'main_loop: loop {
        // Time frame length.
        let frame_start = Instant::now();

        // Parse events.
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'main_loop;
                },

                _ => {},
            }
        }

        // Draw stuff.
        renderer.clear();
        renderer.draw_line(0, 0, 5, 3);
        renderer.display();

        // Sleep until end-of-frame.
        let frame_duration = Instant::now() - frame_start;
        let max_sleep = Duration::new(0, FRAME_LEN_NANOS);
        if max_sleep > frame_duration {
            thread::sleep(max_sleep - frame_duration);
        }
        break;
    }
}

fn error(err: &error::Error) {
    println!("error: {}", err);
    process::exit(-1);
}
