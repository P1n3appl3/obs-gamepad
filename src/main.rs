mod config;
mod gamepad;

use std::time::{Duration, Instant};

use gilrs_core::Gilrs;
use minifb::{Key, ScaleMode, Window, WindowOptions};
use tiny_skia::Pixmap;

use gamepad::Gamepad;

fn main() -> Result<(), ()> {
    let mut gilrs = Gilrs::new().unwrap();
    let mut gamepad = Gamepad::new(&mut gilrs);
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            gamepad.add_debug_inputs(&mut gilrs);
        }
        [path] => {
            todo!()
        }
        [_, _, ..] => {
            println!("Pass no args for debug mode, or just the path to a config file");
            return Err(());
        }
    };

    let bounds = gamepad.bounds();
    let width = bounds.right() as usize;
    let height = bounds.bottom() as usize;
    let mut img = Pixmap::new(width as u32, height as u32).unwrap();
    let mut buf = vec![0u32; width * height];

    let options = WindowOptions {
        resize: false,
        scale_mode: ScaleMode::Stretch,
        ..Default::default()
    };
    let mut window = Window::new("Test", width, height, options).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.limit_update_rate(Some(Duration::from_micros(16666)));
    while window.is_open()
        && !(window.is_key_down(Key::Escape) || window.is_key_down(Key::Q))
    {
        gamepad.update(&mut gilrs);
        gamepad.render(&mut img);
        for (pixel, n) in img.pixels_mut().iter().zip(buf.iter_mut()) {
            *n = (pixel.red() as u32) << 16
                | (pixel.green() as u32) << 8
                | pixel.blue() as u32;
        }
        window.update_with_buffer(&buf, width, height).unwrap();
    }
    Ok(())
}
