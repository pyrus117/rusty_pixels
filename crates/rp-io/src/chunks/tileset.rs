use std::io::Read;

use flate2::read::ZlibDecoder;
use rp_core::{ColorMode, ExternalTilesetRef, Image, Sprite, Tileset, UserData};

use crate::OBJECT_OVERHEAD;
use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::cel::compress;
use super::{add_chunks, charge, chunk, decode_budget, write_user_data};

pub(crate) const TILESET: u16 = 0x2023;

/// Parses one 0x2023 chunk. The caller stores it and points the user-data
/// target at it.
pub(crate) fn read(data: &[u8], sprite: &Sprite, budget: &mut usize) -> Result<Tileset, AseError> {
    let mut r = ByteReader::new(data);
    let id = r.u32()?;
    let flags = r.u32()?;
    let tile_count = r.u32()?;
    let tile_width = r.u16()?;
    let tile_height = r.u16()?;
    let base_index = r.i16()?;
    r.skip(14)?;
    let name = r.string()?;
    let external = if flags & Tileset::EXTERNAL != 0 {
        Some(ExternalTilesetRef {
            file_id: r.u32()?,
            tileset_id: r.u32()?,
        })
    } else {
        None
    };
    let tiles = if flags & Tileset::TILES_IN_FILE != 0 {
        read_tiles(
            &mut r,
            tile_count,
            tile_width,
            tile_height,
            sprite.color_mode,
            budget,
        )?
    } else {
        Vec::new()
    };
    Ok(Tileset {
        id,
        flags,
        tile_count,
        tile_width,
        tile_height,
        base_index,
        name,
        external,
        tiles,
        user_data: UserData::default(),
        tile_user_data: Vec::new(),
    })
}

