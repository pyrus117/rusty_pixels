use rp_core::{AniDir, Sprite, Tag, UserData};

use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::{add_chunks, chunk, write_user_data};

pub(crate) const TAGS: u16 = 0x2018;

/// Parses a 0x2018 chunk. One chunk holds every tag, so it replaces whatever
/// was read before.
pub(crate) fn read(data: &[u8], sprite: &mut Sprite) -> Result<(), AseError> {
    let mut r = ByteReader::new(data);
    let count = r.u16()?;
    r.skip(8)?;
    let mut tags = Vec::new();
    for _ in 0..count {
        let from = u32::from(r.u16()?);
        let to = u32::from(r.u16()?);
        let dir = r.u8()?;
        let direction = AniDir::from_u8(dir).ok_or_else(|| AseError::Invalid {
            what: "tag direction",
            detail: format!("{dir} is not an animation direction"),
        })?;
        let repeat = r.u16()?;
        r.skip(6)?;
        let mut legacy_rgb = [0u8; 3];
        legacy_rgb.copy_from_slice(r.bytes(3)?);
        r.skip(1)?;
        let name = r.string()?;
        tags.push(Tag {
            from,
            to,
            direction,
            repeat,
            legacy_rgb,
            name,
            user_data: UserData::default(),
        });
    }
    sprite.tags = tags;
    Ok(())
}

/// Writes every tag as one chunk, then one user-data chunk per tag even when
/// it is empty, which is what Aseprite does. An empty list writes nothing.
pub(crate) fn write_tags(w: &mut ByteWriter, tags: &[Tag]) -> Result<u32, AseError> {
    if tags.is_empty() {
        return Ok(0);
    }
    let count = u16::try_from(tags.len()).map_err(|_| AseError::OutOfRange {
        what: "tag count",
        value: i64::try_from(tags.len()).unwrap_or(i64::MAX),
    })?;
    let mut data = ByteWriter::new();
    data.u16(count);
    data.bytes(&[0; 8]);
    for tag in tags {
        data.u16(checked_u16(tag.from, "tag frame")?);
        data.u16(checked_u16(tag.to, "tag frame")?);
        data.u8(tag.direction.as_u8());
        data.u16(tag.repeat);
        data.bytes(&[0; 6]);
        data.bytes(&tag.legacy_rgb);
        data.u8(0);
        data.string(&tag.name)?;
    }
    chunk(w, TAGS, &data.into_vec())?;
    let mut written = 1;
    for tag in tags {
        add_chunks(&mut written, write_user_data(w, &tag.user_data)?)?;
    }
    Ok(written)
}

fn checked_u16(value: u32, what: &'static str) -> Result<u16, AseError> {
    u16::try_from(value).map_err(|_| AseError::OutOfRange {
        what,
        value: i64::from(value),
    })
}
