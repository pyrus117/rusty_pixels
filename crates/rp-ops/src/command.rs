use rp_core::{LayerId, PixelValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PixelWrite {
    pub x: i32,
    pub y: i32,
    pub value: PixelValue,
}

/// Write pixels (sprite coordinates) into an image layer's cel at `frame`, creating or growing it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetPixels {
    pub layer: LayerId,
    pub frame: u32,
    pub pixels: Vec<PixelWrite>,
}

/// Add an image layer. Placed directly above `above` (same parent), or at the top of the root stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddLayer {
    pub name: String,
    #[serde(default)]
    pub above: Option<LayerId>,
    /// Explicit layer UUID; generated and filled into the log when absent.
    #[serde(default)]
    pub uuid: Option<[u8; 16]>,
}

/// Remove a layer; a group's descendants and all their cels go with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveLayer {
    pub layer: LayerId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameLayer {
    pub layer: LayerId,
    pub name: String,
}

/// Append a frame at the end.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddFrame {
    pub duration_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetFrameDuration {
    pub frame: u32,
    pub duration_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetPaletteColor {
    pub index: u32,
    pub rgba: [u8; 4],
}

/// Set how many entries the palette has (1 to 256).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResizePalette {
    pub len: u32,
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
    ResizePalette(ResizePalette),
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
            Command::ResizePalette(_) => CommandKind::ResizePalette,
        }
    }
}
