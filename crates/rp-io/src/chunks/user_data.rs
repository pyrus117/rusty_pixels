use rp_core::{LayerId, PropValue, PropertiesMap, Sprite, UserData};

use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::ReadState;
use super::chunk;
use super::store_raw;

pub(crate) const USER_DATA: u16 = 0x2020;

/// The format caps nested vectors and maps at 128 levels.
const MAX_DEPTH: u32 = 128;

/// The object the next 0x2020 chunk belongs to: the "last read chunk/object"
/// of the spec, narrowed to the objects that can own user data.
#[derive(Clone, Copy, Default)]
pub(crate) enum Target {
    /// Nothing attachable was read. This is where later frames start, so a
    /// user-data chunk here has no owner and is kept raw.
    #[default]
    None,
    /// The owner the last user-data chunk filled; a second one is an error.
    Used,
    Sprite,
    Layer(LayerId),
    Cel((LayerId, u32)),
    /// The next tag to fill, so each chunk walks the tag list in order.
    Tag(usize),
    Slice(usize),
    /// A tileset's own slot: the first chunk after the tileset fills it.
    Tileset(usize),
    /// The tileset's own slot is full; chunks now fill its tiles in index order.
    TilesetTiles(usize),
}

/// Parses a 0x2020 chunk and attaches it to the target the reader is holding.
pub(crate) fn read(
    data: &[u8],
    sprite: &mut Sprite,
    state: &mut ReadState,
) -> Result<(), AseError> {
    match state.target {
        Target::None => store_raw(USER_DATA, data, sprite, state),
        Target::Used => Err(duplicate()),
        Target::Sprite => {
            // The flag survives a second palette chunk, which would otherwise
            // re-open the sprite's slot.
            if state.sprite_ud {
                return Err(duplicate());
            }
            state.sprite_ud = true;
            sprite.user_data = parse(data)?;
            state.target = Target::Used;
            Ok(())
        }
        Target::Layer(id) => {
            let layer = sprite
                .layer_mut(id)
                .ok_or_else(|| orphan("the layer it follows is gone"))?;
            layer.user_data = parse(data)?;
            state.target = Target::Used;
            Ok(())
        }
        Target::Cel(key) => {
            let cel = sprite
                .cels
                .get_mut(&key)
                .ok_or_else(|| orphan("the cel it follows is gone"))?;
            cel.user_data = parse(data)?;
            state.target = Target::Used;
            Ok(())
        }
        Target::Tag(index) => {
            // One past the last tag is the documented orphan case.
            let tag = sprite
                .tags
                .get_mut(index)
                .ok_or_else(|| orphan("the tags ran out"))?;
            tag.user_data = parse(data)?;
            state.target = Target::Tag(index + 1);
            Ok(())
        }
        Target::Slice(index) => {
            let slice = sprite
                .slices
                .get_mut(index)
                .ok_or_else(|| orphan("the slice it follows is gone"))?;
            slice.user_data = parse(data)?;
            state.target = Target::Used;
            Ok(())
        }
        Target::Tileset(index) => {
            let tileset = sprite
                .tilesets
                .get_mut(index)
                .ok_or_else(|| orphan("the tileset it follows is gone"))?;
            tileset.user_data = parse(data)?;
            state.target = Target::TilesetTiles(index);
            Ok(())
        }
        Target::TilesetTiles(index) => {
            let tileset = sprite
                .tilesets
                .get_mut(index)
                .ok_or_else(|| orphan("the tileset it follows is gone"))?;
            // One chunk per tile; the next one past the last tile is the
            // documented orphan case.
            let cap = usize::try_from(tileset.tile_count).unwrap_or(usize::MAX);
            if tileset.tile_user_data.len() >= cap {
                return Err(orphan("the tileset's tiles ran out"));
            }
            tileset.tile_user_data.push(parse(data)?);
            Ok(())
        }
    }
}

