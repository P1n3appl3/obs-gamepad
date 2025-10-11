use std::{
    f32::consts::SQRT_2,
    fmt, path,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use notify_debouncer_mini::{
    DebounceEventResult, DebouncedEvent, Debouncer, new_debouncer,
    notify::{self, RecommendedWatcher, RecursiveMode},
};
use serde::{
    Deserialize, Deserializer,
    de::{self, Unexpected, Visitor},
};
use tiny_skia::{self, Path, PathBuilder};

use crate::gamepad::{self, ColorPair};

pub struct ConfigWatcher {
    pub debouncer: Debouncer<RecommendedWatcher>,
    pub rx: Receiver<DebouncedEvent>,
    pub path: Option<PathBuf>,
}

impl ConfigWatcher {
    pub fn new(delay: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            debouncer: new_debouncer(delay, move |e: DebounceEventResult| {
                if let Ok(events) = e {
                    for e in events {
                        tx.send(e).ok();
                    }
                }
            })
            .unwrap(),
            rx,
            path: None,
        }
    }

    pub fn change_file<P: AsRef<path::Path>>(&mut self, path: P) -> notify::Result<()> {
        let path = path.as_ref();
        println!("{:?}", path);
        if let Some(current) = &self.path {
            if current.as_path() == path {
                return Ok(());
            }
            self.debouncer.watcher().unwatch(current.parent().unwrap())?;
        }
        self.path = Some(path.into());
        self.debouncer.watcher().watch(path.parent().unwrap(), RecursiveMode::Recursive)
    }
}

#[rustfmt::skip]
pub fn rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Path {
    const K: f32 = 4.0 * (SQRT_2 - 1.0) / 3.0; // maths
    let left = x;
    let top = y;
    let right = x + width;
    let bottom = y + height;
    let mut path = PathBuilder::new();
    path.move_to(left + radius, top);
    path.line_to(right - radius, top);
    path.cubic_to(
        right + radius * (K - 1.0), top,
        right, top + radius * (1.0 - K),
        right, top + radius,
    );
    path.line_to(right, bottom - radius);
    path.cubic_to(
        right, bottom + radius * (K - 1.0),
        right + radius * (K - 1.0), bottom,
        right - radius, bottom,
    );
    path.line_to(left + radius, bottom);
    path.cubic_to(
        left + radius * (1.0 - K), bottom,
        left, bottom + radius * (K - 1.0),
        left, bottom - radius,
    );
    path.line_to(left, top + radius);
    path.cubic_to(
        left, top + radius * (1.0 - K),
        left + radius * (1.0 - K), top,
        left + radius, top,
    );
    path.finish().unwrap()
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Color::new(0, 0, 0, 0xff)
    }
}

impl From<Color> for tiny_skia::Color {
    fn from(c: Color) -> Self {
        tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)
    }
}

