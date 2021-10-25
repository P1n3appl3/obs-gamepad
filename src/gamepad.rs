use std::io::{self, Write};

use crate::config;
use gilrs_core::Gilrs;
use std::collections::BTreeMap;
use tiny_skia::{
    Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

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
pub struct ColorPair {
    active: Color,
    inactive: Color,
}

impl ColorPair {
    pub fn get(&self, active: bool) -> Color {
        if active {
            self.active
        } else {
            self.inactive
        }
    }
}

#[derive(Clone, Debug)]
pub struct Button {
    pub id: u32,
    pub id_index: u8,
    pub pressed: bool,
    pub path: Path,
    pub fill: ColorPair,
    pub outline: Option<(ColorPair, f32)>,
}

impl Button {
    pub fn bounds(&self) -> Rect {
        if let Some((_, width)) = self.outline {
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

impl RawAxis {
    fn new(id: u8, invert: bool, g: &gilrs_core::Gamepad) -> Self {
        let id_index = id;
        let id = g.axes()[id_index as usize];
        let info = g.axis_info(id).unwrap();
        let id = id.into_u32();
        let deadzone = info.deadzone;
        let current = (info.min + info.max) / 2;
        let range = (info.min, info.max);
        RawAxis {
            id,
            id_index,
            deadzone,
            invert,
            current,
            range,
        }
    }

    fn update_id(&mut self, g: &gilrs_core::Gamepad) {
        let id = g.axes()[self.id_index as usize];
        let info = g.axis_info(id).unwrap();
        self.id = id.into_u32();
        self.deadzone = info.deadzone;
        self.range = (info.min, info.max);
        self.current = (info.min + info.max) / 2;
    }

    fn normalized(&self) -> f32 {
        fn remap(from: (f32, f32), to: (f32, f32), s: f32) -> f32 {
            to.0 + (s - from.0) * (to.1 - to.0) / (from.1 - from.0)
        }
        remap(
            (self.range.0 as f32, self.range.1 as f32),
            (-1.0, 1.0),
            self.current as f32,
        ) * if self.invert { -1.0 } else { 1.0 }
    }

    fn active(&self) -> bool {
        let midpoint = (self.range.0 + self.range.1) / 2;
        self.deadzone.map_or(true, |n| {
            self.current > midpoint + n as i32 || self.current < midpoint - n as i32
        })
    }
}

#[derive(Clone, Debug)]
pub struct Stick {
    pub x: RawAxis,
    pub y: RawAxis,
    pub path: Path,
    pub displacement: f32,
    pub fill: ColorPair,
    pub outline: Option<(ColorPair, f32)>,
    pub gate: Option<(Path, ColorPair, f32)>,
}

impl Stick {
    pub fn bounds(&self) -> Rect {
        let bounds = if let Some((_, width)) = self.outline {
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

#[derive(Copy, Clone, Debug, PartialEq)]
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
    pub fill: ColorPair,
    pub outline: (Color, f32),
}

impl Axis {
    pub fn bounds(&self) -> Rect {
        expand(self.path, self.outline.1)
    }
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct Dpad {
    pub x: RawAxis,
    pub y: RawAxis,
    pub up: Path,
    pub down: Path,
    pub left: Path,
    pub right: Path,
    pub fill: ColorPair,
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

    pub fn clear(&mut self) {
        self.buttons.clear();
        self.sticks.clear();
        self.axes.clear();
    }

    pub fn add_debug_inputs(&mut self, gilrs: &mut Gilrs) {
        self.clear();
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
                fill: ColorPair { active, inactive },
                outline: None,
                // outline: Some((inactive, inactive, 2.0)),
            });
        }

        for i in 0..gamepad.axes().len() {
            self.axes.push(Axis {
                axis: RawAxis::new(i as u8, false, &gamepad),
                path: Rect::from_xywh(
                    radius * 2.0 + 10.0,
                    i as f32 * spacing,
                    radius * 10.0,
                    radius * 2.0,
                )
                .unwrap(),
                direction: FillDir::LeftToRight,
                fill: ColorPair {
                    active: Color::from_rgba8(20, 105, 200, 150),
                    inactive: Color::BLACK,
                },
                outline: (Color::from_rgba8(20, 20, 20, 150), 2.0),
            });
        }
    }

    pub fn load_config(&mut self, gilrs: &mut Gilrs, config: &config::Gamepad) {
        self.clear();
        let gamepad = gilrs.gamepad(self.id).unwrap();
        let buttons = gamepad.buttons();
        let global_outline = config.weight.is_some() || config.outline.is_some();
        for b in &config.buttons {
            use config::ButtonShape::*;
            let (x, y) = b.pos;
            self.buttons.push(Button {
                id: buttons[b.id as usize].into_u32(),
                id_index: b.id,
                pressed: false,
                path: match b.shape.unwrap_or(config.button_shape) {
                    Circle { radius } => PathBuilder::from_circle(x, y, radius).unwrap(),
                    RoundedRect {
                        width,
                        height,
                        radius,
                    } => config::rounded_rect(x, y, width, height, radius),
                },
                fill: ColorPair {
                    inactive: b.fill.unwrap_or(config.inactive).into(),
                    active: b.fill_active.unwrap_or(config.active).into(),
                },
                outline: (global_outline
                    || b.weight.is_some()
                    || b.outline.is_some()
                    || b.outline_active.is_some())
                .then(|| {
                    (
                        ColorPair {
                            active: b
                                .outline_active
                                .or(b.outline)
                                .or(config.outline)
                                .unwrap_or_default()
                                .into(),
                            inactive: b
                                .outline
                                .or(config.outline)
                                .unwrap_or_default()
                                .into(),
                        },
                        b.weight.or(config.weight).unwrap_or(2.0),
                    )
                }),
            })
        }

        for s in &config.sticks {
            let (x, y) = s.pos;
            let r = s.radius.unwrap_or(config.stick_radius);
            self.sticks.push(Stick {
                x: RawAxis::new(s.x_axis, s.invert_x, &gamepad),
                y: RawAxis::new(s.y_axis, s.invert_y, &gamepad),
                path: PathBuilder::from_circle(x, y, r).unwrap(),
                displacement: s.displacement.unwrap_or(r * 3.0 / 4.0),
                fill: ColorPair {
                    inactive: s.fill.unwrap_or(config.inactive).into(),
                    active: s.fill_active.unwrap_or(config.active).into(),
                },
                outline: (global_outline
                    || s.outline_weight.is_some()
                    || s.outline.is_some()
                    || s.outline_active.is_some())
                .then(|| {
                    (
                        ColorPair {
                            active: s
                                .outline_active
                                .or(s.outline)
                                .or(config.outline)
                                .unwrap_or_default()
                                .into(),
                            inactive: s
                                .outline
                                .or(config.outline)
                                .unwrap_or_default()
                                .into(),
                        },
                        s.outline_weight.or(config.weight).unwrap_or(2.0),
                    )
                }),
                gate: (s.gate_radius.is_some()
                    || s.gate_weight.is_some()
                    || s.gate.is_some()
                    || s.gate_active.is_some())
                .then(|| {
                    (
                        PathBuilder::from_circle(
                            x,
                            y,
                            s.gate_radius.or(config.gate_radius).unwrap_or(r * 1.5),
                        )
                        .unwrap(),
                        ColorPair {
                            active: s
                                .gate_active
                                .or(s.gate)
                                .or(config.outline)
                                .unwrap_or_default()
                                .into(),
                            inactive: s
                                .gate
                                .or(config.outline)
                                .unwrap_or_default()
                                .into(),
                        },
                        s.gate_weight
                            .or(s.outline_weight)
                            .or(config.weight)
                            .unwrap_or(3.0),
                    )
                }),
            });
        }

        // for a in &config.axes {
        //     // TODO:
        // }

        self.minimize()
    }

    // TODO: custom gamecube config with octagonal gate and x/y/z paths

    pub fn minimize(&mut self) {
        let bounds = self.bounds();
        let t = Transform::from_translate(-bounds.left(), -bounds.top());
        for b in &mut self.buttons {
            b.path = b.path.clone().transform(t).unwrap()
        }
        for s in &mut self.sticks {
            s.path = s.path.clone().transform(t).unwrap()
        }
        for a in &mut self.axes {
            a.path = Rect::from_xywh(
                a.path.x() - bounds.left(),
                a.path.y() - bounds.top(),
                a.path.width(),
                a.path.height(),
            )
            .unwrap()
        }
    }

    pub fn switch_gamepad(&mut self, gilrs: &mut Gilrs, id: usize) {
        let gamepad = gilrs.gamepad(id).unwrap();
        let buttons = gamepad.buttons();
        for b in &mut self.buttons {
            b.id = buttons[b.id_index as usize].into_u32();
        }
        for s in &mut self.sticks {
            s.x.update_id(&gamepad);
            s.y.update_id(&gamepad);
        }
        for a in &mut self.axes {
            a.axis.update_id(&gamepad);
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
                AxisValueChanged(val, code) => self.set_axis(code.into_u32(), val),
                Connected | Disconnected => {
                    // TODO:
                }
            }
        }
    }

    pub fn render(&self, img: &mut Pixmap) {
        let mut stroke = Stroke::default();
        let mut paint = Paint::default();
        let f = FillRule::default();
        let t = Transform::default();
        paint.anti_alias = true;
        img.fill(Color::TRANSPARENT);

        for button in &self.buttons {
            paint.set_color(button.fill.get(button.pressed));
            img.fill_path(&button.path, &paint, f, t, None);

            if let Some((colors, weight)) = &button.outline {
                paint.set_color(colors.get(button.pressed));
                stroke.width = *weight;
                img.stroke_path(&button.path, &paint, &stroke, t, None);
            }
        }

        for axis in &self.axes {
            // background
            let path = PathBuilder::from_rect(axis.path);
            paint.set_color(axis.fill.inactive);
            img.fill_path(&path, &paint, f, t, None);

            // active partial fill
            let mut percent = (axis.axis.normalized() + 1.0) / 2.0;
            use FillDir::*;
            let mut left = axis.path.left();
            let mut top = axis.path.top();
            let mut right = axis.path.right();
            let mut bottom = axis.path.bottom();
            match axis.direction {
                TopToBottom => bottom -= axis.path.height() * percent,
                LeftToRight => right -= axis.path.width() * percent,
                BottomToTop => top += axis.path.height() * (1.0 - percent),
                RightToLeft => left += axis.path.width() * (1.0 - percent),
            };
            let active_path = PathBuilder::from_rect(
                Rect::from_ltrb(left, top, right, bottom).unwrap(),
            );
            paint.set_color(axis.fill.active);
            img.fill_path(&active_path, &paint, f, t, None);

            // border
            let (color, weight) = axis.outline;
            stroke.width = weight;
            paint.set_color(color);
            img.stroke_path(&path, &paint, &stroke, t, None);
        }

        for stick in &self.sticks {
            let x = stick.x.normalized();
            let y = stick.y.normalized();
            let cx = stick.displacement * x * (1.0 - y * y / 2.0).sqrt();
            let cy = stick.displacement * y * (1.0 - x * x / 2.0).sqrt();
            let is_active = stick.x.active() || stick.y.active();

            if let Some((path, color, weight)) = &stick.gate {
                paint.set_color(color.get(is_active));
                stroke.width = *weight;
                img.stroke_path(path, &paint, &stroke, t, None);
            }

            let trans = Transform::from_translate(cx, cy);
            paint.set_color(stick.fill.get(is_active));
            img.fill_path(&stick.path, &paint, f, trans, None);

            if let Some((colors, weight)) = &stick.outline {
                paint.set_color(colors.get(is_active));
                stroke.width = *weight;
                img.stroke_path(&stick.path, &paint, &stroke, trans, None);
            }
        }
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
            }
        }
        for stick in &mut self.sticks {
            if stick.x.id == id {
                stick.x.current = state;
            }
            if stick.y.id == id {
                stick.y.current = state;
            }
        }
    }

    pub fn bounds(&self) -> Rect {
        self.buttons
            .iter()
            .map(Button::bounds)
            .chain(self.sticks.iter().map(Stick::bounds))
            .chain(self.axes.iter().map(Axis::bounds))
            .reduce(|a, b| combine(a, b))
            .unwrap_or(Rect::from_ltrb(0.0, 0.0, 100.0, 100.0).unwrap())
    }
}