/// Writes one 0x2020 chunk. Tags call this even for empty user data; every
/// other owner gates on [`UserData::is_empty`].
pub(crate) fn write_user_data(w: &mut ByteWriter, ud: &UserData) -> Result<u32, AseError> {
    let mut data = ByteWriter::new();
    let mut flags = 0u32;
    if ud.text.is_some() {
        flags |= 1;
    }
    if ud.color.is_some() {
        flags |= 2;
    }
    if !ud.properties.is_empty() {
        flags |= 4;
    }
    data.u32(flags);
    if let Some(text) = &ud.text {
        data.string(text)?;
    }
    if let Some(color) = &ud.color {
        data.bytes(color);
    }
    if !ud.properties.is_empty() {
        let mut maps = ByteWriter::new();
        for map in &ud.properties {
            maps.u32(map.key);
            maps.u32(count("property count", map.properties.len())?);
            for (name, value) in &map.properties {
                maps.string(name)?;
                maps.u16(value.type_code());
                write_value(&mut maps, value, 1)?;
            }
        }
        let maps = maps.into_vec();
        // The size counts itself and the map-count field as well.
        let size =
            count("properties size", maps.len())?
                .checked_add(8)
                .ok_or(AseError::OutOfRange {
                    what: "properties size",
                    value: i64::from(u32::MAX),
                })?;
        data.u32(size);
        data.u32(count("properties map count", ud.properties.len())?);
        data.bytes(&maps);
    }
    chunk(w, USER_DATA, &data.into_vec())?;
    Ok(1)
}

fn parse(data: &[u8]) -> Result<UserData, AseError> {
    let mut r = ByteReader::new(data);
    let flags = r.u32()?;
    let text = if flags & 1 != 0 {
        Some(r.string()?)
    } else {
        None
    };
    let color = if flags & 2 != 0 {
        let mut rgba = [0u8; 4];
        rgba.copy_from_slice(r.bytes(4)?);
        Some(rgba)
    } else {
        None
    };
    let properties = if flags & 4 != 0 {
        read_maps(&mut r)?
    } else {
        Vec::new()
    };
    Ok(UserData {
        text,
        color,
        properties,
    })
}

/// The size field is the region's only length guard: it must match what the
/// maps really occupy, counted from the start of the field itself.
fn read_maps(r: &mut ByteReader) -> Result<Vec<PropertiesMap>, AseError> {
    let start = r.pos();
    let size = r.u32()?;
    let count = r.u32()?;
    let mut maps = Vec::new();
    for _ in 0..count {
        let key = r.u32()?;
        let n = r.u32()?;
        let mut properties = Vec::new();
        for _ in 0..n {
            let name = r.string()?;
            let code = r.u16()?;
            properties.push((name, read_value(r, code, 1)?));
        }
        maps.push(PropertiesMap { key, properties });
    }
    let used = r.pos() - start;
    if size as usize != used {
        return Err(AseError::Invalid {
            what: "properties size",
            detail: format!("{size} covers {used} bytes of maps"),
        });
    }
    Ok(maps)
}

fn read_value(r: &mut ByteReader, code: u16, depth: u32) -> Result<PropValue, AseError> {
    match code {
        0x01 => Ok(PropValue::Bool(r.u8()? != 0)),
        0x02 => Ok(PropValue::I8(i8::from_le_bytes([r.u8()?]))),
        0x03 => Ok(PropValue::U8(r.u8()?)),
        0x04 => Ok(PropValue::I16(r.i16()?)),
        0x05 => Ok(PropValue::U16(r.u16()?)),
        0x06 => Ok(PropValue::I32(r.i32()?)),
        0x07 => Ok(PropValue::U32(r.u32()?)),
        0x08 => Ok(PropValue::I64(r.i64()?)),
        0x09 => Ok(PropValue::U64(r.u64()?)),
        0x0A => Ok(PropValue::Fixed(r.i32()?)),
        0x0B => Ok(PropValue::F32(r.f32()?)),
        0x0C => Ok(PropValue::F64(r.f64()?)),
        0x0D => Ok(PropValue::String(r.string()?)),
        0x0E => Ok(PropValue::Point {
            x: r.i32()?,
            y: r.i32()?,
        }),
        0x0F => Ok(PropValue::Size {
            width: r.i32()?,
            height: r.i32()?,
        }),
        0x10 => Ok(PropValue::Rect {
            x: r.i32()?,
            y: r.i32()?,
            width: r.i32()?,
            height: r.i32()?,
        }),
        0x11 => read_vector(r, depth),
        0x12 => read_map(r, depth),
        0x13 => {
            let mut uuid = [0u8; 16];
            uuid.copy_from_slice(r.bytes(16)?);
            Ok(PropValue::Uuid(uuid))
        }
        _ => Err(AseError::Invalid {
            what: "property type",
            detail: format!("{code:#06x} is not a property type"),
        }),
    }
}