impl Color {
    const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    // #rrggbb
    const fn from_rgb(n: u32) -> Self {
        Self::new((n >> 16) as u8, (n >> 8) as u8, n as u8, 0xff)
    }
    // #rrggbbaa
    const fn from_rgba(n: u32) -> Self {
        Self::new((n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, n as u8)
    }
    // #rgb
    const fn from_rgb_packed(n: u32) -> Self {
        Self::new((n >> 4 & 0xf0) as u8, (n & 0xf0) as u8, (n << 4) as u8, 0xff)
    }
    // #rgba
    const fn from_rgba_packed(n: u32) -> Self {
        Self::new(
            (n >> 8 & 0xf0) as u8,
            (n >> 4 & 0xf0) as u8,
            (n & 0xf0) as u8,
            (n << 4) as u8,
        )
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;
        impl<'de> Visitor<'de> for ColorVisitor {
            type Value = Color;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("`secs` or `nanos`")
            }

            fn visit_str<E>(self, value: &str) -> Result<Color, E>
            where
                E: de::Error,
            {
                if !value.starts_with('#') {
                    return Err(de::Error::invalid_value(
                        Unexpected::Str(value),
                        &"Hex code starting with a '#'",
                    ));
                }
                let err = Err(de::Error::invalid_value(
                    Unexpected::Str(value),
                    &"RGB or RGBA hex code",
                ));
                match &value[1..] {
                    rgb if rgb.len() == 6 => {
                        u32::from_str_radix(rgb, 16).map(Color::from_rgb).or(err)
                    }
                    rgb if rgb.len() == 3 => {
                        u32::from_str_radix(rgb, 16).map(Color::from_rgb_packed).or(err)
                    }
                    rgba if rgba.len() == 8 => {
                        u32::from_str_radix(rgba, 16).map(Color::from_rgba).or(err)
                    }
                    rgba if rgba.len() == 4 => {
                        u32::from_str_radix(rgba, 16).map(Color::from_rgba_packed).or(err)
                    }
                    _ => err,
                }
            }
        }
        deserializer.deserialize_str(ColorVisitor)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Shape {
    RoundedRect { size: (f32, f32), radius: Option<f32> },
    Circle { radius: f32 },
}

impl Default for Shape {
    fn default() -> Self {
        Self::Circle { radius: 15.0 }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Button {
    pub id: u8,
    pub pos: (f32, f32),
    pub shape: Option<Shape>,
    pub fill: Option<Color>,
    pub fill_active: Option<Color>,
    pub outline_weight: Option<f32>,
    pub outline: Option<Color>,
    pub outline_active: Option<Color>,
}

impl Button {
    pub fn load(&self, config: &Gamepad) -> gamepad::Button {
        let outline_active = self
            .outline_active
            .or(self.outline)
            .or(config.outline)
            .unwrap_or_default()
            .into();
        let outline_inactive = self.outline.or(config.outline).unwrap_or_default().into();
        let weight = self.outline_weight.or(config.outline_weight).unwrap_or(2.0);

        let (x, y) = self.pos;
        use Shape::*;
        gamepad::Button {
            id: self.id,
            path: match self.shape.unwrap_or(config.button_shape) {
                Circle { radius } => PathBuilder::from_circle(x, y, radius).unwrap(),
                RoundedRect { size, radius } => {
                    rounded_rect(x, y, size.0, size.1, radius.unwrap_or_default())
                }
            },
            fill: ColorPair {
                inactive: self.fill.unwrap_or(config.inactive).into(),
                active: self.fill_active.unwrap_or(config.active).into(),
            },
            outline: (config.default_outline()
                || self.outline_weight.is_some()
                || self.outline.is_some()
                || self.outline_active.is_some())
            .then(|| (ColorPair::new(outline_active, outline_inactive), weight)),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stick {
    pub pos: (f32, f32),
    pub axes: (u8, u8),
    #[serde(default)]
    pub invert_x: bool,
    #[serde(default)]
    pub invert_y: bool,
    pub deadzone: Option<f32>,
    pub radius: Option<f32>,
    pub displacement: Option<f32>,
    pub fill: Option<Color>,
    pub fill_active: Option<Color>,
    pub outline_weight: Option<f32>,
    pub outline: Option<Color>,
    pub outline_active: Option<Color>,
    pub gate_radius: Option<f32>,
    pub gate_weight: Option<f32>,
    pub gate: Option<Color>,
    pub gate_active: Option<Color>,
}

impl Stick {
    pub fn load(&self, config: &Gamepad) -> gamepad::Stick {
        let r = self.radius.unwrap_or(config.stick_radius);
        let outline_active = self
            .outline_active
            .or(self.outline)
            .or(config.outline)
            .unwrap_or_default()
            .into();
        let outline_inactive = self.outline.or(config.outline).unwrap_or_default().into();
        let outline_weight = self.outline_weight.or(config.outline_weight).unwrap_or(2.0);
        let (x, y) = self.pos;
        let gate_path = PathBuilder::from_circle(
            x,
            y,
            self.gate_radius.or(config.gate_radius).unwrap_or(r * 1.5),
        )
        .unwrap();
        let gate_active =
            self.gate_active.or(self.gate).or(config.outline).unwrap_or_default().into();
        let gate_inactive = self.gate.or(config.outline).unwrap_or_default().into();
        let gate_weight = self
            .gate_weight
            .or(self.outline_weight)
            .or(config.outline_weight)
            .unwrap_or(4.0);

        gamepad::Stick {
            x: gamepad::RawAxis { id: self.axes.0, invert: self.invert_x },
            y: gamepad::RawAxis { id: self.axes.1, invert: self.invert_y },
            deadzone: self.deadzone.unwrap_or(0.05),
            path: PathBuilder::from_circle(x, y, r).unwrap(),
            displacement: self.displacement.unwrap_or(r * 3.0 / 4.0),
            fill: ColorPair {
                inactive: self.fill.unwrap_or(config.inactive).into(),
                active: self.fill_active.unwrap_or(config.active).into(),
            },
            outline: (config.default_outline()
                || self.outline_weight.is_some()
                || self.outline.is_some()
                || self.outline_active.is_some())
            .then(|| (ColorPair::new(outline_active, outline_inactive), outline_weight)),
            gate: (self.gate_radius.is_some()
                || self.gate_weight.is_some()
                || self.gate.is_some()
                || self.gate_active.is_some())
            .then(|| (gate_path, ColorPair::new(gate_active, gate_inactive), gate_weight)),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FillDir {
    TopToBottom,
    #[default]
    LeftToRight,
    BottomToTop,
    RightToLeft,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    pub pos: (f32, f32),
    pub id: u8,
    #[serde(default)]
    pub invert: bool,
    pub shape: Option<Shape>,
    pub fill_dir: Option<FillDir>,
    pub fill: Option<Color>,
    pub fill_active: Option<Color>,
    pub outline_weight: Option<f32>,
    pub outline: Option<Color>,
}

impl Axis {
    pub fn load(&self, config: &Gamepad) -> gamepad::Axis {
        let axis = gamepad::RawAxis { id: self.id, invert: self.invert };

        let (x, y) = self.pos;
        use Shape::*;
        let path = match self.shape.unwrap_or(config.axis_shape) {
            Circle { radius } => PathBuilder::from_circle(x, y, radius).unwrap(),
            RoundedRect { size, radius } => {
                rounded_rect(x, y, size.0, size.1, radius.unwrap_or_default())
            }
        };

        let direction = self.fill_dir.unwrap_or(config.fill_dir);

        let fill = ColorPair {
            inactive: self.fill.unwrap_or(config.inactive).into(),
            active: self.fill_active.unwrap_or(config.active).into(),
        };

        let outline_weight = self.outline_weight.or(config.outline_weight).unwrap_or(2.0);
        let outline = (config.default_outline()
            || self.outline.is_some()
            || self.outline_weight.is_some())
        .then(|| {
            (
                self.outline.unwrap_or_else(|| config.outline.unwrap_or_default()).into(),
                outline_weight,
            )
        });

        gamepad::Axis { axis, path, direction, fill, outline }
    }
}

const fn default_fill() -> Color {
    Color::new(127, 127, 127, 127)
}

const fn default_active() -> Color {
    Color::new(34, 59, 224, 200)
}

const fn default_stick() -> f32 {
    40.0
}

const fn default_axis() -> Shape {
    Shape::RoundedRect { size: (120.0, 20.0), radius: Some(5.0) }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gamepad {
    #[serde(default = "default_active")]
    pub active: Color,
    #[serde(default = "default_fill")]
    pub inactive: Color,
    #[serde(default)]
    pub outline: Option<Color>,
    #[serde(default)]
    pub outline_weight: Option<f32>,
    #[serde(default)]
    pub button_shape: Shape,
    #[serde(default = "default_axis")]
    pub axis_shape: Shape,
    #[serde(default = "default_stick")]
    pub stick_radius: f32,
    #[serde(default)]
    pub gate_radius: Option<f32>,
    #[serde(default)]
    pub fill_dir: FillDir,
    #[serde(default)]
    pub buttons: Vec<Button>,
    #[serde(default)]
    pub sticks: Vec<Stick>,
    #[serde(default)]
    pub axes: Vec<Axis>,
}

impl Gamepad {
    pub fn default_outline(&self) -> bool {
        self.outline_weight.is_some() || self.outline.is_some()
    }
}
