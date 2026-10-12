use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::user_data::UserData;

/// Stable identity. Never reused within one Engine session (see rp-ops); on file load it equals
/// the layer's index in file order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct LayerId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayerFlags(pub u16);

impl LayerFlags {
    pub const VISIBLE: u16 = 1;
    pub const EDITABLE: u16 = 2;
    pub const LOCK_MOVEMENT: u16 = 4;
    pub const BACKGROUND: u16 = 8;
    pub const PREFER_LINKED_CELS: u16 = 16;
    pub const COLLAPSED: u16 = 32;
    pub const REFERENCE: u16 = 64;

    pub fn contains(self, bit: u16) -> bool {
        self.0 & bit == bit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    Image,
    Group,
    Tilemap { tileset_index: u32 },
}

/// Discriminants are the file-format values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
    Addition = 16,
    Subtract = 17,
    Divide = 18,
}

impl BlendMode {
    pub fn from_u16(v: u16) -> Option<BlendMode> {
        match v {
            0 => Some(BlendMode::Normal),
            1 => Some(BlendMode::Multiply),
            2 => Some(BlendMode::Screen),
            3 => Some(BlendMode::Overlay),
            4 => Some(BlendMode::Darken),
            5 => Some(BlendMode::Lighten),
            6 => Some(BlendMode::ColorDodge),
            7 => Some(BlendMode::ColorBurn),
            8 => Some(BlendMode::HardLight),
            9 => Some(BlendMode::SoftLight),
            10 => Some(BlendMode::Difference),
            11 => Some(BlendMode::Exclusion),
            12 => Some(BlendMode::Hue),
            13 => Some(BlendMode::Saturation),
            14 => Some(BlendMode::Color),
            15 => Some(BlendMode::Luminosity),
            16 => Some(BlendMode::Addition),
            17 => Some(BlendMode::Subtract),
            18 => Some(BlendMode::Divide),
            _ => None,
        }
    }

    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub parent: Option<LayerId>,
    pub name: String,
    pub kind: LayerKind,
    pub flags: LayerFlags,
    pub blend_mode: BlendMode,
    pub opacity: u8,
    pub uuid: Option<[u8; 16]>,
    pub user_data: UserData,
}
