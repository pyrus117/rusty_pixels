use std::collections::HashMap;
use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use rp_core::{
    Cel, CelContent, CelExtra, ColorMode, Image, LayerId, LayerKind, Sprite, Tilemap, UserData,
};

use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::{add_chunks, charge, chunk, decode_budget, write_user_data};

pub(crate) const CEL: u16 = 0x2005;
pub(crate) const CEL_EXTRA: u16 = 0x2006;

/// Parses one cel chunk and stores it under (layer, frame). Returns the key,
/// which a following cel-extra chunk attaches to.
pub(crate) fn read(
    data: &[u8],
    sprite: &mut Sprite,
    frame: u32,
    budget: &mut usize,
) -> Result<(LayerId, u32), AseError> {
    let mut r = ByteReader::new(data);
    let layer_index = r.u16()?;
    let layer_kind = sprite
        .layers
        .get(usize::from(layer_index))
        .map(|l| l.kind)
        .ok_or_else(|| AseError::Invalid {
            what: "cel layer",
            detail: format!(
                "layer index {layer_index} but only {} layers were read so far",
                sprite.layers.len()
            ),
        })?;
    let layer = LayerId(u32::from(layer_index));
    let x = i32::from(r.i16()?);
    let y = i32::from(r.i16()?);
    let opacity = r.u8()?;
    let cel_type = r.u16()?;
    let z_index = r.i16()?;
    r.skip(5)?;
    let content = match cel_type {
        0 => CelContent::Image(image(&mut r, sprite.color_mode, budget)?),
        1 => CelContent::Linked {
            frame: u32::from(r.u16()?),
        },
        2 => CelContent::Image(zlib_image(&mut r, sprite.color_mode, budget)?),
        3 => {
            // Aseprite only writes tilemap cels on tilemap layers.
            if !matches!(layer_kind, LayerKind::Tilemap { .. }) {
                return Err(AseError::Invalid {
                    what: "cel type",
                    detail: format!(
                        "tilemap cel on layer {layer_index}, which is not a tilemap layer"
                    ),
                });
            }
            CelContent::Tilemap(tilemap(&mut r, budget)?)
        }
        _ => {
            return Err(AseError::Invalid {
                what: "cel type",
                detail: format!("{cel_type} is not a supported cel type"),
            });
        }
    };

    let key = (layer, frame);
    if sprite.cels.contains_key(&key) {
        return Err(AseError::Invalid {
            what: "duplicate cel",
            detail: format!("layer {} frame {frame} already has a cel", layer.0),
        });
    }
    sprite.cels.insert(
        key,
        Cel {
            x,
            y,
            opacity,
            z_index,
            content,
            extra: None,
            user_data: UserData::default(),
        },
    );
    Ok(key)
}

/// Parses one cel-extra chunk onto the cel read just before it.
pub(crate) fn read_extra(
    data: &[u8],
    sprite: &mut Sprite,
    last: Option<(LayerId, u32)>,
) -> Result<(), AseError> {
    let mut r = ByteReader::new(data);
    let flags = r.u32()?;
    let x = r.i32()?;
    let y = r.i32()?;
    let width = r.i32()?;
    let height = r.i32()?;
    r.skip(16)?;
    let key = last.ok_or_else(|| AseError::Invalid {
        what: "orphan cel extra",
        detail: "no cel was read in this frame".to_owned(),
    })?;
    let cel = sprite.cels.get_mut(&key).ok_or_else(|| AseError::Invalid {
        what: "orphan cel extra",
        detail: "the cel it follows is gone".to_owned(),
    })?;
    cel.extra = Some(CelExtra {
        flags,
        x,
        y,
        width,
        height,
    });
    Ok(())
}

fn image(r: &mut ByteReader, mode: ColorMode, budget: &mut usize) -> Result<Image, AseError> {
    let width = r.u16()?;
    let height = r.u16()?;
    let expected = charge_pixels(budget, width, height, mode.bytes_per_pixel())?;
    // The chunk ends with the pixels, so the rest is the raw image.
    let pixels = r.bytes(r.remaining())?.to_vec();
    if pixels.len() != expected {
        return Err(AseError::Invalid {
            what: "cel image",
            detail: format!("raw cel holds {} bytes, expected {expected}", pixels.len()),
        });
    }
    Ok(Image::from_raw(
        u32::from(width),
        u32::from(height),
        mode,
        pixels,
    )?)
}

fn zlib_image(r: &mut ByteReader, mode: ColorMode, budget: &mut usize) -> Result<Image, AseError> {
    let width = r.u16()?;
    let height = r.u16()?;
    let expected = charge_pixels(budget, width, height, mode.bytes_per_pixel())?;
    // One past the expected size is enough to tell "too big" from "exact",
    // and caps what a zlib bomb can make us allocate.
    let mut decoder =
        ZlibDecoder::new(r.bytes(r.remaining())?).take(expected.saturating_add(1) as u64);
    let mut pixels = Vec::with_capacity(expected);
    decoder
        .read_to_end(&mut pixels)
        .map_err(|e| AseError::Zlib(e.to_string()))?;
    if pixels.len() != expected {
        return Err(AseError::Invalid {
            what: "cel image",
            detail: format!(
                "zlib cel inflates to {} bytes, expected {expected}",
                pixels.len()
            ),
        });
    }
    Ok(Image::from_raw(
        u32::from(width),
        u32::from(height),
        mode,
        pixels,
    )?)
}

