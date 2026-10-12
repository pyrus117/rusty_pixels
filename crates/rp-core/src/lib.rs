//! Core data model for Rusty Pixels.

mod cel;
mod color;
mod error;
mod image;
mod layer;
mod palette;
mod raw_chunk;
mod rgba;
mod slice;
mod sprite;
mod tag;
mod tileset;
mod user_data;

pub use cel::{Cel, CelContent, CelExtra, Tilemap};
pub use color::{ColorMode, PixelValue};
pub use error::CoreError;
pub use image::Image;
pub use layer::{BlendMode, Layer, LayerFlags, LayerId, LayerKind};
pub use palette::{Palette, PaletteEntry};
pub use raw_chunk::RawChunk;
pub use rgba::{MAX_RGBA_BYTES, RgbaImage};
pub use slice::{Pivot, Slice, SliceCenter, SliceKey};
pub use sprite::{ColorProfile, ExternalFile, Frame, Grid, HeaderFlags, Sprite};
pub use tag::{AniDir, Tag};
pub use tileset::{ExternalTilesetRef, Tileset};
pub use user_data::{PropValue, PropertiesMap, UserData};
