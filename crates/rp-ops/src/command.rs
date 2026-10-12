use rp_core::{LayerId, PixelValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PixelWrite {
    pub x: i32,
    pub y: i32,
    pub value: PixelValue,
}

/// Write pixels (sprite coordinates) into an image layer's cel at `frame`, creating or growing it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SetPixels {
    pub layer: LayerId,
    pub frame: u32,
    pub pixels: Vec<PixelWrite>,
}

/// Add an image layer. Placed directly above `above` (same parent), or at the top of the root stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AddLayer {
    pub name: String,
    #[serde(default)]
    pub above: Option<LayerId>,
}

/// Remove a layer; a group's descendants and all their cels go with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RemoveLayer {
    pub layer: LayerId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RenameLayer {
    pub layer: LayerId,
    pub name: String,
}

/// Append a frame at the end.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AddFrame {
    pub duration_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SetFrameDuration {
    pub frame: u32,
    pub duration_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SetPaletteColor {
    pub index: u32,
    pub rgba: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, strum::EnumDiscriminants)]
#[serde(tag = "op", rename_all = "snake_case")]
#[strum_discriminants(name(CommandKind), derive(strum::EnumIter, strum::IntoStaticStr, Hash))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
pub enum Command {
    SetPixels(SetPixels),
    AddLayer(AddLayer),
    RemoveLayer(RemoveLayer),
    RenameLayer(RenameLayer),
    AddFrame(AddFrame),
    SetFrameDuration(SetFrameDuration),
    SetPaletteColor(SetPaletteColor),
}

impl Command {
    pub fn kind(&self) -> CommandKind {
        match self {
            Command::SetPixels(_) => CommandKind::SetPixels,
            Command::AddLayer(_) => CommandKind::AddLayer,
            Command::RemoveLayer(_) => CommandKind::RemoveLayer,
            Command::RenameLayer(_) => CommandKind::RenameLayer,
            Command::AddFrame(_) => CommandKind::AddFrame,
            Command::SetFrameDuration(_) => CommandKind::SetFrameDuration,
            Command::SetPaletteColor(_) => CommandKind::SetPaletteColor,
        }
    }
}
