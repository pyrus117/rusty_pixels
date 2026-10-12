use crate::command::{
    AddFrame, AddLayer, CommandKind, RemoveLayer, RenameLayer, SetFrameDuration, SetPaletteColor,
    SetPixels,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpSpec {
    /// Equals the CommandKind's snake_case name and the JSON "op" tag.
    pub id: &'static str,
    pub title: &'static str,
    pub doc: &'static str,
    pub menu_path: Option<&'static str>,
    pub default_shortcut: Option<&'static str>,
    pub undoable: bool,
    pub agent_exposed: bool,
}

static SPECS: [OpSpec; 7] = [
    OpSpec {
        id: "set_pixels",
        title: "Set Pixels",
        doc: "Write pixels into an image layer's cel at a frame, creating or growing it.",
        menu_path: None,
        default_shortcut: None,
        undoable: true,
        agent_exposed: true,
    },
    OpSpec {
        id: "add_layer",
        title: "New Layer",
        doc: "Add an image layer above another layer, or at the top of the root stack.",
        menu_path: Some("Layer/New/New Layer"),
        default_shortcut: Some("Shift+N"),
        undoable: true,
        agent_exposed: true,
    },
    OpSpec {
        id: "remove_layer",
        title: "Delete Layer",
        doc: "Remove a layer; a group's descendants and all their cels go with it.",
        menu_path: Some("Layer/Delete Layer"),
        default_shortcut: None,
        undoable: true,
        agent_exposed: true,
    },
    OpSpec {
        id: "rename_layer",
        title: "Rename Layer",
        doc: "Change a layer's name.",
        menu_path: Some("Layer/Properties…"),
        default_shortcut: Some("Shift+P"),
        undoable: true,
        agent_exposed: true,
    },
    OpSpec {
        id: "add_frame",
        title: "New Frame",
        doc: "Append a frame at the end.",
        menu_path: Some("Frame/New Frame"),
        default_shortcut: Some("Alt+N"),
        undoable: true,
        agent_exposed: true,
    },
    OpSpec {
        id: "set_frame_duration",
        title: "Frame Duration",
        doc: "Set how long one frame stays on screen.",
        menu_path: Some("Frame/Frame Properties…"),
        default_shortcut: Some("P"),
        undoable: true,
        agent_exposed: true,
    },
    OpSpec {
        id: "set_palette_color",
        title: "Set Palette Colour",
        doc: "Set one palette entry's RGBA colour.",
        menu_path: None,
        default_shortcut: None,
        undoable: true,
        agent_exposed: true,
    },
];

pub fn registry() -> &'static [OpSpec] {
    &SPECS
}

/// Every CommandKind has exactly one spec (the parity test proves it), so this is total.
pub fn spec(kind: CommandKind) -> &'static OpSpec {
    match kind {
        CommandKind::SetPixels => &SPECS[0],
        CommandKind::AddLayer => &SPECS[1],
        CommandKind::RemoveLayer => &SPECS[2],
        CommandKind::RenameLayer => &SPECS[3],
        CommandKind::AddFrame => &SPECS[4],
        CommandKind::SetFrameDuration => &SPECS[5],
        CommandKind::SetPaletteColor => &SPECS[6],
    }
}

/// JSON Schema (schemars, draft 2020-12) of the op's parameter struct, without the "op" field.
pub fn params_schema(kind: CommandKind) -> serde_json::Value {
    match kind {
        CommandKind::SetPixels => schemars::schema_for!(SetPixels).to_value(),
        CommandKind::AddLayer => schemars::schema_for!(AddLayer).to_value(),
        CommandKind::RemoveLayer => schemars::schema_for!(RemoveLayer).to_value(),
        CommandKind::RenameLayer => schemars::schema_for!(RenameLayer).to_value(),
        CommandKind::AddFrame => schemars::schema_for!(AddFrame).to_value(),
        CommandKind::SetFrameDuration => schemars::schema_for!(SetFrameDuration).to_value(),
        CommandKind::SetPaletteColor => schemars::schema_for!(SetPaletteColor).to_value(),
    }
}