fn tilemap(r: &mut ByteReader, budget: &mut usize) -> Result<Tilemap, AseError> {
    let width = r.u16()?; // in tiles
    let height = r.u16()?;
    let bits_per_tile = r.u16()?;
    let tile_id_mask = r.u32()?;
    let x_flip_mask = r.u32()?;
    let y_flip_mask = r.u32()?;
    let diagonal_flip_mask = r.u32()?;
    r.skip(10)?;
    // Tiles widen to u32 in memory whatever the file stores.
    let bytes_per_tile = match bits_per_tile {
        8 => 1,
        16 => 2,
        32 => 4,
        _ => {
            return Err(AseError::Invalid {
                what: "tile bits",
                detail: format!("{bits_per_tile} is not 8, 16 or 32"),
            });
        }
    };
    let count = usize::from(width)
        .checked_mul(usize::from(height))
        .ok_or_else(|| AseError::Invalid {
            what: "tilemap",
            detail: "width x height in tiles overflows".to_owned(),
        })?;
    // The in-memory map holds a u32 per tile; that is what the budget sees
    // before the zlib stream is touched.
    charge_pixels(budget, width, height, 4)?;
    let expected = count
        .checked_mul(bytes_per_tile)
        .ok_or_else(|| AseError::Invalid {
            what: "tilemap",
            detail: "width x height x bytes per tile overflows".to_owned(),
        })?;
    // One past the expected size is enough to tell "too big" from "exact",
    // and caps what a zlib bomb can make us allocate.
    let mut decoder =
        ZlibDecoder::new(r.bytes(r.remaining())?).take(expected.saturating_add(1) as u64);
    let mut raw = Vec::with_capacity(expected);
    decoder
        .read_to_end(&mut raw)
        .map_err(|e| AseError::Zlib(e.to_string()))?;
    if raw.len() != expected {
        return Err(AseError::Invalid {
            what: "tilemap",
            detail: format!(
                "zlib tilemap inflates to {} bytes, expected {expected}",
                raw.len()
            ),
        });
    }
    let mut raw = ByteReader::new(&raw);
    let mut tiles = Vec::with_capacity(count);
    for _ in 0..count {
        tiles.push(match bytes_per_tile {
            1 => u32::from(raw.u8()?),
            2 => u32::from(raw.u16()?),
            _ => raw.u32()?,
        });
    }
    Ok(Tilemap {
        width: u32::from(width),
        height: u32::from(height),
        bits_per_tile,
        tile_id_mask,
        x_flip_mask,
        y_flip_mask,
        diagonal_flip_mask,
        tiles,
    })
}

/// The cel's declared decoded size, charged to the budget before anything is
/// inflated. An overflowing size is a budget failure like an over-budget one.
fn charge_pixels(
    budget: &mut usize,
    width: u16,
    height: u16,
    bytes_per_pixel: usize,
) -> Result<usize, AseError> {
    let size = usize::from(width)
        .checked_mul(usize::from(height))
        .and_then(|px| px.checked_mul(bytes_per_pixel))
        .ok_or_else(decode_budget)?;
    charge(budget, size)?;
    Ok(size)
}

/// Writes one frame's cels in layer order, each followed by its cel extra and
/// user data.
pub(crate) fn write_frame_cels(
    w: &mut ByteWriter,
    sprite: &Sprite,
    layer_index: &HashMap<LayerId, usize>,
    cels: &[(LayerId, &Cel)],
) -> Result<u32, AseError> {
    let mut ordered: Vec<(usize, &Cel)> = Vec::new();
    for &(id, cel) in cels {
        let position = *layer_index.get(&id).ok_or_else(|| AseError::Invalid {
            what: "cel layer",
            detail: format!("no layer has id {}", id.0),
        })?;
        // A tilemap cel outside a tilemap layer would write a file that no
        // reader accepts back.
        if matches!(cel.content, CelContent::Tilemap(_)) {
            let on_tilemap = sprite
                .layers
                .get(position)
                .is_some_and(|l| matches!(l.kind, LayerKind::Tilemap { .. }));
            if !on_tilemap {
                return Err(AseError::Invalid {
                    what: "cel type",
                    detail: format!(
                        "tilemap cel on layer {}, which is not a tilemap layer",
                        id.0
                    ),
                });
            }
        }
        // The header holds one colour mode, so a cel image in another mode
        // cannot survive a round-trip.
        if let CelContent::Image(img) = &cel.content
            && img.mode() != sprite.color_mode
        {
            return Err(AseError::Invalid {
                what: "cel image",
                detail: format!(
                    "cel on layer {} is {:?} but the sprite is {:?}",
                    id.0,
                    img.mode(),
                    sprite.color_mode
                ),
            });
        }
        ordered.push((position, cel));
    }
    ordered.sort_by_key(|&(position, _)| position);

    let mut written = 0u32;
    for (position, cel) in ordered {
        let layer_index = u16::try_from(position).map_err(|_| AseError::OutOfRange {
            what: "layer index",
            value: i64::try_from(position).unwrap_or(i64::MAX),
        })?;
        write_cel(w, layer_index, cel)?;
        add_chunks(&mut written, 1)?;
        if let Some(extra) = cel.extra {
            write_extra(w, extra)?;
            add_chunks(&mut written, 1)?;
        }
        if !cel.user_data.is_empty() {
            add_chunks(&mut written, write_user_data(w, &cel.user_data)?)?;
        }
    }
    Ok(written)
}

