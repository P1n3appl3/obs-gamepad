mod gamepad;
use std::time;

use gilrs_core::{Gamepad, Gilrs};
use minifb::{Key, ScaleMode, Window, WindowOptions};
use tiny_skia::Pixmap;

fn main() {
    let mut gilrs = Gilrs::new().unwrap();
    let mut gamepad = gamepad::Gamepad::new(&mut gilrs);
    println!("id: {}", gamepad.id);

    let width = 500;
    let height = 500;
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

    window.limit_update_rate(Some(time::Duration::from_micros(16666)));
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
}
