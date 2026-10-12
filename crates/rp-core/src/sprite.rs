use std::collections::BTreeMap;
use std::ops::Range;

use crate::cel::{Cel, CelContent};
use crate::color::ColorMode;
use crate::layer::{Layer, LayerId};
use crate::palette::Palette;
use crate::raw_chunk::RawChunk;
use crate::slice::Slice;
use crate::tag::Tag;
use crate::tileset::Tileset;
use crate::user_data::UserData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeaderFlags(pub u32);

impl HeaderFlags {
    pub const LAYER_OPACITY_VALID: u32 = 1;
    pub const GROUP_BLEND_VALID: u32 = 2;
    pub const LAYER_UUIDS: u32 = 4;

    pub fn contains(self, bit: u32) -> bool {
        self.0 & bit == bit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Grid {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub duration_ms: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorProfile {
    pub kind: u16,
    pub flags: u16,
    pub gamma: i32,
    pub icc: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalFile {
    pub id: u32,
    pub kind: u8,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub width: u16,
    pub height: u16,
    pub color_mode: ColorMode,
    pub flags: HeaderFlags,
    pub transparent_index: u8,
    pub pixel_width: u8,
    pub pixel_height: u8,
    pub grid: Grid,
    /// File order: depth-first, each group directly followed by its descendants,
    /// bottom-most first within siblings.
    pub layers: Vec<Layer>,
    pub frames: Vec<Frame>,
    pub cels: BTreeMap<(LayerId, u32), Cel>,
    pub tags: Vec<Tag>,
    pub palette: Palette,
    pub slices: Vec<Slice>,
    pub tilesets: Vec<Tileset>,
    pub color_profile: Option<ColorProfile>,
    pub external_files: Vec<ExternalFile>,
    pub user_data: UserData,
    /// Chunks kept verbatim: types this reader does not model, palette chunks after frame 0, and user data with no owner in later frames. Re-emitted before their frame's cels, in order.
    pub raw_chunks: Vec<RawChunk>,
}

impl Sprite {
    /// One 100 ms frame, no layers, empty palette, flags = LAYER_OPACITY_VALID,
    /// pixel ratio 1:1, grid {0,0,16,16}, transparent_index 0, sRGB colour profile
    /// (kind 1, flags 0, gamma 0, no ICC).
    pub fn new(width: u16, height: u16, color_mode: ColorMode) -> Sprite {
        Sprite {
            width,
            height,
            color_mode,
            flags: HeaderFlags(HeaderFlags::LAYER_OPACITY_VALID),
            transparent_index: 0,
            pixel_width: 1,
            pixel_height: 1,
            grid: Grid {
                x: 0,
                y: 0,
                width: 16,
                height: 16,
            },
            layers: Vec::new(),
            frames: vec![Frame { duration_ms: 100 }],
            cels: BTreeMap::new(),
            tags: Vec::new(),
            palette: Palette::default(),
            slices: Vec::new(),
            tilesets: Vec::new(),
            color_profile: Some(ColorProfile {
                kind: 1,
                flags: 0,
                gamma: 0,
                icc: Vec::new(),
            }),
            external_files: Vec::new(),
            user_data: UserData::default(),
            raw_chunks: Vec::new(),
        }
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn layer_index(&self, id: LayerId) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    /// Index range of the layer plus all descendants in `layers` (contiguous by invariant).
    pub fn subtree_range(&self, id: LayerId) -> Option<Range<usize>> {
        let start = self.layer_index(id)?;
        let depth = self.depth(id)?;
        let mut end = start + 1;
        while let Some(next) = self.layers.get(end) {
            match self.depth(next.id) {
                Some(d) if d > depth => end += 1,
                _ => break,
            }
        }
        Some(start..end)
    }

    /// Nesting depth: 0 for root layers. None if the id is unknown or a parent is missing.
    pub fn depth(&self, id: LayerId) -> Option<u16> {
        let mut layer = self.layer(id)?;
        let mut depth = 0u16;
        // A parent cycle in corrupt data would loop forever; the budget bounds the walk.
        let mut budget = self.layers.len();
        while let Some(parent) = layer.parent {
            if budget == 0 {
                return None;
            }
            budget -= 1;
            layer = self.layer(parent)?;
            depth = depth.saturating_add(1);
        }
        Some(depth)
    }

    pub fn cel(&self, layer: LayerId, frame: u32) -> Option<&Cel> {
        self.cels.get(&(layer, frame))
    }

    /// Follows one Linked hop: returns the frame actually holding the content, and that cel.
    pub fn resolve_cel(&self, layer: LayerId, frame: u32) -> Option<(u32, &Cel)> {
        let cel = self.cel(layer, frame)?;
        match &cel.content {
            CelContent::Linked { frame: target } => {
                let linked = self.cel(layer, *target)?;
                Some((*target, linked))
            }
            _ => Some((frame, cel)),
        }
    }
}
