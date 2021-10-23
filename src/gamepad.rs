use std::io::{self, Write};

use gilrs_core::Gilrs;
use std::collections::BTreeMap;
use tiny_skia::{
    Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

#[derive(Clone, Debug)]
pub struct Button {
    pub id: u32,
    pub pressed: bool,
    pub path: Path,
    pub fill: (Color, Color),
    pub outline: Option<(Color, Color, f32)>,
}

impl Button {
    pub fn new(id: u32, x: f32, y: f32, r: f32, active: Color, inactive: Color) -> Self {
        let c = PathBuilder::from_circle(x, y, r).unwrap();
        Button {
            id,
            pressed: false,
            path: c,
            fill: (active, inactive),
            outline: None,
            // outline: Some((inactive, inactive, 2.0)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RawAxis {
    pub id: u32,
    pub invert: bool,
    pub current: i32,
    pub range: (i32, i32),
}

#[derive(Clone, Debug)]
pub struct Stick {
    pub x: RawAxis,
    pub y: RawAxis,
    pub path: Path,
    pub fill: (Color, Color),
    pub outline: Option<(Path, Color, f32)>,
}

#[derive(Copy, Clone, Debug)]
pub enum FillDir {
    TopToBottom,
    LeftToRight,
    BottomToTop,
    RightToLeft,
}

#[derive(Clone, Debug)]
pub struct Axis {
    pub axis: RawAxis,
    pub path: Rect,
    pub direction: FillDir,
    pub fill: (Color, Color),
    pub outline: (Color, f32),
}

#[derive(Clone, Debug)]
pub struct Dpad {
    pub x: RawAxis,
    pub y: RawAxis,
    pub up: Path,
    pub down: Path,
    pub left: Path,
    pub right: Path,
    pub fill: (Color, Color),
    pub outline: Option<(Path, Color, f32)>,
}

#[derive(Clone, Default, Debug)]
pub struct Gamepad {
    pub id: usize,
    pub buttons: Vec<Button>,
    pub sticks: Vec<Stick>,
    pub axes: Vec<Axis>,
}

fn remap(from: (f32, f32), to: (f32, f32), s: f32) -> f32 {
    to.0 + (s - from.0) * (to.1 - to.0) / (from.1 - from.0)
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

    pub fn add_debug_inputs(&mut self, gilrs: &mut Gilrs) {
        let spacing = 35;
        let mut y = spacing;

        let gamepad = gilrs.gamepad(self.id).unwrap();
        for button in gamepad.buttons() {
            self.buttons.push(Button::new(
                button.into_u32(),
                30.0,
                y as f32,
                15.0,
                Color::from_rgba8(20, 105, 200, 150),
                Color::from_rgba8(20, 20, 20, 150),
            ));
            y += spacing;
        }
        y = spacing;
        for axis in gamepad.axes() {
            let info = gamepad.axis_info(*axis).unwrap();
            self.axes.push(Axis {
                axis: RawAxis {
                    id: axis.into_u32(),
                    invert: false,
                    current: (info.min + info.max) / 2,
                    range: (info.min, info.max),
                },
                path: Rect::from_xywh(100.0, y as f32, 150.0, 30.0).unwrap(),
                direction: FillDir::LeftToRight,
                fill: (Color::from_rgba8(20, 105, 200, 150), Color::BLACK),
                outline: (Color::from_rgba8(20, 20, 20, 150), 2.0),
            });
            y += spacing;
        }
    }

    pub fn update(&mut self, gilrs: &mut Gilrs) {
        while let Some(ev) = gilrs.next_event() {
            if ev.id != self.id {
                continue;
            }
            use gilrs_core::EventType::*;
            match ev.event {
                ButtonPressed(code) => self.set_button(code.into_u32(), true),
                ButtonReleased(code) => self.set_button(code.into_u32(), false),
                AxisValueChanged(val, code) => {
                    self.set_axis(code.into_u32(), val);
                }
                Connected | Disconnected => {
                    // todo!()
                }
            }
        }
    }

    pub fn render(&self, img: &mut Pixmap) {
        let mut stroke = Stroke::default();
        let mut paint = Paint::default();
        paint.anti_alias = false;

        // img.fill(Color::from_rgba8(50, 50, 50, 127));

        for button in &self.buttons {
            let fill = if button.pressed {
                button.fill.0
            } else {
                button.fill.1
            };
            paint.set_color(fill);
            img.fill_path(
                &button.path,
                &paint,
                FillRule::default(),
                Transform::default(),
                None,
            );
            if let Some((active, inactive, weight)) = button.outline {
                paint.set_color(if button.pressed { active } else { inactive });
                stroke.width = weight;
                img.stroke_path(
                    &button.path,
                    &paint,
                    &stroke,
                    Transform::default(),
                    None,
                );
            }
        }
        for axis in &self.axes {
            let path = PathBuilder::from_rect(axis.path);
            paint.set_color(axis.fill.1);
            img.fill_path(
                &path,
                &paint,
                FillRule::default(),
                Transform::default(),
                None,
            );

            // TODO: fill direction
            let (low, high) = axis.axis.range;
            let partial = remap(
                (low as f32, high as f32),
                (0.0, axis.path.width()),
                axis.axis.current as f32,
            );
            let active_path = PathBuilder::from_rect(
                Rect::from_xywh(
                    axis.path.x(),
                    axis.path.y(),
                    partial,
                    axis.path.height(),
                )
                .unwrap(),
            );
            paint.set_color(axis.fill.0);
            img.fill_path(
                &active_path,
                &paint,
                FillRule::default(),
                Transform::default(),
                None,
            );

            let (color, weight) = axis.outline;
            stroke.width = weight;
            paint.set_color(color);
            img.stroke_path(&path, &paint, &stroke, Transform::default(), None);
        }
        for stick in &self.sticks {}
        // for dpad in &self.dpads {}
    }

    fn set_button(&mut self, id: u32, state: bool) {
        for b in &mut self.buttons {
            if b.id == id {
                b.pressed = state;
                return;
            }
        }
    }

    fn set_axis(&mut self, id: u32, state: i32) {
        for axis in &mut self.axes {
            if axis.axis.id == id {
                axis.axis.current = state;
                return;
            }
        }
    }

    fn set_stick_axis(&mut self, id: u32, state: i32) {
        for stick in &mut self.sticks {
            if stick.x.id == id {
                stick.x.current = state;
                return;
            }
            if stick.y.id == id {
                stick.y.current = state;
                return;
            }
        }
    }
}
