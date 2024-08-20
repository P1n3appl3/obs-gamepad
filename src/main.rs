mod config;
mod gamepad;
mod serial;
mod usb;

use std::collections::BTreeMap;
use std::io::Write;
use std::time::{Duration, Instant};
use std::{fs, io};

use gilrs_core::Gilrs;
use minifb::{Key, ScaleMode, Window, WindowOptions};
use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind};
use tiny_skia::Pixmap;

use config::ConfigWatcher;
use gamepad::{Gamepad, Inputs};
use usb::UsbGamepad;

const FPS: usize = 60;
const BENCHMARK: bool = false;

fn main() -> Result<(), ()> {
    let gilrs = Gilrs::new().unwrap();
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
    watcher.change_file(&watch_file).unwrap();

    let max_gamepads = gilrs.last_gamepad_hint();
    let id = pick_input(max_gamepads, &gilrs);

    let config: Result<config::Gamepad, toml::de::Error> =
        toml::from_str(&fs::read_to_string(&watch_file).unwrap());
    if let Err(e) = config.map(|c| {
        if let Err(e) = gamepad.load::<UsbGamepad>(&c, (Gilrs::new().unwrap(), id)) {
            println!("Failed to initialize backend {e:?}");
        }
    }) {
        println!("Invalid config: {e}\n")
    }

    // } else {
    //     Some(SerialGamepad::new("/dev/ttyACM0", 115200))
    // };

    let options = WindowOptions {
        resize: false,
        scale_mode: ScaleMode::Stretch,
        ..Default::default()
    };

    let mut img = create_image(&gamepad.inputs);
    let mut width = img.width() as usize;
    let mut height = img.height() as usize;
    let mut buf = vec![0u32; width * height];
    gamepad.render(&mut img);
    update_screen(&mut img, &mut buf);
    let mut window = Window::new("Test", width, height, options).unwrap();
    window.set_target_fps(FPS);

    let mut times = 0;
    let mut total = 0u128;
    while window.is_open()
        && !(window.is_key_down(Key::Escape) || window.is_key_down(Key::Q))
    {
        while let Ok(DebouncedEvent { path, kind: DebouncedEventKind::Any }) =
            watcher.rx.try_recv()
        {
            if watch_file == path {
                match toml::from_str(&fs::read_to_string(path).unwrap()) {
                    Ok(config) => {
                        println!("Reloaded config...");
                        gamepad.reload(&config);
                        let bounds = gamepad.inputs.bounds();
                        if width != bounds.right() as usize
                            || height != bounds.bottom() as usize
                        {
                            println!("Resized, making new window...");
                            img = create_image(&gamepad.inputs);
                            width = img.width() as usize;
                            height = img.height() as usize;
                            buf = vec![0u32; width * height];
                            window = Window::new("Test", width, height, options).unwrap();
                            window.set_target_fps(FPS);
                        }
                        gamepad.render(&mut img);
                        update_screen(&mut img, &mut buf);
                    }
                    Err(e) => println!("Config reload failed: {}", e),
                }
            }
        }

        let start = Instant::now();
        if gamepad.poll() || BENCHMARK {
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

// returns selected id
fn pick_input(max_gamepads: usize, gilrs: &Gilrs) -> usize {
    let gamepads: BTreeMap<usize, String> = (0..max_gamepads)
        .filter_map(|i| gilrs.gamepad(i).map(|g| (i, g.name().to_string())))
        .collect();
    println!("\nDetected {} gamepads:", max_gamepads);
    for (id, name) in gamepads {
        println!("{}: {}", id, name);
    }
    serial::print_ports(max_gamepads);
    print!("\nEnter an id: ");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    line.trim().parse().expect("input a number")
}

fn update_screen(img: &mut Pixmap, buf: &mut [u32]) {
    for (pixel, n) in img.pixels_mut().iter().zip(buf.iter_mut()) {
        *n = (pixel.red() as u32) << 16 | (pixel.green() as u32) << 8 | pixel.blue() as u32;
    }
}

fn create_image(inputs: &Inputs) -> Pixmap {
    let bounds = inputs.bounds();
    let width = bounds.right() as usize;
    let height = bounds.bottom() as usize;
    Pixmap::new(width as u32, height as u32).unwrap()
}
