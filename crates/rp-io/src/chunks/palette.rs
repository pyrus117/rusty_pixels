use rp_core::{Palette, PaletteEntry, Sprite};

use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::chunk;

pub(crate) const PALETTE: u16 = 0x2019;
pub(crate) const OLD_PALETTE: u16 = 0x0004;
pub(crate) const OLD_PALETTE_6BIT: u16 = 0x0011;

/// The header's colour-count field is a WORD, so a bigger size is a lie that
/// must not reach the resize below.
const MAX_ENTRIES: u32 = 65535;

/// Parses a 0x2019 chunk: the palette grows or shrinks to the new size, then
/// the entries in [first, last] are replaced.
pub(crate) fn read(data: &[u8], sprite: &mut Sprite) -> Result<(), AseError> {
    let mut r = ByteReader::new(data);
    let size = r.u32()?;
    if size > MAX_ENTRIES {
        return Err(AseError::Invalid {
            what: "palette size",
            detail: format!("{size} entries is over the {MAX_ENTRIES} limit"),
        });
    }
    let first = r.u32()?;
    let last = r.u32()?;
    r.skip(8)?;
    if first > last || last >= size {
        return Err(range(format!(
            "{first}..{last} does not fit a palette of {size} entries"
        )));
    }
    let mut set = Vec::new();
    for _ in first..=last {
        let flags = r.u16()?;
        let mut rgba = [0u8; 4];
        rgba.copy_from_slice(r.bytes(4)?);
        let name = if flags & 1 != 0 {
            Some(r.string()?)
        } else {
            None
        };
        set.push(PaletteEntry { rgba, name });
    }
    let entries = &mut sprite.palette.entries;
    entries.resize(size as usize, blank());
    let slots = entries
        .get_mut(first as usize..=last as usize)
        .ok_or_else(|| range("the entries went missing after the resize".to_owned()))?;
    for (slot, entry) in slots.iter_mut().zip(set) {
        *slot = entry;
    }
    Ok(())
}

/// Parses an old palette chunk (0x0004/0x0011). A 0x2019 chunk anywhere in the
/// file wins, so a superseded chunk is skipped.
pub(crate) fn read_old(
    kind: u16,
    data: &[u8],
    sprite: &mut Sprite,
    superseded: bool,
) -> Result<(), AseError> {
    if superseded {
        return Ok(());
    }
    let mut r = ByteReader::new(data);
    let packets = r.u16()?;
    let mut entries = Vec::new();
    let mut cursor = 0usize;
    for _ in 0..packets {
        cursor += usize::from(r.u8()?);
        // A count byte of 0 encodes 256.
        let count = match r.u8()? {
            0 => 256,
            n => usize::from(n),
        };
        let end = cursor + count;
        // The format's palette stops at 256 entries; a packet past that has
        // nowhere to go.
        if end > 256 {
            return Err(AseError::Invalid {
                what: "old palette",
                detail: format!("packet at {cursor} reaches entry index 256 or beyond"),
            });
        }
        if end > entries.len() {
            entries.resize(end, blank());
        }
        let slots = entries.get_mut(cursor..end).ok_or_else(|| {
            range(format!(
                "packet at {cursor} covers {count} colours past the palette"
            ))
        })?;
        for slot in slots {
            *slot = PaletteEntry {
                rgba: colour(&mut r, kind == OLD_PALETTE_6BIT)?,
                name: None,
            };
        }
        cursor = end;
    }
    sprite.palette.entries = entries;
    Ok(())
}

/// Writes every entry as one 0x2019 chunk; an empty palette writes nothing.
pub(crate) fn write_palette(w: &mut ByteWriter, palette: &Palette) -> Result<u32, AseError> {
    let entries = &palette.entries;
    if entries.is_empty() {
        return Ok(0);
    }
    let size = u32::try_from(entries.len()).map_err(|_| AseError::OutOfRange {
        what: "palette size",
        value: i64::try_from(entries.len()).unwrap_or(i64::MAX),
    })?;
    let mut data = ByteWriter::new();
    data.u32(size);
    data.u32(0);
    data.u32(size - 1); // the whole palette is written, so first is 0
    data.bytes(&[0; 8]);
    for entry in entries {
        // A name costs a flag, a length and the bytes, so it appears only when set.
        data.u16(u16::from(entry.name.is_some()));
        data.bytes(&entry.rgba);
        if let Some(name) = &entry.name {
            data.string(name)?;
        }
    }
    chunk(w, PALETTE, &data.into_vec())?;
    Ok(1)
}

fn colour(r: &mut ByteReader, six_bit: bool) -> Result<[u8; 4], AseError> {
    let mut rgba = [0u8; 4];
    for channel in &mut rgba[..3] {
        let v = r.u8()?;
        *channel = if six_bit { scale6(v)? } else { v };
    }
    rgba[3] = 255;
    Ok(rgba)
}

/// 0x0011 stores 6-bit VGA values; the formula is the spec's scaling.
fn scale6(v: u8) -> Result<u8, AseError> {
    u8::try_from(u16::from(v) * 255 / 63).map_err(|_| AseError::Invalid {
        what: "old palette",
        detail: format!("{v} is outside the 0-63 range"),
    })
}

fn blank() -> PaletteEntry {
    PaletteEntry {
        rgba: [0, 0, 0, 255],
        name: None,
    }
}

fn range(detail: String) -> AseError {
    AseError::Invalid {
        what: "palette range",
        detail,
    }
}
