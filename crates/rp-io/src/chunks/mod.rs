mod cel;
mod external;
mod layer;
mod palette;
mod profile;
mod slice;
mod tags;
mod tileset;
mod user_data;

use rp_core::{LayerId, RawChunk, Sprite};

use crate::bytes::ByteWriter;
use crate::error::AseError;

pub(crate) use cel::write_frame_cels;
pub(crate) use external::write_external_files;
pub(crate) use layer::{check_tree, write_layers};
pub(crate) use palette::write_palette;
pub(crate) use profile::write_color_profile;
pub(crate) use slice::write_slices;
pub(crate) use tags::write_tags;
pub(crate) use tileset::write_tilesets;
pub(crate) use user_data::write_user_data;

use user_data::Target;

/// State that flows between chunks of one file: the child-level stack filled
/// by layer chunks, the cel a cel-extra chunk follows, which is per frame,
/// whether a 0x2019 chunk was seen, which retires the old palette chunks, the
/// sprite's single user-data slot, the object a user-data chunk attaches to
/// next, and the decode budget left for the file.
pub(crate) struct ReadState {
    pub(crate) frame: u32,
    pub(crate) levels: Vec<LayerId>,
    pub(crate) last_cel: Option<(LayerId, u32)>,
    pub(crate) new_palette: bool,
    pub(crate) sprite_ud: bool,
    pub(crate) target: Target,
    pub(crate) budget: usize,
}

impl Default for ReadState {
    fn default() -> Self {
        ReadState {
            frame: 0,
            levels: Vec::new(),
            last_cel: None,
            new_palette: false,
            sprite_ud: false,
            target: Target::default(),
            // One read() call carries one budget for the whole file.
            budget: crate::DECODE_BUDGET,
        }
    }
}

impl ReadState {
    /// Moves to the next frame. A cel-extra chunk only attaches to a cel of
    /// its own frame. Observed: user data at the start of frame 0 belongs to
    /// the sprite; a later frame starts with no target, where user data has no
    /// owner and is kept raw.
    pub(crate) fn start_frame(&mut self, index: u32) {
        self.frame = index;
        self.last_cel = None;
        self.target = if index == 0 {
            Target::Sprite
        } else {
            Target::None
        };
    }
}

/// Applies one chunk body to the sprite.
pub(crate) fn apply(
    kind: u16,
    data: &[u8],
    sprite: &mut Sprite,
    state: &mut ReadState,
) -> Result<(), AseError> {
    match kind {
        layer::LAYER => {
            let id = layer::read(data, sprite, &mut state.levels)?;
            state.target = Target::Layer(id);
            Ok(())
        }
        cel::CEL => {
            let key = cel::read(data, sprite, state.frame, &mut state.budget)?;
            state.last_cel = Some(key);
            state.target = Target::Cel(key);
            Ok(())
        }
        // A cel extra is part of the cel, so it keeps the target.
        cel::CEL_EXTRA => cel::read_extra(data, sprite, state.last_cel),
        profile::COLOR_PROFILE => {
            sprite.color_profile = Some(profile::read(data)?);
            Ok(())
        }
        external::EXTERNAL_FILES => {
            external::read(data, sprite)?;
            Ok(())
        }
        // The model holds one palette, defined by frame 0; later chunks stay raw.
        palette::PALETTE | palette::OLD_PALETTE | palette::OLD_PALETTE_6BIT if state.frame > 0 => {
            store_raw(kind, data, sprite, state)
        }
        palette::PALETTE => {
            // Any 0x2019 chunk in frame 0 retires the old chunks whatever the
            // order, so the palette restarts empty at the first one.
            if !state.new_palette {
                state.new_palette = true;
                sprite.palette.entries.clear();
            }
            state.target = Target::Sprite;
            palette::read(data, sprite)
        }
        // Whichever old chunk comes first, a 0x2019 chunk anywhere retires it.
        palette::OLD_PALETTE | palette::OLD_PALETTE_6BIT => {
            state.target = Target::Sprite;
            palette::read_old(kind, data, sprite, state.new_palette)
        }
        tags::TAGS => {
            tags::read(data, sprite)?;
            state.target = Target::Tag(0);
            Ok(())
        }
        slice::SLICE => {
            let index = slice::read(data, sprite)?;
            state.target = Target::Slice(index);
            Ok(())
        }
        tileset::TILESET => {
            let index = sprite.tilesets.len();
            sprite
                .tilesets
                .push(tileset::read(data, sprite, &mut state.budget)?);
            state.target = Target::Tileset(index);
            Ok(())
        }
        user_data::USER_DATA => user_data::read(data, sprite, state),
        // Types without a decoder — including the deprecated 0x2016/0x2017 —
        // are kept raw and leave the user-data target alone.
        _ => store_raw(kind, data, sprite, state),
    }
}

/// Keeps a chunk verbatim in its frame. It leaves the user-data target alone,
/// so a user-data chunk that follows still attaches to the object before it.
fn store_raw(
    kind: u16,
    data: &[u8],
    sprite: &mut Sprite,
    state: &mut ReadState,
) -> Result<(), AseError> {
    sprite.raw_chunks.push(RawChunk {
        frame: state.frame,
        kind,
        data: data.to_vec(),
    });
    Ok(())
}

/// Emits one kept-verbatim chunk, body untouched.
pub(crate) fn write_raw(w: &mut ByteWriter, raw: &RawChunk) -> Result<u32, AseError> {
    chunk(w, raw.kind, &raw.data)?;
    Ok(1)
}

/// Chunk counters are DWORDs in the file, so an overflow is a format limit.
pub(crate) fn add_chunks(total: &mut u32, n: u32) -> Result<(), AseError> {
    *total = total.checked_add(n).ok_or(AseError::OutOfRange {
        what: "chunk count",
        value: i64::from(u32::MAX),
    })?;
    Ok(())
}

/// Charges declared decoded bytes to the budget before anything is inflated.
/// An over-budget size is a failure like an overflowing one.
pub(crate) fn charge(budget: &mut usize, size: usize) -> Result<(), AseError> {
    if size > *budget {
        return Err(decode_budget());
    }
    *budget -= size;
    Ok(())
}

pub(crate) fn decode_budget() -> AseError {
    AseError::Invalid {
        what: "decode budget",
        detail: "the declared decoded size runs past the decode budget".to_owned(),
    }
}

fn chunk(w: &mut ByteWriter, kind: u16, data: &[u8]) -> Result<(), AseError> {
    if data.len() > u32::MAX as usize - 6 {
        return Err(AseError::OutOfRange {
            what: "chunk size",
            value: i64::try_from(data.len()).unwrap_or(i64::MAX),
        });
    }
    // The size field counts its own 4 bytes plus the 2 type bytes.
    w.u32(data.len() as u32 + 6);
    w.u16(kind);
    w.bytes(data);
    Ok(())
}
