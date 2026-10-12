use rp_core::{ColorMode, Frame, Grid, HeaderFlags, Sprite};

use crate::ChunkInfo;
use crate::bytes::ByteReader;
use crate::chunks;
use crate::error::AseError;

const HEADER_MAGIC: u16 = 0xA5E0;
const FRAME_MAGIC: u16 = 0xF1FA;
const FRAME_HEADER: usize = 16;
const CHUNK_HEADER: usize = 6;

struct FileHeader {
    frames: u16,
    width: u16,
    height: u16,
    color_mode: ColorMode,
    flags: u32,
    speed: u16,
    transparent_index: u8,
    pixel_width: u8,
    pixel_height: u8,
    grid: Grid,
}

struct FrameHeader {
    duration: u16,
    chunk_count: u32,
    body_len: usize,
}

/// Parses an .aseprite file into a [`Sprite`].
pub fn read(bytes: &[u8]) -> Result<Sprite, AseError> {
    let mut r = ByteReader::new(bytes);
    let h = parse_header(&mut r)?;
    // A file with no frames holds no sprite.
    if h.frames == 0 {
        return Err(AseError::Invalid {
            what: "frames",
            detail: "the header says the file has no frames".to_owned(),
        });
    }
    let mut sprite = Sprite::new(h.width, h.height, h.color_mode);
    sprite.flags = HeaderFlags(h.flags);
    sprite.transparent_index = h.transparent_index;
    sprite.pixel_width = h.pixel_width;
    sprite.pixel_height = h.pixel_height;
    sprite.grid = h.grid;
    // The 0x2007 chunk sets the profile; absence means None.
    sprite.color_profile = None;

    let mut frames = Vec::new();
    let mut state = chunks::ReadState::default();
    for index in 0..u32::from(h.frames) {
        let fh = read_frame_header(&mut r, index)?;
        let mut body = ByteReader::new(r.bytes(fh.body_len)?);
        state.start_frame(index);
        for _ in 0..fh.chunk_count {
            let (kind, _size, data) = next_chunk(&mut body)?;
            chunks::apply(kind, data, &mut sprite, &mut state)?;
        }
        frames.push(Frame {
            duration_ms: fh.duration,
        });
    }

    // Tile user data comes padded to the tile count; the model keeps it
    // trailing-trimmed.
    for tileset in &mut sprite.tilesets {
        while tileset
            .tile_user_data
            .last()
            .is_some_and(|ud| ud.is_empty())
        {
            tileset.tile_user_data.pop();
        }
    }

    // Old files carry one duration in the header and zeros per frame.
    let speed = if h.speed == 0 { 100 } else { h.speed };
    for frame in &mut frames {
        if frame.duration_ms == 0 {
            frame.duration_ms = speed;
        }
    }
    sprite.frames = frames;
    // Tags name frames, so the ranges only make sense once every frame is in.
    let frame_count = u32::try_from(sprite.frames.len()).unwrap_or(u32::MAX);
    for tag in &sprite.tags {
        if tag.from > tag.to || tag.to >= frame_count {
            return Err(AseError::Invalid {
                what: "tag range",
                detail: format!(
                    "tag {} covers {}..{} but the file has {frame_count} frames",
                    tag.name, tag.from, tag.to
                ),
            });
        }
    }
    Ok(sprite)
}

/// Lists every chunk in the file without decoding chunk bodies.
pub fn dump_chunks(bytes: &[u8]) -> Result<Vec<ChunkInfo>, AseError> {
    let mut r = ByteReader::new(bytes);
    let h = parse_header(&mut r)?;
    let mut out = Vec::new();
    for index in 0..u32::from(h.frames) {
        let fh = read_frame_header(&mut r, index)?;
        let mut body = ByteReader::new(r.bytes(fh.body_len)?);
        for _ in 0..fh.chunk_count {
            let (kind, size, _data) = next_chunk(&mut body)?;
            out.push(ChunkInfo {
                frame: index,
                kind,
                size,
            });
        }
    }
    Ok(out)
}

fn parse_header(r: &mut ByteReader) -> Result<FileHeader, AseError> {
    r.u32()?; // the file-size field is advisory; the byte slice bounds the parse
    let magic = r.u16()?;
    if magic != HEADER_MAGIC {
        return Err(AseError::BadMagic { found: magic });
    }
    let frames = r.u16()?;
    let width = r.u16()?;
    let height = r.u16()?;
    let depth = r.u16()?;
    let color_mode = ColorMode::from_depth_bits(depth).ok_or(AseError::UnsupportedDepth(depth))?;
    let flags = r.u32()?;
    let speed = r.u16()?;
    r.skip(8)?; // two reserved DWORDs
    let transparent_index = r.u8()?;
    r.skip(3)?;
    r.u16()?; // palette chunks carry the colours
    let pixel_width = r.u8()?;
    let pixel_height = r.u8()?;
    let grid = Grid {
        x: r.i16()?,
        y: r.i16()?,
        width: r.u16()?,
        height: r.u16()?,
    };
    r.skip(84)?;
    Ok(FileHeader {
        frames,
        width,
        height,
        color_mode,
        flags,
        speed,
        transparent_index,
        pixel_width,
        pixel_height,
        grid,
    })
}

fn read_frame_header(r: &mut ByteReader, frame: u32) -> Result<FrameHeader, AseError> {
    let size = r.u32()?;
    let magic = r.u16()?;
    if magic != FRAME_MAGIC {
        return Err(AseError::BadFrameMagic {
            frame,
            found: magic,
        });
    }
    let old = r.u16()?;
    let duration = r.u16()?;
    r.skip(2)?;
    let new = r.u32()?;
    if size < FRAME_HEADER as u32 {
        return Err(AseError::Invalid {
            what: "frame size",
            detail: format!("{size} is below the {FRAME_HEADER}-byte frame header"),
        });
    }
    Ok(FrameHeader {
        duration,
        chunk_count: if new != 0 { new } else { u32::from(old) },
        body_len: size as usize - FRAME_HEADER,
    })
}

/// One chunk from the frame body, its data sliced to exactly `size - 6` bytes
/// so no chunk parser can read into the chunk after it.
fn next_chunk<'a>(r: &mut ByteReader<'a>) -> Result<(u16, u32, &'a [u8]), AseError> {
    let available = r.remaining();
    if available < CHUNK_HEADER {
        return Err(AseError::Invalid {
            what: "chunk size",
            detail: format!("{available} bytes left in the frame, not even a chunk header"),
        });
    }
    let size = r.u32()?;
    let kind = r.u16()?;
    if size < CHUNK_HEADER as u32 {
        return Err(AseError::Invalid {
            what: "chunk size",
            detail: format!("{size} is below the {CHUNK_HEADER}-byte chunk header"),
        });
    }
    if size as usize > available {
        return Err(AseError::Invalid {
            what: "chunk size",
            detail: format!("chunk of {size} bytes runs past its frame"),
        });
    }
    let data = r.bytes(size as usize - CHUNK_HEADER)?;
    Ok((kind, size, data))
}
