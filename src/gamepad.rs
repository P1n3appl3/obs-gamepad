use std::io::{self, Write};

use gilrs_core::Gilrs;
use std::collections::BTreeMap;
use tiny_skia::{
    Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

fn remap(from: (f32, f32), to: (f32, f32), s: f32) -> f32 {
    to.0 + (s - from.0) * (to.1 - to.0) / (from.1 - from.0)
}

fn combine(a: Rect, b: Rect) -> Rect {
    Rect::from_ltrb(
        a.left().min(b.left()),
        a.top().min(b.top()),
        a.right().max(b.right()),
        a.bottom().max(b.bottom()),
    )
    .unwrap()
}

fn expand(r: Rect, f: f32) -> Rect {
    Rect::from_ltrb(r.left() - f, r.top() - f, r.right() + f, r.bottom() + f).unwrap()
}

#[derive(Clone, Debug)]
pub struct Button {
    pub id: u32,
    pub id_index: u8,
    pub pressed: bool,
    pub path: Path,
    pub fill: (Color, Color),
    pub outline: Option<(Color, Color, f32)>,
}

impl Button {
    pub fn bounds(&self) -> Rect {
        if let Some((_, _, width)) = self.outline {
            expand(self.path.bounds(), width)
        } else {
            self.path.bounds()
        }
    }
}

#[derive(Clone, Debug)]
pub struct RawAxis {
    pub id: u32,
    pub id_index: u8,
    pub deadzone: Option<u32>,
    pub invert: bool,
    pub current: i32,
    pub range: (i32, i32),
}

#[derive(Clone, Debug)]
pub struct Stick {
    pub x: RawAxis,
    pub y: RawAxis,
    pub path: Path,
    pub displacement: f32,
    pub fill: (Color, Color),
    pub outline: Option<(Color, Color, f32)>,
    pub gate: Option<(Path, Color, f32)>,
}

impl Stick {
    pub fn bounds(&self) -> Rect {
        let bounds = if let Some((_, _, width)) = self.outline {
            expand(self.path.bounds(), width)
        } else {
            self.path.bounds()
        };
        if let Some((path, _, width)) = &self.gate {
            combine(expand(path.bounds(), *width), bounds)
        } else {
            bounds
        }
    }
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

impl Axis {
    pub fn bounds(&self) -> Rect {
        expand(self.path, self.outline.1)
    }
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
        let spacing = 35.0;
        let radius = 15.0;

        let gamepad = gilrs.gamepad(self.id).unwrap();
        for (i, button) in gamepad.buttons().into_iter().enumerate() {
            let c = PathBuilder::from_circle(radius, i as f32 * spacing + radius, radius)
                .unwrap();
            let active = Color::from_rgba8(20, 105, 200, 150);
            let inactive = Color::from_rgba8(20, 20, 20, 150);
            self.buttons.push(Button {
                id: button.into_u32(),
                id_index: 0,
                pressed: false,
                path: c,
                fill: (active, inactive),
                outline: None,
                // outline: Some((inactive, inactive, 2.0)),
            });
        }

        for (i, axis) in gamepad.axes().into_iter().enumerate() {
            let info = gamepad.axis_info(*axis).unwrap();
            self.axes.push(Axis {
                axis: RawAxis {
                    id: axis.into_u32(),
                    id_index: 0,
                    deadzone: info.deadzone,
                    invert: false,
                    current: (info.min + info.max) / 2,
                    range: (info.min, info.max),
                },
                path: Rect::from_xywh(
                    radius * 2.0 + 10.0,
                    i as f32 * spacing,
                    radius * 10.0,
                    radius * 2.0,
                )
                .unwrap(),
                direction: FillDir::LeftToRight,
                fill: (Color::from_rgba8(20, 105, 200, 150), Color::BLACK),
                outline: (Color::from_rgba8(20, 20, 20, 150), 2.0),
            });
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
            let (low, high) = axis.axis.range;
            let active_path = match axis.direction {
                FillDir::TopToBottom => todo!(),
                FillDir::LeftToRight => {
                    let partial = remap(
                        (low as f32, high as f32),
                        (0.0, axis.path.width()),
                        axis.axis.current as f32,
                    );
                    PathBuilder::from_rect(
                        Rect::from_xywh(
                            axis.path.x(),
                            axis.path.y(),
                            partial,
                            axis.path.height(),
                        )
                        .unwrap(),
                    )
                }
                FillDir::BottomToTop => todo!(),
                FillDir::RightToLeft => todo!(),
            };
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

    pub fn bounds(&self) -> Rect {
        let smallest = Rect::from_ltrb(0.0, 0.0, 1.0, 1.0).unwrap();
        [
            self.buttons
                .iter()
                .map(Button::bounds)
                .reduce(|a, b| combine(a, b))
                .unwrap_or(smallest),
            self.sticks
                .iter()
                .map(Stick::bounds)
                .reduce(|a, b| combine(a, b))
                .unwrap_or(smallest),
            self.axes
                .iter()
                .map(Axis::bounds)
                .reduce(|a, b| combine(a, b))
                .unwrap_or(smallest),
        ]
        .into_iter()
        .reduce(|a, b| combine(a, b))
        .unwrap()
    }
}
