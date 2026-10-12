use png::{BitDepth, ColorType, Encoder};
use rp_core::RgbaImage;

use super::ExportError;

/// 8-bit RGBA, non-interlaced.
pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, ExportError> {
    let mut out = Vec::new();
    let mut enc = Encoder::new(&mut out, img.width(), img.height());
    enc.set_color(ColorType::Rgba);
    enc.set_depth(BitDepth::Eight);
    let mut writer = enc.write_header().map_err(encode_err)?;
    writer.write_image_data(img.data()).map_err(encode_err)?;
    drop(writer);
    Ok(out)
}

fn encode_err(e: png::EncodingError) -> ExportError {
    ExportError::Encode(e.to_string())
}
