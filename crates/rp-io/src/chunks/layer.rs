use std::collections::HashMap;

use rp_core::{BlendMode, HeaderFlags, Layer, LayerFlags, LayerId, LayerKind, Sprite, UserData};

use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::{add_chunks, chunk, write_user_data};

pub(crate) const LAYER: u16 = 0x2004;

/// Parses one layer chunk and appends it to the sprite. `levels` holds the
/// most recent layer at each child level, so level L's parent sits at L-1.
/// Returns the new layer's id, which a following user-data chunk attaches to.
pub(crate) fn read(
    data: &[u8],
    sprite: &mut Sprite,
    levels: &mut Vec<LayerId>,
) -> Result<LayerId, AseError> {
    let mut r = ByteReader::new(data);
    let flags = LayerFlags(r.u16()?);
    let type_word = r.u16()?;
    if type_word > 2 {
        return Err(AseError::Invalid {
            what: "layer type",
            detail: format!("{type_word} is not 0, 1 or 2"),
        });
    }
    let child_level = usize::from(r.u16()?);
    r.skip(4)?; // default width/height; the format ignores them
    let blend_word = r.u16()?;
    let blend_mode = BlendMode::from_u16(blend_word).ok_or_else(|| AseError::Invalid {
        what: "blend mode",
        detail: format!("{blend_word} is not a blend mode"),
    })?;
    let opacity = r.u8()?;
    r.skip(3)?;
    let name = r.string()?;
    let kind = match type_word {
        0 => LayerKind::Image,
        1 => LayerKind::Group,
        _ => LayerKind::Tilemap {
            tileset_index: r.u32()?,
        },
    };
    let uuid = if sprite.flags.contains(HeaderFlags::LAYER_UUIDS) {
        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(r.bytes(16)?);
        Some(uuid)
    } else {
        None
    };

    let id = LayerId(
        u32::try_from(sprite.layers.len()).map_err(|_| AseError::OutOfRange {
            what: "layer count",
            value: i64::try_from(sprite.layers.len()).unwrap_or(i64::MAX),
        })?,
    );
    let parent = parent_of(child_level, sprite, levels)?;
    levels.truncate(child_level);
    levels.push(id);
    sprite.layers.push(Layer {
        id,
        parent,
        name,
        kind,
        flags,
        blend_mode,
        opacity,
        uuid,
        user_data: UserData::default(),
    });
    Ok(id)
}

fn parent_of(
    child_level: usize,
    sprite: &Sprite,
    levels: &[LayerId],
) -> Result<Option<LayerId>, AseError> {
    if child_level == 0 {
        return Ok(None);
    }
    // A level deeper than the stack can reach is the "jump" case: the file
    // skipped the level above.
    let parent = *levels.get(child_level - 1).ok_or_else(|| {
        tree(format!(
            "child level {child_level} has no layer one level up"
        ))
    })?;
    let parent_layer = sprite
        // On read a layer's id is its file index, so this indexes directly.
        .layers
        .get(usize::try_from(parent.0).unwrap_or(usize::MAX))
        .ok_or_else(|| tree("the parent layer is missing".to_owned()))?;
    if !matches!(parent_layer.kind, LayerKind::Group) {
        return Err(tree(format!(
            "the parent of child level {child_level} is not a group"
        )));
    }
    Ok(Some(parent))
}

/// Validates the parent links and returns each layer's child level (its depth)
/// and the id -> index map the writer looks layers up through. A parent must
/// be a group earlier in `layers`, and one group's subtree must be a
/// contiguous run — the only order layer chunks can encode.
pub(crate) fn check_tree(sprite: &Sprite) -> Result<(Vec<u16>, HashMap<LayerId, usize>), AseError> {
    let mut index: HashMap<LayerId, usize> = HashMap::with_capacity(sprite.layers.len());
    for (i, layer) in sprite.layers.iter().enumerate() {
        if index.insert(layer.id, i).is_some() {
            return Err(tree(format!(
                "layer id {} appears more than once",
                layer.id.0
            )));
        }
    }
    let mut levels = Vec::with_capacity(sprite.layers.len());
    let mut depths: Vec<u16> = Vec::with_capacity(sprite.layers.len());
    for (i, layer) in sprite.layers.iter().enumerate() {
        let depth = match layer.parent {
            None => 0u16,
            Some(parent) => {
                let j = *index
                    .get(&parent)
                    .ok_or_else(|| tree("a parent is missing".to_owned()))?;
                // j < i also rules out parent cycles and self-parents.
                if j >= i {
                    return Err(tree("a parent appears after its children".to_owned()));
                }
                let p = sprite
                    .layers
                    .get(j)
                    .ok_or_else(|| tree("a parent is missing".to_owned()))?;
                if !matches!(p.kind, LayerKind::Group) {
                    return Err(tree("a parent is not a group".to_owned()));
                }
                // A parent sits earlier in the file, so its depth is known and
                // one lookup covers the whole chain walk.
                let parent_depth = depths
                    .get(j)
                    .copied()
                    .ok_or_else(|| tree("a parent link is broken".to_owned()))?;
                parent_depth.checked_add(1).ok_or(AseError::OutOfRange {
                    what: "layer depth",
                    value: i64::from(u16::MAX),
                })?
            }
        };
        if let Some(parent) = layer.parent {
            // A layer with a parent has depth >= 1, so this cannot underflow.
            if levels.get(usize::from(depth).saturating_sub(1)) != Some(&parent) {
                return Err(tree("a subtree is not contiguous".to_owned()));
            }
        }
        levels.truncate(usize::from(depth));
        levels.push(layer.id);
        depths.push(depth);
    }
    Ok((depths, index))
}

/// Writes every layer as one chunk in frame 0, each followed by its user data.
/// The uuid field exists only when the header says so; a missing uuid is then a
/// hard error, not a blank one.
pub(crate) fn write_layers(
    w: &mut ByteWriter,
    sprite: &Sprite,
    child_levels: &[u16],
) -> Result<u32, AseError> {
    let store_uuids = sprite.flags.contains(HeaderFlags::LAYER_UUIDS);
    let mut written = 0u32;
    for (i, layer) in sprite.layers.iter().enumerate() {
        let child_level = *child_levels.get(i).ok_or_else(|| AseError::Invalid {
            what: "layer tree",
            detail: format!("no child level computed for layer {i}"),
        })?;
        let mut data = ByteWriter::new();
        data.u16(layer.flags.0);
        data.u16(match layer.kind {
            LayerKind::Image => 0,
            LayerKind::Group => 1,
            LayerKind::Tilemap { .. } => 2,
        });
        data.u16(child_level);
        data.u16(0); // default width/height, always zero
        data.u16(0);
        data.u16(layer.blend_mode.as_u16());
        data.u8(layer.opacity);
        data.bytes(&[0; 3]);
        data.string(&layer.name)?;
        if let LayerKind::Tilemap { tileset_index } = layer.kind {
            data.u32(tileset_index);
        }
        if store_uuids {
            let uuid = layer.uuid.ok_or_else(|| AseError::Invalid {
                what: "layer uuid",
                detail: format!("{} has no uuid but the file stores uuids", layer.name),
            })?;
            data.bytes(&uuid);
        }
        chunk(w, LAYER, &data.into_vec())?;
        add_chunks(&mut written, 1)?;
        if !layer.user_data.is_empty() {
            add_chunks(&mut written, write_user_data(w, &layer.user_data)?)?;
        }
    }
    Ok(written)
}

fn tree(detail: String) -> AseError {
    AseError::Invalid {
        what: "layer tree",
        detail,
    }
}
