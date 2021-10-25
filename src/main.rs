mod config;
mod gamepad;

use std::fs;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use gilrs_core::Gilrs;
use minifb::{Key, ScaleMode, Window, WindowOptions};
use notify::{self, DebouncedEvent, RecursiveMode, Watcher};
use tiny_skia::Pixmap;

use gamepad::Gamepad;

fn main() -> Result<(), ()> {
    let mut gilrs = Gilrs::new().unwrap();
    let mut gamepad = Gamepad::new(&mut gilrs);
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::watcher(tx, Duration::from_millis(100)).unwrap();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let watch_file = match args.as_slice() {
        [] => {
            gamepad.add_debug_inputs(&mut gilrs);
            None
        }
        [path] => {
            gamepad.load_config(
                &mut gilrs,
                &toml::from_str(&fs::read_to_string(path).unwrap()).unwrap(),
            );
            let path = fs::canonicalize(path).unwrap();
            watcher
                .watch(path.parent().unwrap(), RecursiveMode::Recursive)
                .unwrap();
            Some(path)
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
    let mut times = 0;
    let mut total = 0u128;
    while window.is_open()
        && !(window.is_key_down(Key::Escape) || window.is_key_down(Key::Q))
    {
        while let Ok(event) = rx.try_recv() {
            use DebouncedEvent::*;
            match event {
                Create(p) | Write(p) if p == *watch_file.as_ref().unwrap() => {
                    match toml::from_str(&fs::read_to_string(p).unwrap()) {
                        Ok(config) => {
                            println!("Reloaded config...");
                            gamepad.load_config(&mut gilrs, &config);
                        }
                        Err(e) => println!("Config reload failed: {}", e),
                    }
                }
                _ => {}
            }
        }
        let start = Instant::now();
        gamepad.update(&mut gilrs);
        gamepad.render(&mut img);
        for (pixel, n) in img.pixels_mut().iter().zip(buf.iter_mut()) {
            *n = (pixel.red() as u32) << 16
                | (pixel.green() as u32) << 8
                | pixel.blue() as u32;
        }
        let end = Instant::now();
        total += (end - start).as_micros();
        times += 1;
        window.update_with_buffer(&buf, width, height).unwrap();
    }
    println!("{}us average render time", total / times);
    Ok(())
}
