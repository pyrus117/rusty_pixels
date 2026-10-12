use std::collections::HashMap;

use gif::DisposalMethod;
use rp_core::RgbaImage;

use super::ExportError;

/// One animation frame: a full canvas image shown for `duration_ms`.
#[derive(Debug, Clone, Copy)]
pub struct GifFrame<'a> {
    pub image: &'a RgbaImage,
    pub duration_ms: u16,
}

/// Every frame covers the whole canvas and disposes to Background, so each
/// frame stands alone no matter what the previous one drew.
pub fn encode_gif(frames: &[GifFrame], looping: bool) -> Result<Vec<u8>, ExportError> {
    let first = frames.first().ok_or(ExportError::Empty)?;
    let (width, height) = (first.image.width(), first.image.height());
    if frames
        .iter()
        .any(|f| f.image.width() != width || f.image.height() != height)
    {
        return Err(ExportError::MixedSizes);
    }
    let (width, height) = (canvas_dim(width)?, canvas_dim(height)?);
    let mut enc = gif::Encoder::new(Vec::new(), width, height, &[]).map_err(encode_err)?;
    // Play-once is stored as no repeat block at all; Finite(0) writes none.
    enc.set_repeat(if looping {
        gif::Repeat::Infinite
    } else {
        gif::Repeat::Finite(0)
    })
    .map_err(encode_err)?;
    for f in frames {
        let mut frame = gif_frame(f.image)?;
        frame.delay = f.duration_ms / 10;
        frame.dispose = DisposalMethod::Background;
        enc.write_frame(&frame).map_err(encode_err)?;
    }
    enc.into_inner().map_err(encode_err)
}

fn canvas_dim(v: u32) -> Result<u16, ExportError> {
    u16::try_from(v).map_err(|_| ExportError::Encode(format!("{v} exceeds the GIF limit of 65535")))
}

fn encode_err(e: gif::EncodingError) -> ExportError {
    ExportError::Encode(e.to_string())
}

/// Alpha 0 is transparent, any other alpha is opaque with its RGB kept
/// (observed). An exact palette needs one index for transparency, so it only
/// holds 255 opaque colours; beyond that the gif crate quantises.
fn gif_frame(image: &RgbaImage) -> Result<gif::Frame<'static>, ExportError> {
    let (width, height) = (canvas_dim(image.width())?, canvas_dim(image.height())?);
    let mut palette: Vec<u8> = vec![0, 0, 0];
    let mut lookup: HashMap<[u8; 3], u8> = HashMap::new();
    let mut indices = Vec::with_capacity(image.data().len() / 4);
    for px in image.data().chunks_exact(4) {
        if px[3] == 0 {
            indices.push(0);
            continue;
        }
        let rgb = [px[0], px[1], px[2]];
        let index = match lookup.get(&rgb) {
            Some(&i) => i,
            None => {
                if lookup.len() == 255 {
                    let mut pixels = image.data().to_vec();
                    return Ok(gif::Frame::from_rgba(width, height, &mut pixels));
                }
                let i = lookup.len() as u8 + 1;
                lookup.insert(rgb, i);
                palette.extend_from_slice(&rgb);
                i
            }
        };
        indices.push(index);
    }
    Ok(gif::Frame::from_palette_pixels(
        width,
        height,
        indices,
        palette,
        Some(0),
    ))
}