fn write_cel(w: &mut ByteWriter, layer_index: u16, cel: &Cel) -> Result<(), AseError> {
    let x = i16::try_from(cel.x).map_err(|_| AseError::OutOfRange {
        what: "cel x",
        value: i64::from(cel.x),
    })?;
    let y = i16::try_from(cel.y).map_err(|_| AseError::OutOfRange {
        what: "cel y",
        value: i64::from(cel.y),
    })?;
    let mut data = ByteWriter::new();
    data.u16(layer_index);
    data.i16(x);
    data.i16(y);
    data.u8(cel.opacity);
    match &cel.content {
        CelContent::Image(img) => {
            // The raw cel type exists only for very old files.
            data.u16(2);
            data.i16(cel.z_index);
            data.bytes(&[0; 5]);
            data.u16(checked_u16(img.width(), "cel width")?);
            data.u16(checked_u16(img.height(), "cel height")?);
            data.bytes(&compress(img.data())?);
        }
        CelContent::Linked { frame } => {
            data.u16(1);
            data.i16(cel.z_index);
            data.bytes(&[0; 5]);
            data.u16(checked_u16(*frame, "linked frame")?);
        }
        CelContent::Tilemap(map) => {
            data.u16(3);
            data.i16(cel.z_index);
            data.bytes(&[0; 5]);
            data.u16(checked_u16(map.width, "tilemap width")?);
            data.u16(checked_u16(map.height, "tilemap height")?);
            let bytes_per_tile = match map.bits_per_tile {
                8 => 1,
                16 => 2,
                32 => 4,
                _ => {
                    return Err(AseError::Invalid {
                        what: "tile bits",
                        detail: format!("{} is not 8, 16 or 32", map.bits_per_tile),
                    });
                }
            };
            data.u16(map.bits_per_tile);
            data.u32(map.tile_id_mask);
            data.u32(map.x_flip_mask);
            data.u32(map.y_flip_mask);
            data.u32(map.diagonal_flip_mask);
            data.bytes(&[0; 10]);
            let count = usize::try_from(map.width)
                .ok()
                .and_then(|w| {
                    usize::try_from(map.height)
                        .ok()
                        .and_then(|h| w.checked_mul(h))
                })
                .ok_or_else(|| AseError::Invalid {
                    what: "tilemap",
                    detail: "width x height in tiles overflows".to_owned(),
                })?;
            if map.tiles.len() != count {
                return Err(AseError::Invalid {
                    what: "tilemap",
                    detail: format!("{} tiles but the map holds {count}", map.tiles.len()),
                });
            }
            let mut raw = Vec::new();
            for (index, &tile) in map.tiles.iter().enumerate() {
                // The file stores bits_per_tile bits per tile, so a wider
                // value has no place in it.
                let fits = match bytes_per_tile {
                    1 => tile <= u32::from(u8::MAX),
                    2 => tile <= u32::from(u16::MAX),
                    _ => true,
                };
                if !fits {
                    return Err(AseError::Invalid {
                        what: "tilemap",
                        detail: format!(
                            "tile {index} value {tile:#x} does not fit {} bits",
                            map.bits_per_tile
                        ),
                    });
                }
                let bytes = tile.to_le_bytes();
                raw.extend_from_slice(&bytes[..bytes_per_tile]);
            }
            data.bytes(&compress(&raw)?);
        }
    }
    chunk(w, CEL, &data.into_vec())
}

fn write_extra(w: &mut ByteWriter, extra: CelExtra) -> Result<(), AseError> {
    let mut data = ByteWriter::new();
    data.u32(extra.flags);
    data.i32(extra.x);
    data.i32(extra.y);
    data.i32(extra.width);
    data.i32(extra.height);
    data.bytes(&[0; 16]);
    chunk(w, CEL_EXTRA, &data.into_vec())
}

pub(crate) fn compress(data: &[u8]) -> Result<Vec<u8>, AseError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| AseError::Zlib(e.to_string()))?;
    encoder.finish().map_err(|e| AseError::Zlib(e.to_string()))
}

fn checked_u16(value: u32, what: &'static str) -> Result<u16, AseError> {
    u16::try_from(value).map_err(|_| AseError::OutOfRange {
        what,
        value: i64::from(value),
    })
}
