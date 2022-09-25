#![allow(unused)]
mod config;
mod gamepad;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::{fs, io};

use gilrs_core::Gilrs;
use minifb::{Key, ScaleMode, Window, WindowOptions};
use notify::{self, DebouncedEvent, RecursiveMode, Watcher};
use tiny_skia::Pixmap;

use config::ConfigWatcher;
use gamepad::Gamepad;

fn create_image(gamepad: &Gamepad) -> Pixmap {
    let bounds = gamepad.bounds();
    let width = bounds.right() as usize;
    let height = bounds.bottom() as usize;
    Pixmap::new(width as u32, height as u32).unwrap()
}

fn pick_gamepad(gilrs: &mut Gilrs, gamepad: &mut Gamepad) {
    let max_gamepads = gilrs.last_gamepad_hint();
    let gamepads: BTreeMap<usize, String> = (0..max_gamepads)
        .filter_map(|i| gilrs.gamepad(i).map(|g| (i, g.name().to_string())))
        .collect();
    println!("\nDetected {} gamepads:", max_gamepads);
    for (id, name) in gamepads {
        println!("{}: {}", id, name);
    }
    print!("\nEnter an id: ");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    gamepad.switch_gamepad(gilrs, line.trim().parse().unwrap());
}

fn update_screen(img: &mut Pixmap, buf: &mut [u32]) {
    for (pixel, n) in img.pixels_mut().iter().zip(buf.iter_mut()) {
        *n = (pixel.red() as u32) << 16 | (pixel.green() as u32) << 8 | pixel.blue() as u32;
    }
}

const FPS: Option<Duration> = Some(Duration::from_micros(16666));
const BENCHMARK: bool = false;

fn main() -> Result<(), ()> {
    let mut gilrs = Gilrs::new().unwrap();
    let mut gamepad = Gamepad::default();
    let mut watcher = ConfigWatcher::new(Duration::from_millis(100));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = match args.as_slice() {
        [] => "test.toml",
        [path] => path,
        [_, _, ..] => {
            println!("Pass no args for debug mode, or just the path to a config file");
            return Err(());
        }
    };
    let watch_file = fs::canonicalize(arg).unwrap();
    watcher.change_file(&watch_file);

    let config: Result<config::Gamepad, toml::de::Error> =
        toml::from_str(&fs::read_to_string(&watch_file).unwrap());
    match config {
        Ok(c) => gamepad.load_config(&mut gilrs, &c),
        Err(e) => println!("Invalid config: {}\n", e),
    }
    pick_gamepad(&mut gilrs, &mut gamepad);

    let options = WindowOptions {
        resize: false,
        scale_mode: ScaleMode::Stretch,
        ..Default::default()
    };

    let mut img = create_image(&gamepad);
    let mut width = img.width() as usize;
    let mut height = img.height() as usize;
    let mut buf = vec![0u32; width * height];
    gamepad.render(&mut img);
    update_screen(&mut img, &mut buf);
    let mut window = Window::new("Test", width, height, options).unwrap();
    window.limit_update_rate(FPS);

    let mut times = 0;
    let mut total = 0u128;
    while window.is_open() && !(window.is_key_down(Key::Escape) || window.is_key_down(Key::Q)) {
        while let Ok(event) = watcher.rx.try_recv() {
            use DebouncedEvent::*;
            match event {
                Create(p) | Write(p) if watch_file == p => {
                    match toml::from_str(&fs::read_to_string(p).unwrap()) {
                        Ok(config) => {
                            println!("Reloaded config...");
                            gamepad.load_config(&mut gilrs, &config);
                            let bounds = gamepad.bounds();
                            if width != bounds.right() as usize
                                || height != bounds.bottom() as usize
                            {
                                println!("Resized, making new window...");
                                img = create_image(&gamepad);
                                width = img.width() as usize;
                                height = img.height() as usize;
                                buf = vec![0u32; width * height];
                                window = Window::new("Test", width, height, options).unwrap();
                                window.limit_update_rate(FPS);
                            }
                            gamepad.render(&mut img);
                            update_screen(&mut img, &mut buf);
                        }
                        Err(e) => println!("Config reload failed: {}", e),
                    }
                }
                _ => {}
            }
        }

        let start = Instant::now();
        if gamepad.update(&mut gilrs) || BENCHMARK {
            gamepad.render(&mut img);
            update_screen(&mut img, &mut buf);
        }
        let end = Instant::now();
        total += (end - start).as_micros();
        times += 1;
        window.update_with_buffer(&buf, width, height).unwrap();
    }
    println!("{}us average render time per frame", total / times);
    Ok(())
}
