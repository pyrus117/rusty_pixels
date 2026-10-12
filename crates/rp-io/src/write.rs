use std::collections::BTreeMap;

use rp_core::{Cel, LayerId, RawChunk, Sprite};

use crate::bytes::ByteWriter;
use crate::chunks;
use crate::error::AseError;

const HEADER_MAGIC: u16 = 0xA5E0;
const FRAME_MAGIC: u16 = 0xF1FA;

/// Serialises a [`Sprite`] into .aseprite bytes.
pub fn write(sprite: &Sprite) -> Result<Vec<u8>, AseError> {
    let Some(first) = sprite.frames.first() else {
        return Err(AseError::Invalid {
            what: "frames",
            detail: "a sprite with no frames cannot be written".to_owned(),
        });
    };
    for (index, frame) in sprite.frames.iter().enumerate() {
        if frame.duration_ms == 0 {
            return Err(AseError::Invalid {
                what: "frame duration",
                detail: format!("frame {index} has zero duration"),
            });
        }
    }

    // The tree is checked first: layer chunks can only encode a contiguous
    // depth-first order with group parents before their children.
    let (child_levels, layer_index) = chunks::check_tree(sprite)?;
    // A cel for a frame that does not exist would never be written.
    for &(_, frame) in sprite.cels.keys() {
        if usize::try_from(frame).unwrap_or(usize::MAX) >= sprite.frames.len() {
            return Err(AseError::Invalid {
                what: "cel frame",
                detail: format!(
                    "cel for frame {frame} but the sprite has {} frames",
                    sprite.frames.len()
                ),
            });
        }
    }
    // The same goes for a kept-verbatim chunk past the last frame.
    for raw in &sprite.raw_chunks {
        if usize::try_from(raw.frame).unwrap_or(usize::MAX) >= sprite.frames.len() {
            return Err(AseError::Invalid {
                what: "raw chunk",
                detail: format!(
                    "raw chunk for frame {} but the sprite has {} frames",
                    raw.frame,
                    sprite.frames.len()
                ),
            });
        }
    }
    // Tags name frame ranges, so every one must fit the frames.
    for tag in &sprite.tags {
        if tag.from > tag.to || usize::try_from(tag.to).unwrap_or(usize::MAX) >= sprite.frames.len()
        {
            return Err(AseError::Invalid {
                what: "tag range",
                detail: format!(
                    "tag {} covers {}..{} but the sprite has {} frames",
                    tag.name,
                    tag.from,
                    tag.to,
                    sprite.frames.len()
                ),
            });
        }
    }

    let mut w = ByteWriter::new();
    w.u32(0); // file size, set once the file is assembled
    w.u16(HEADER_MAGIC);
    w.u16(count("frames", sprite.frames.len())?);
    w.u16(sprite.width);
    w.u16(sprite.height);
    w.u16(sprite.color_mode.depth_bits());
    w.u32(sprite.flags.0);
    w.u16(first.duration_ms);
    w.u32(0);
    w.u32(0);
    w.u8(sprite.transparent_index);
    w.bytes(&[0; 3]);
    w.u16(count("palette entries", sprite.palette.entries.len())?);
    w.u8(sprite.pixel_width);
    w.u8(sprite.pixel_height);
    w.i16(sprite.grid.x);
    w.i16(sprite.grid.y);
    w.u16(sprite.grid.width);
    w.u16(sprite.grid.height);
    w.bytes(&[0; 84]);

    // Grouping the cels and kept-verbatim chunks by frame once keeps the emit
    // linear; scanning both for every frame is quadratic in the frame count.
    let mut cels_by_frame: BTreeMap<u32, Vec<(LayerId, &Cel)>> = BTreeMap::new();
    for (key, cel) in &sprite.cels {
        let (id, frame) = *key;
        cels_by_frame.entry(frame).or_default().push((id, cel));
    }
    let mut raws_by_frame: BTreeMap<u32, Vec<&RawChunk>> = BTreeMap::new();
    for raw in &sprite.raw_chunks {
        raws_by_frame.entry(raw.frame).or_default().push(raw);
    }

    for (index, frame) in sprite.frames.iter().enumerate() {
        let mut body = ByteWriter::new();
        let mut chunk_count = 0u32;
        let frame_index = u32::try_from(index).map_err(|_| AseError::OutOfRange {
            what: "frame index",
            value: i64::try_from(index).unwrap_or(i64::MAX),
        })?;
        if index == 0 {
            // Sprite user data anchors to the palette chunk, or, without one,
            // to the start of frame 0 (observed: it opens the frame).
            let sprite_ud = (!sprite.user_data.is_empty()).then_some(&sprite.user_data);
            if sprite.palette.entries.is_empty()
                && let Some(ud) = sprite_ud
            {
                chunks::add_chunks(&mut chunk_count, chunks::write_user_data(&mut body, ud)?)?;
            }
            // Frame-0 order: external files, profile, palette, tags, tilesets,
            // layers, slices, then the raw chunks and cels of this frame below.
            chunks::add_chunks(
                &mut chunk_count,
                chunks::write_external_files(&mut body, &sprite.external_files)?,
            )?;
            if let Some(profile) = &sprite.color_profile {
                chunks::add_chunks(
                    &mut chunk_count,
                    chunks::write_color_profile(&mut body, profile)?,
                )?;
            }
            chunks::add_chunks(
                &mut chunk_count,
                chunks::write_palette(&mut body, &sprite.palette)?,
            )?;
            if !sprite.palette.entries.is_empty()
                && let Some(ud) = sprite_ud
            {
                chunks::add_chunks(&mut chunk_count, chunks::write_user_data(&mut body, ud)?)?;
            }
            chunks::add_chunks(
                &mut chunk_count,
                chunks::write_tags(&mut body, &sprite.tags)?,
            )?;
            // Fixtures place tilesets here; none has both tags and a tileset,
            // so that order stays open for Task 10.
            chunks::add_chunks(&mut chunk_count, chunks::write_tilesets(&mut body, sprite)?)?;
            chunks::add_chunks(
                &mut chunk_count,
                chunks::write_layers(&mut body, sprite, &child_levels)?,
            )?;
            chunks::add_chunks(
                &mut chunk_count,
                chunks::write_slices(&mut body, &sprite.slices)?,
            )?;
        }
        // Kept-verbatim chunks open the frame's own chunks, in stored order:
        // ahead of the cels so raw user data re-reads raw instead of attaching
        // to a cel that would precede it.
        for raw in raws_by_frame.get(&frame_index).into_iter().flatten() {
            chunks::add_chunks(&mut chunk_count, chunks::write_raw(&mut body, raw)?)?;
        }
        chunks::add_chunks(
            &mut chunk_count,
            chunks::write_frame_cels(
                &mut body,
                sprite,
                &layer_index,
                cels_by_frame.get(&frame_index).map_or(&[], Vec::as_slice),
            )?,
        )?;
        let body = body.into_vec();
        let size = u32::try_from(body.len())
            .map_err(|_| AseError::OutOfRange {
                what: "frame size",
                value: i64::try_from(body.len()).unwrap_or(i64::MAX),
            })?
            .checked_add(16)
            .ok_or(AseError::OutOfRange {
                what: "frame size",
                value: i64::from(u32::MAX),
            })?;
        w.u32(size);
        w.u16(FRAME_MAGIC);
        // Old readers use the WORD field; new ones only when it is 0xFFFF.
        w.u16(u16::try_from(chunk_count).unwrap_or(u16::MAX));
        w.u16(frame.duration_ms);
        w.bytes(&[0; 2]);
        w.u32(chunk_count);
        w.bytes(&body);
    }

    let mut out = w.into_vec();
    let total = u32::try_from(out.len()).map_err(|_| AseError::OutOfRange {
        what: "file size",
        value: i64::try_from(out.len()).unwrap_or(i64::MAX),
    })?;
    let size_field = out.get_mut(0..4).ok_or(AseError::Invalid {
        what: "file size",
        detail: "the assembled file is smaller than its own size field".to_owned(),
    })?;
    size_field.copy_from_slice(&total.to_le_bytes());
    Ok(out)
}

/// Header fields that are WORDs, so anything over 65535 cannot be stored.
fn count(what: &'static str, n: usize) -> Result<u16, AseError> {
    u16::try_from(n).map_err(|_| AseError::OutOfRange {
        what,
        value: i64::try_from(n).unwrap_or(i64::MAX),
    })
}
