use std::fmt::Debug;

use tiny_skia::{
    Color, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

use crate::config::{self, FillDir};

#[derive(Debug, Default)]
pub struct Gamepad<'b> {
    pub backend: Option<Box<dyn Backend + 'b>>,
    pub inputs: Inputs,
    pub input_state: InputState,
}

impl<'b> Gamepad<'b> {
    #[allow(dead_code)]
    fn new(config: &config::Gamepad) -> Self {
        let inputs: Inputs = config.into();
        let input_state = (&inputs).into();
        Self { backend: None, inputs, input_state }
    }

    pub fn reload(&mut self, config: &config::Gamepad) {
        self.inputs = config.into();
        self.input_state = (&self.inputs).into();
        if let Some(b) = &mut self.backend {
            b.reload(&self.inputs)
        }
    }

    #[allow(dead_code)]
    pub fn load<B: Backend + 'b>(
        &mut self,
        config: &config::Gamepad,
        state: B::InitState,
    ) -> Result<(), B::Err> {
        self.inputs = config.into();
        self.input_state = (&self.inputs).into();
        self.backend = Some(Box::new(B::init(state, &self.inputs)?));
        Ok(())
    }

    pub fn poll(&mut self) -> bool {
        self.backend.as_mut().map(|b| b.poll(&mut self.input_state)).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Inputs {
    pub buttons: Vec<Button>,
    pub sticks: Vec<Stick>,
    pub axes: Vec<Axis>,
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub buttons: Vec<bool>,
    pub sticks: Vec<(f32, f32)>,
    pub axes: Vec<f32>,
}

pub trait Backend: Debug {
    type InitState
    where
        Self: Sized;
    type Err: Debug + Send + Sync + 'static
    where
        Self: Sized;

    fn init(state: Self::InitState, inputs: &Inputs) -> Result<Self, Self::Err>
    where
        Self: Sized;

    fn poll(&mut self, state: &mut InputState) -> bool;

    fn reload(&mut self, inputs: &Inputs);
}

impl Inputs {
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

#[derive(Clone, Debug)]
pub struct Button {
    pub id: u8,
    pub path: Path,
    pub fill: ColorPair,
    pub outline: Option<(ColorPair, f32)>,
}

#[derive(Clone, Debug)]
pub struct Axis {
    pub axis: RawAxis,
    pub path: Rect,
    pub direction: FillDir,
    pub fill: ColorPair,
    pub outline: Option<(Color, f32)>,
}

#[derive(Clone, Debug)]
pub struct Stick {
    pub x: RawAxis,
    pub y: RawAxis,
    pub deadzone: f32,
    pub path: Path,
    pub displacement: f32,
    pub fill: ColorPair,
    pub outline: Option<(ColorPair, f32)>,
    pub gate: Option<(Path, ColorPair, f32)>,
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

#[derive(Clone, Debug, Default)]
pub struct RawAxis {
    pub id: u8,
    pub invert: bool,
}

impl From<&config::Gamepad> for Inputs {
    fn from(config: &config::Gamepad) -> Self {
        let mut temp = Self {
            buttons: config.buttons.iter().map(|b| b.load(config)).collect(),
            axes: config.axes.iter().map(|b| b.load(config)).collect(),
            sticks: config.sticks.iter().map(|b| b.load(config)).collect(),
        };
        temp.minimize();
        temp
    }
}

impl Gamepad<'_> {
    pub fn render(&self, img: &mut Pixmap) {
        let mut stroke = Stroke::default();
        let mut paint = Paint { anti_alias: true, ..Default::default() };
        let f = FillRule::default();
        let t = Transform::default();
        img.fill(Color::TRANSPARENT);

        for (button, &pressed) in self.inputs.buttons.iter().zip(&self.input_state.buttons)
        {
            paint.set_color(button.fill.get(pressed));
            img.fill_path(&button.path, &paint, f, t, None);

            if let Some((colors, weight)) = &button.outline {
                paint.set_color(colors.get(pressed));
                stroke.width = *weight;
                img.stroke_path(&button.path, &paint, &stroke, t, None);
            }
        }

        for (axis, &percent) in self.inputs.axes.iter().zip(&self.input_state.axes) {
            let percent = if axis.axis.invert { 1.0 - percent } else { percent };
            // background
            let path = PathBuilder::from_rect(axis.path);
            paint.set_color(axis.fill.inactive);
            img.fill_path(&path, &paint, f, t, None);

            // active fill
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

        for (stick, &(x, y)) in self.inputs.sticks.iter().zip(&self.input_state.sticks) {
            let deadzone = stick.deadzone;
            let is_active =
                !(-deadzone < x && x < deadzone && -deadzone < y && y < deadzone);
            let x = if stick.x.invert { -x } else { x };
            let y = if stick.x.invert { -y } else { y };
            let cx = stick.displacement * x * (1.0 - y * y / 2.0).sqrt();
            let cy = stick.displacement * y * (1.0 - x * x / 2.0).sqrt();

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

impl Button {
    pub fn bounds(&self) -> Rect {
        if let Some((_, width)) = self.outline {
            expand(self.path.bounds(), width)
        } else {
            self.path.bounds()
        }
    }
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

impl Axis {
    pub fn bounds(&self) -> Rect {
        if let Some((_, width)) = &self.outline {
            expand(self.path, *width)
        } else {
            self.path
        }
    }
}

impl From<&Inputs> for InputState {
    fn from(inputs: &Inputs) -> Self {
        Self {
            buttons: vec![false; inputs.buttons.len()],
            axes: vec![0.5; inputs.axes.len()],
            sticks: vec![Default::default(); inputs.sticks.len()],
        }
    }
}
