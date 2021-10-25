use std::{f32::consts::SQRT_2, fmt};

use serde::{
    de::{self, Unexpected, Visitor},
    Deserialize, Deserializer,
};
use tiny_skia::{self, Path, PathBuilder};

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
        Self::new(
            (n >> 4 & 0xf0) as u8,
            (n & 0xf0) as u8,
            (n << 4) as u8,
            0xff,
        )
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
                if !value.starts_with("#") {
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
                    rgb if rgb.len() == 3 => u32::from_str_radix(rgb, 16)
                        .map(Color::from_rgb_packed)
                        .or(err),
                    rgba if rgba.len() == 8 => {
                        u32::from_str_radix(rgba, 16).map(Color::from_rgba).or(err)
                    }
                    rgba if rgba.len() == 4 => u32::from_str_radix(rgba, 16)
                        .map(Color::from_rgba_packed)
                        .or(err),
                    _ => err,
                }
            }
        }
        deserializer.deserialize_str(ColorVisitor)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ButtonShape {
    Circle {
        radius: f32,
    },
    RoundedRect {
        width: f32,
        height: f32,
        radius: f32,
    },
}

impl Default for ButtonShape {
    fn default() -> Self {
        Self::Circle { radius: 40.0 }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Button {
    pub id: u8,
    pub pos: (f32, f32),
    pub shape: Option<ButtonShape>,
    pub fill: Option<Color>,
    pub fill_active: Option<Color>,
    pub weight: Option<f32>,
    pub outline: Option<Color>,
    pub outline_active: Option<Color>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Stick {
    pub pos: (f32, f32),
    pub x_axis: u8,
    pub y_axis: u8,
    #[serde(default)]
    pub invert_x: bool,
    #[serde(default)]
    pub invert_y: bool,
    pub radius: Option<f32>,
    pub displacement: Option<f32>,
    pub fill: Option<Color>,
    pub fill_active: Option<Color>,
    pub outline_weight: Option<f32>,
    pub outline: Option<Color>,
    pub outline_active: Option<Color>,
    pub gate_radius: Option<f32>,
    pub gate: Option<Color>,
    pub gate_active: Option<Color>,
    pub gate_weight: Option<f32>,
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

#[derive(Clone, Debug, Deserialize)]
pub struct Gamepad {
    #[serde(default = "default_active")]
    pub active: Color,
    #[serde(default = "default_fill")]
    pub inactive: Color,
    #[serde(default)]
    pub outline: Option<Color>,
    #[serde(default)]
    pub weight: Option<f32>,
    #[serde(default)]
    pub button_shape: ButtonShape,
    #[serde(default = "default_stick")]
    pub stick_radius: f32,
    #[serde(default)]
    pub gate_radius: Option<f32>,
    #[serde(default, rename = "button")]
    pub buttons: Vec<Button>,
    #[serde(default, rename = "stick")]
    pub sticks: Vec<Stick>,
}
