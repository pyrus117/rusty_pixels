use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    Rgba,
    Grayscale,
    Indexed,
}

impl ColorMode {
    /// 4, 2 or 1.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            ColorMode::Rgba => 4,
            ColorMode::Grayscale => 2,
            ColorMode::Indexed => 1,
        }
    }

    /// 32, 16 or 8: the header's colour-depth field.
    pub fn depth_bits(self) -> u16 {
        match self {
            ColorMode::Rgba => 32,
            ColorMode::Grayscale => 16,
            ColorMode::Indexed => 8,
        }
    }

    /// Inverse of depth_bits; None for any other value.
    pub fn from_depth_bits(bits: u16) -> Option<ColorMode> {
        match bits {
            32 => Some(ColorMode::Rgba),
            16 => Some(ColorMode::Grayscale),
            8 => Some(ColorMode::Indexed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PixelValue {
    Rgba { r: u8, g: u8, b: u8, a: u8 },
    Grayscale { v: u8, a: u8 },
    Indexed { index: u8 },
}

impl PixelValue {
    pub fn mode(&self) -> ColorMode {
        match self {
            PixelValue::Rgba { .. } => ColorMode::Rgba,
            PixelValue::Grayscale { .. } => ColorMode::Grayscale,
            PixelValue::Indexed { .. } => ColorMode::Indexed,
        }
    }

    /// Bytes in file order (RGBA / value,alpha / index); slice length = bytes_per_pixel.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PixelValue::Rgba { r, g, b, a } => vec![*r, *g, *b, *a],
            PixelValue::Grayscale { v, a } => vec![*v, *a],
            PixelValue::Indexed { index } => vec![*index],
        }
    }
}
