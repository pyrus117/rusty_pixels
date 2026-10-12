use rp_core::ColorProfile;

use crate::bytes::{ByteReader, ByteWriter};
use crate::error::AseError;

use super::chunk;

pub(crate) const COLOR_PROFILE: u16 = 0x2007;

pub(crate) fn read(data: &[u8]) -> Result<ColorProfile, AseError> {
    let mut r = ByteReader::new(data);
    let kind = r.u16()?;
    let flags = r.u16()?;
    let gamma = r.i32()?;
    r.skip(8)?;
    // Only an ICC profile (kind 2) carries the trailing data block.
    let icc = if kind == 2 {
        let len = r.u32()? as usize;
        r.bytes(len)?.to_vec()
    } else {
        Vec::new()
    };
    Ok(ColorProfile {
        kind,
        flags,
        gamma,
        icc,
    })
}

pub(crate) fn write_color_profile(
    w: &mut ByteWriter,
    profile: &ColorProfile,
) -> Result<u32, AseError> {
    // Only an ICC profile (kind 2) has a place for the data block; dropping it
    // would silently change the profile.
    if profile.kind != 2 && !profile.icc.is_empty() {
        return Err(AseError::Invalid {
            what: "color profile",
            detail: "icc bytes but the profile kind is not ICC".to_owned(),
        });
    }
    let mut data = ByteWriter::new();
    data.u16(profile.kind);
    data.u16(profile.flags);
    data.i32(profile.gamma);
    data.bytes(&[0; 8]);
    if profile.kind == 2 {
        let len = u32::try_from(profile.icc.len()).map_err(|_| AseError::OutOfRange {
            what: "icc profile length",
            value: i64::try_from(profile.icc.len()).unwrap_or(i64::MAX),
        })?;
        data.u32(len);
        data.bytes(&profile.icc);
    }
    chunk(w, COLOR_PROFILE, &data.into_vec())?;
    Ok(1)
}
