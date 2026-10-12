use rp_core::{Pivot, Slice, SliceCenter, SliceKey, Sprite, UserData};

use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::{add_chunks, chunk, write_user_data};

pub(crate) const SLICE: u16 = 0x2022;

/// Parses a 0x2022 chunk holding one slice. The slice's flags decide which
/// blocks follow each key. Returns the slice's index, which a following
/// user-data chunk attaches to.
pub(crate) fn read(data: &[u8], sprite: &mut Sprite) -> Result<usize, AseError> {
    let mut r = ByteReader::new(data);
    let key_count = r.u32()?;
    let flags = r.u32()?;
    r.skip(4)?;
    let name = r.string()?;
    let mut keys = Vec::new();
    for _ in 0..key_count {
        let frame = r.u32()?;
        let x = r.i32()?;
        let y = r.i32()?;
        let width = r.u32()?;
        let height = r.u32()?;
        let center = if flags & Slice::NINE_PATCH != 0 {
            Some(SliceCenter {
                x: r.i32()?,
                y: r.i32()?,
                width: r.u32()?,
                height: r.u32()?,
            })
        } else {
            None
        };
        let pivot = if flags & Slice::PIVOT != 0 {
            Some(Pivot {
                x: r.i32()?,
                y: r.i32()?,
            })
        } else {
            None
        };
        keys.push(SliceKey {
            frame,
            x,
            y,
            width,
            height,
            center,
            pivot,
        });
    }
    let index = sprite.slices.len();
    sprite.slices.push(Slice {
        name,
        flags,
        keys,
        user_data: UserData::default(),
    });
    Ok(index)
}

/// Writes one chunk per slice, each followed by its user data. A key that
/// carries the wrong blocks for its slice's flags cannot round-trip, so it is
/// rejected.
pub(crate) fn write_slices(w: &mut ByteWriter, slices: &[Slice]) -> Result<u32, AseError> {
    let mut written = 0u32;
    for slice in slices {
        let key_count = u32::try_from(slice.keys.len()).map_err(|_| AseError::OutOfRange {
            what: "slice key count",
            value: i64::try_from(slice.keys.len()).unwrap_or(i64::MAX),
        })?;
        let mut data = ByteWriter::new();
        data.u32(key_count);
        data.u32(slice.flags);
        data.u32(0);
        data.string(&slice.name)?;
        for key in &slice.keys {
            if key.center.is_some() != (slice.flags & Slice::NINE_PATCH != 0)
                || key.pivot.is_some() != (slice.flags & Slice::PIVOT != 0)
            {
                return Err(AseError::Invalid {
                    what: "slice key",
                    detail: format!(
                        "frame {} of slice {} does not match flags {}",
                        key.frame, slice.name, slice.flags
                    ),
                });
            }
            data.u32(key.frame);
            data.i32(key.x);
            data.i32(key.y);
            data.u32(key.width);
            data.u32(key.height);
            if let Some(center) = key.center {
                data.i32(center.x);
                data.i32(center.y);
                data.u32(center.width);
                data.u32(center.height);
            }
            if let Some(pivot) = key.pivot {
                data.i32(pivot.x);
                data.i32(pivot.y);
            }
        }
        chunk(w, SLICE, &data.into_vec())?;
        add_chunks(&mut written, 1)?;
        if !slice.user_data.is_empty() {
            add_chunks(&mut written, write_user_data(w, &slice.user_data)?)?;
        }
    }
    Ok(written)
}