/// The stored image is the tiles stacked vertically, so splitting it is one
/// slice per tile_height rows.
fn read_tiles(
    r: &mut ByteReader,
    tile_count: u32,
    tile_width: u16,
    tile_height: u16,
    mode: ColorMode,
    budget: &mut usize,
) -> Result<Vec<Image>, AseError> {
    // A zero-sized tile would split into unbounded empty tiles below.
    if tile_width == 0 || tile_height == 0 {
        return Err(AseError::Invalid {
            what: "tileset image",
            detail: "tile width and height must be nonzero".to_owned(),
        });
    }
    let tile_bytes = usize::from(tile_width)
        .checked_mul(usize::from(tile_height))
        .and_then(|px| px.checked_mul(mode.bytes_per_pixel()))
        .ok_or_else(size_overflow)?;
    // Every tile becomes its own image object, so the budget is charged with
    // overhead per tile before the zlib stream is touched.
    let per_tile = tile_bytes
        .checked_add(OBJECT_OVERHEAD)
        .ok_or_else(decode_budget)?;
    let cost = usize::try_from(tile_count)
        .ok()
        .and_then(|n| n.checked_mul(per_tile))
        .ok_or_else(decode_budget)?;
    charge(budget, cost)?;
    let expected = usize::try_from(tile_count)
        .ok()
        .and_then(|n| n.checked_mul(tile_bytes))
        .ok_or_else(size_overflow)?;
    let length = r.u32()?;
    // One past the expected size is enough to tell "too big" from "exact",
    // and caps what a zlib bomb can make us allocate.
    let mut decoder =
        ZlibDecoder::new(r.bytes(length as usize)?).take(expected.saturating_add(1) as u64);
    let mut pixels = Vec::with_capacity(expected);
    decoder
        .read_to_end(&mut pixels)
        .map_err(|e| AseError::Zlib(e.to_string()))?;
    if pixels.len() != expected {
        return Err(AseError::Invalid {
            what: "tileset image",
            detail: format!(
                "zlib tileset inflates to {} bytes, expected {expected}",
                pixels.len()
            ),
        });
    }
    Ok(pixels
        .chunks_exact(tile_bytes)
        .map(|tile| {
            Image::from_raw(
                u32::from(tile_width),
                u32::from(tile_height),
                mode,
                tile.to_vec(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?)
}

fn size_overflow() -> AseError {
    AseError::Invalid {
        what: "tileset image",
        detail: "tile width x height x tile count x bytes per pixel overflows".to_owned(),
    }
}

/// Writes every tileset as one chunk in frame 0. A tileset's user-data run is
/// written when the tileset or any tile carries user data: the tileset's own
/// (possibly empty) chunk first, then one chunk per stored tile user data.
pub(crate) fn write_tilesets(w: &mut ByteWriter, sprite: &Sprite) -> Result<u32, AseError> {
    let mut written = 0;
    for tileset in &sprite.tilesets {
        add_chunks(&mut written, write_tileset(w, sprite.color_mode, tileset)?)?;
    }
    Ok(written)
}

fn write_tileset(w: &mut ByteWriter, mode: ColorMode, ts: &Tileset) -> Result<u32, AseError> {
    if (ts.flags & Tileset::EXTERNAL != 0) != ts.external.is_some() {
        return Err(AseError::Invalid {
            what: "tileset",
            detail: format!(
                "tileset {}: flags bit 1 and the external reference disagree",
                ts.id
            ),
        });
    }
    let tiles_in_file = ts.flags & Tileset::TILES_IN_FILE != 0;
    if tiles_in_file {
        // A zero-sized tile would split into unbounded empty tiles on read.
        if ts.tile_width == 0 || ts.tile_height == 0 {
            return Err(AseError::Invalid {
                what: "tileset",
                detail: format!("tileset {}: zero tile size with tiles in file", ts.id),
            });
        }
        let count = usize::try_from(ts.tile_count).unwrap_or(usize::MAX);
        if ts.tiles.len() != count {
            return Err(AseError::Invalid {
                what: "tileset",
                detail: format!(
                    "tileset {}: {} tiles but the tile count is {}",
                    ts.id,
                    ts.tiles.len(),
                    ts.tile_count
                ),
            });
        }
        for (index, tile) in ts.tiles.iter().enumerate() {
            if tile.width() != u32::from(ts.tile_width)
                || tile.height() != u32::from(ts.tile_height)
                || tile.mode() != mode
            {
                return Err(AseError::Invalid {
                    what: "tileset",
                    detail: format!(
                        "tileset {}: tile {index} is not {}x{} in the sprite's colour mode",
                        ts.id, ts.tile_width, ts.tile_height
                    ),
                });
            }
        }
    } else if !ts.tiles.is_empty() {
        return Err(AseError::Invalid {
            what: "tileset",
            detail: format!(
                "tileset {}: flags bit 2 is clear but tiles are stored",
                ts.id
            ),
        });
    }
    let count = usize::try_from(ts.tile_count).unwrap_or(usize::MAX);
    if ts.tile_user_data.len() > count {
        return Err(AseError::Invalid {
            what: "tileset",
            detail: format!(
                "tileset {}: user data for {} tiles but the tile count is {}",
                ts.id,
                ts.tile_user_data.len(),
                ts.tile_count
            ),
        });
    }

    let mut data = ByteWriter::new();
    data.u32(ts.id);
    data.u32(ts.flags);
    data.u32(ts.tile_count);
    data.u16(ts.tile_width);
    data.u16(ts.tile_height);
    data.i16(ts.base_index);
    data.bytes(&[0; 14]);
    data.string(&ts.name)?;
    if let Some(external) = ts.external {
        data.u32(external.file_id);
        data.u32(external.tileset_id);
    }
    if tiles_in_file {
        // Tiles stack vertically in one image.
        let mut pixels = Vec::new();
        for tile in &ts.tiles {
            pixels.extend_from_slice(tile.data());
        }
        let compressed = compress(&pixels)?;
        let length = u32::try_from(compressed.len()).map_err(|_| AseError::OutOfRange {
            what: "tileset image size",
            value: i64::try_from(compressed.len()).unwrap_or(i64::MAX),
        })?;
        data.u32(length);
        data.bytes(&compressed);
    }
    chunk(w, TILESET, &data.into_vec())?;
    let mut written = 1;
    if !ts.user_data.is_empty() || ts.tile_user_data.iter().any(|ud| !ud.is_empty()) {
        add_chunks(&mut written, write_user_data(w, &ts.user_data)?)?;
        if tiles_in_file {
            // Observed: with tiles in the file Aseprite reads one chunk per
            // tile and warns "Unexpected chunk type" if the run is shorter, so
            // empty chunks pad the run out to the tile count.
            let empty = UserData::default();
            for index in 0..count {
                let ud = ts.tile_user_data.get(index).unwrap_or(&empty);
                add_chunks(&mut written, write_user_data(w, ud)?)?;
            }
        } else {
            // The tile count is unbacked without tiles in the file (H1), so
            // only stored tile user data gets a chunk.
            for ud in &ts.tile_user_data {
                add_chunks(&mut written, write_user_data(w, ud)?)?;
            }
        }
    }
    Ok(written)
}