fn read_vector(r: &mut ByteReader, depth: u32) -> Result<PropValue, AseError> {
    check_depth(depth)?;
    let count = r.u32()?;
    let element_type = r.u16()?;
    let mut items = Vec::new();
    for _ in 0..count {
        // A mixed vector tags each element; a uniform one stores raw values.
        let code = if element_type == 0 {
            r.u16()?
        } else {
            element_type
        };
        items.push(read_value(r, code, depth + 1)?);
    }
    Ok(PropValue::Vector {
        element_type,
        items,
    })
}

fn read_map(r: &mut ByteReader, depth: u32) -> Result<PropValue, AseError> {
    check_depth(depth)?;
    let count = r.u32()?;
    let mut properties = Vec::new();
    for _ in 0..count {
        let name = r.string()?;
        let code = r.u16()?;
        properties.push((name, read_value(r, code, depth + 1)?));
    }
    Ok(PropValue::Map(properties))
}

fn check_depth(depth: u32) -> Result<(), AseError> {
    if depth > MAX_DEPTH {
        return Err(AseError::Invalid {
            what: "properties depth",
            detail: format!("{depth} is past the {MAX_DEPTH}-level limit"),
        });
    }
    Ok(())
}

fn write_value(w: &mut ByteWriter, value: &PropValue, depth: u32) -> Result<(), AseError> {
    match value {
        PropValue::Bool(v) => w.u8(u8::from(*v)),
        PropValue::I8(v) => w.bytes(&v.to_le_bytes()),
        PropValue::U8(v) => w.u8(*v),
        PropValue::I16(v) => w.i16(*v),
        PropValue::U16(v) => w.u16(*v),
        PropValue::I32(v) => w.i32(*v),
        PropValue::U32(v) => w.u32(*v),
        PropValue::I64(v) => w.i64(*v),
        PropValue::U64(v) => w.u64(*v),
        PropValue::Fixed(v) => w.i32(*v),
        PropValue::F32(v) => w.f32(*v),
        PropValue::F64(v) => w.f64(*v),
        PropValue::String(v) => w.string(v)?,
        PropValue::Point { x, y } => {
            w.i32(*x);
            w.i32(*y);
        }
        PropValue::Size { width, height } => {
            w.i32(*width);
            w.i32(*height);
        }
        PropValue::Rect {
            x,
            y,
            width,
            height,
        } => {
            w.i32(*x);
            w.i32(*y);
            w.i32(*width);
            w.i32(*height);
        }
        PropValue::Vector {
            element_type,
            items,
        } => {
            check_depth(depth)?;
            w.u32(count("vector length", items.len())?);
            w.u16(*element_type);
            for item in items {
                if *element_type != 0 && item.type_code() != *element_type {
                    return Err(AseError::Invalid {
                        what: "property vector",
                        detail: format!(
                            "element type {element_type:#06x} but an item is {:#06x}",
                            item.type_code()
                        ),
                    });
                }
                // Only a mixed vector stores the type with each element.
                if *element_type == 0 {
                    w.u16(item.type_code());
                }
                write_value(w, item, depth + 1)?;
            }
        }
        PropValue::Map(properties) => {
            check_depth(depth)?;
            w.u32(count("property count", properties.len())?);
            for (name, v) in properties {
                w.string(name)?;
                w.u16(v.type_code());
                write_value(w, v, depth + 1)?;
            }
        }
        PropValue::Uuid(v) => w.bytes(v),
    }
    Ok(())
}

fn count(what: &'static str, n: usize) -> Result<u32, AseError> {
    u32::try_from(n).map_err(|_| AseError::OutOfRange {
        what,
        value: i64::try_from(n).unwrap_or(i64::MAX),
    })
}

fn orphan(detail: &str) -> AseError {
    AseError::Invalid {
        what: "orphan user data",
        detail: detail.to_owned(),
    }
}

fn duplicate() -> AseError {
    AseError::Invalid {
        what: "duplicate user data",
        detail: "the owner it follows already has user data".to_owned(),
    }
}
