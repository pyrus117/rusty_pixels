use rp_core::{ExternalFile, Sprite};

use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::chunk;

pub(crate) const EXTERNAL_FILES: u16 = 0x2008;

/// Parses a 0x2008 chunk. One chunk holds the whole list, so it replaces
/// whatever was read before.
pub(crate) fn read(data: &[u8], sprite: &mut Sprite) -> Result<(), AseError> {
    let mut r = ByteReader::new(data);
    let count = r.u32()?;
    r.skip(8)?;
    let mut files = Vec::new();
    for _ in 0..count {
        let id = r.u32()?;
        let kind = r.u8()?;
        r.skip(7)?;
        files.push(ExternalFile {
            id,
            kind,
            name: r.string()?,
        });
    }
    sprite.external_files = files;
    Ok(())
}

/// Writes the external files as one chunk; an empty list writes nothing.
pub(crate) fn write_external_files(
    w: &mut ByteWriter,
    files: &[ExternalFile],
) -> Result<u32, AseError> {
    if files.is_empty() {
        return Ok(0);
    }
    let count = u32::try_from(files.len()).map_err(|_| AseError::OutOfRange {
        what: "external file count",
        value: i64::try_from(files.len()).unwrap_or(i64::MAX),
    })?;
    let mut data = ByteWriter::new();
    data.u32(count);
    data.bytes(&[0; 8]);
    for file in files {
        data.u32(file.id);
        data.u8(file.kind);
        data.bytes(&[0; 7]);
        data.string(&file.name)?;
    }
    chunk(w, EXTERNAL_FILES, &data.into_vec())?;
    Ok(1)
}
