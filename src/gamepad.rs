use std::io;
use std::io::Write;

use gilrs_core::Gilrs;
use std::collections::BTreeMap;
use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Clone, Default, Debug)]
pub struct Stick {
    pub x_id: u32,
    pub y_id: u32,
    pub button_id: u32,
    pub invert_x: bool,
    pub invert_y: bool,
    pub x_val: i32,
    pub y_val: i32,
    pub pressed: bool,
    pub x_pos: f32,
    pub y_pos: f32,
    pub radius: f32,
    pub border_radius: f32,
}

#[derive(Clone, Debug)]
pub struct Button {
    pub id: u32,
    pub pressed: bool,
    pub path: Path,
    pub background: Color,
    pub active_background: Color,
    pub outline: Option<Color>,
    pub active_outline: Option<Color>,
}

impl Button {
    pub fn new(id: u32, x: f32, y: f32, r: f32, active: Color, inactive: Color) -> Self {
        let c = PathBuilder::from_circle(x, y, r).unwrap();
        Button {
            id,
            pressed: false,
            path: c,
            background: inactive,
            active_background: active,
            outline: None,
            active_outline: None,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Axis {
    // TODO (analog triggers)
}

#[derive(Clone, Default, Debug)]
pub struct Gamepad {
    pub id: usize,
    pub buttons: Vec<Button>,
    pub sticks: Vec<Stick>,
    pub axis: Vec<Axis>,
}

impl Gamepad {
    pub fn new(gilrs: &mut Gilrs) -> Self {
        let max_gamepads = gilrs.last_gamepad_hint();
        let gamepads: BTreeMap<usize, String> = (0..max_gamepads)
            .filter_map(|i| gilrs.gamepad(i).map(|g| (i, g.name().to_string())))
            .collect();
        for (id, name) in gamepads {
            println!("{}: {}", id, name);
        }
        print!("\nEnter an id: ");
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        Gamepad {
            id: line.trim().parse().unwrap(),
            ..Default::default()
        }
    }

    pub fn update(&mut self, gilrs: &mut Gilrs) {
        while let Some(ev) = gilrs.next_event() {
            if ev.id != self.id {
                println!("extraneous event: {:?}", ev.event);
                continue;
            }
            println!("{:?}", ev.event);
            use gilrs_core::EventType::*;
            match ev.event {
                ButtonPressed(code) => self.set_button(code.into_u32(), true),
                ButtonReleased(code) => self.set_button(code.into_u32(), false),
                AxisValueChanged(val, code) => {
                    // todo!()
                }
                Connected => {
                    // todo!()
                }
                Disconnected => {
                    // todo!()
                }
            }
        }
    }

    pub fn render(&self, img: &mut Pixmap) {
        let mut paint = Paint::default();

        // img.fill(Color::from_rgba8(50, 50, 50, 127));

        for button in &self.buttons {
            let fill = if button.pressed {
                button.active_background
            } else {
                button.background
            };
            paint.set_color(fill);
            img.fill_path(
                &button.path,
                &paint,
                FillRule::default(),
                Transform::default(),
                None,
            );
            // img.stroke_path(&c, &paint, &Stroke::default(),
            // Transform::default(), None);
        }
        for stick in &self.sticks {}
        for axis in &self.axis {}
    }

    fn set_button(&mut self, id: u32, state: bool) {
        for b in &mut self.buttons {
            if b.id == id {
                b.pressed = state;
                return;
            }
        }
    }
}
