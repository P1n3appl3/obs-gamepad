use crate::config::{self, FillDir};
use gilrs_core::{EvCode, Gilrs};
use log::error;
use tiny_skia::{Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform};

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
    pub active: Color,
    pub inactive: Color,
}

impl ColorPair {
    pub fn new(active: Color, inactive: Color) -> Self {
        Self { active, inactive }
    }

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

#[derive(Clone, Debug, Default)]
pub struct RawAxis {
    pub id: u32,
    pub id_index: u8,
    pub deadzone: Option<u32>,
    pub invert: bool,
    pub current: i32,
    pub range: (i32, i32),
}

impl RawAxis {
    pub fn new(id: u8, invert: bool, g: &gilrs_core::Gamepad) -> Self {
        let id_index = id;
        if let Some(id) = g.axes().get(id_index as usize) {
            let info = g.axis_info(*id).unwrap();
            let id = id.into_u32();
            let deadzone = info.deadzone;
            let current = (info.min + info.max) / 2;
            let range = (info.min, info.max);
            return RawAxis {
                id,
                id_index,
                deadzone,
                invert,
                current,
                range,
            };
        }
        Default::default()
    }

    fn update_id(&mut self, g: &gilrs_core::Gamepad) {
        if let Some(id) = g.axes().get(self.id_index as usize) {
            let info = g.axis_info(*id).unwrap();
            self.id = id.into_u32();
            self.deadzone = info.deadzone;
            self.range = (info.min, info.max);
            self.current = (info.min + info.max) / 2;
        }
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
        let mut bounds = expand(self.path.bounds(), self.displacement);
        if let Some((_, width)) = self.outline {
            bounds = expand(bounds, width)
        }
        if let Some((path, _, width)) = &self.gate {
            bounds = combine(bounds, expand(path.bounds(), *width))
        }
        bounds
    }
}

#[derive(Clone, Debug)]
pub struct Axis {
    pub axis: RawAxis,
    pub path: Rect,
    pub direction: FillDir,
    pub fill: ColorPair,
    pub outline: Option<(Color, f32)>,
}

impl Axis {
    pub fn bounds(&self) -> Rect {
        if let Some((_, width)) = &self.outline {
            expand(self.path, *width)
        } else {
            self.path
        }
    }
}

// TODO
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
    pub connected: bool, // TODO: grey out or something
    pub buttons: Vec<Button>,
    pub sticks: Vec<Stick>,
    pub axes: Vec<Axis>,
}

impl Gamepad {
    pub fn clear(&mut self) {
        self.buttons.clear();
        self.sticks.clear();
        self.axes.clear();
    }

    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.buttons.is_empty() && self.sticks.is_empty() && self.axes.is_empty()
    }

    pub fn load_config(&mut self, gilrs: &mut Gilrs, config: &config::Gamepad) {
        self.clear();
        if let Some(gamepad) = gilrs.gamepad(self.id) {
            for b in &config.buttons {
                self.buttons.push(b.load(gamepad, config));
            }
            for s in &config.sticks {
                self.sticks.push(s.load(gamepad, config));
            }
            for a in &config.axes {
                self.axes.push(a.load(gamepad, config));
            }
            self.minimize()
        } else {
            error!("couldn't get info for gamepad {}", self.id)
        }
    }

    pub fn minimize(&mut self) {
        let bounds = self.bounds();
        let t = Transform::from_translate(-bounds.left(), -bounds.top());
        for b in &mut self.buttons {
            b.path = b.path.clone().transform(t).unwrap();
        }
        for s in &mut self.sticks {
            s.path = s.path.clone().transform(t).unwrap();
            if let Some((path, _, _)) = &mut s.gate {
                *path = path.clone().transform(t).unwrap();
            }
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
        if let Some(gamepad) = gilrs.gamepad(id) {
            self.id = id;
            let buttons = gamepad.buttons();
            for b in &mut self.buttons {
                b.id = buttons
                    .get(b.id_index as usize)
                    .cloned()
                    .map(EvCode::into_u32)
                    .unwrap_or_default();
            }
            for s in &mut self.sticks {
                s.x.update_id(gamepad);
                s.y.update_id(gamepad);
            }
            for a in &mut self.axes {
                a.axis.update_id(gamepad);
            }
        } else {
            error!("couldn't get info for gamepad {}", id);
        }
    }

    pub fn update(&mut self, gilrs: &mut Gilrs) -> bool {
        let mut modified = false;
        while let Some(ev) = gilrs.next_event() {
            if ev.id != self.id {
                continue;
            }
            use gilrs_core::EventType::*;
            modified |= match ev.event {
                ButtonPressed(code) => self.set_button(code.into_u32(), true),
                ButtonReleased(code) => self.set_button(code.into_u32(), false),
                AxisValueChanged(val, code) => self.set_axis(code.into_u32(), val),
                ev @ (Connected | Disconnected) => {
                    self.connected = ev == Connected;
                    true
                }
            }
        }
        modified
    }

    pub fn render(&self, img: &mut Pixmap) {
        let mut stroke = Stroke::default();
        let mut paint = Paint {
            anti_alias: true,
            ..Default::default()
        };
        let f = FillRule::default();
        let t = Transform::default();
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

            // active fill
            let percent = (axis.axis.normalized() + 1.0) / 2.0;
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
            if let Some(rect) = Rect::from_ltrb(left, top, right, bottom) {
                let active_path = PathBuilder::from_rect(rect);
                paint.set_color(axis.fill.active);
                img.fill_path(&active_path, &paint, f, t, None);
            }

            // border
            if let Some((color, weight)) = axis.outline {
                stroke.width = weight;
                paint.set_color(color);
                img.stroke_path(&path, &paint, &stroke, t, None);
            }
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

    fn set_button(&mut self, id: u32, state: bool) -> bool {
        let mut modified = false;
        for b in &mut self.buttons {
            if b.id == id {
                modified |= b.pressed != state;
                b.pressed = state;
            }
        }
        modified
    }

    fn set_axis(&mut self, id: u32, state: i32) -> bool {
        let mut modified = false;
        for axis in &mut self.axes {
            if axis.axis.id == id {
                modified |= axis.axis.current != state;
                axis.axis.current = state;
            }
        }
        for stick in &mut self.sticks {
            if stick.x.id == id {
                stick.x.current = state;
                modified |= state == stick.x.current;
            }
            if stick.y.id == id {
                stick.y.current = state;
                modified |= state == stick.y.current;
            }
        }
        modified
    }

    pub fn bounds(&self) -> Rect {
        self.buttons
            .iter()
            .map(Button::bounds)
            .chain(self.sticks.iter().map(Stick::bounds))
            .chain(self.axes.iter().map(Axis::bounds))
            .reduce(combine)
            .unwrap_or_else(|| Rect::from_ltrb(0.0, 0.0, 100.0, 100.0).unwrap())
    }
}
