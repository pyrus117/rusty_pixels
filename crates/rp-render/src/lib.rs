//! CPU compositor for Rusty Pixels: flattens a sprite frame into an RgbaImage.

mod blend;
mod composite;

use rp_core::{CoreError, LayerId, RgbaImage, Sprite};

/// Which layers `render_frame` draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerFilter {
    /// The layer and every ancestor must be visible.
    Visible,
    /// Every layer, whatever its visibility.
    All,
    /// The named layers with their subtrees, plus any layer under a named
    /// ancestor. Visibility is ignored; ancestor group opacity and blend
    /// still apply.
    Only(Vec<LayerId>),
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub layers: LayerFilter,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            layers: LayerFilter::Visible,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("frame {0} does not exist")]
    UnknownFrame(u32),
    #[error("layer {0:?} does not exist")]
    UnknownLayer(LayerId),
    #[error("layer {layer:?} frame {frame}: {reason}")]
    BadCel {
        layer: LayerId,
        frame: u32,
        reason: String,
    },
    #[error(transparent)]
    Image(#[from] CoreError),
    #[error("scale factor {0} is outside 1..=64")]
    BadFactor(u32),
}

/// Flattens one frame of `sprite` to a canvas-sized RGBA8 image.
pub fn render_frame(
    sprite: &Sprite,
    frame: u32,
    opts: &RenderOptions,
) -> Result<RgbaImage, RenderError> {
    composite::render(sprite, frame, opts)
}

/// Nearest-neighbour scale: each pixel becomes a `factor` x `factor` block.
pub fn scale_nearest(img: &RgbaImage, factor: u32) -> Result<RgbaImage, RenderError> {
    if !(1..=64).contains(&factor) {
        return Err(RenderError::BadFactor(factor));
    }
    // Saturating keeps the hostile product big enough for new() to reject.
    let mut out = RgbaImage::new(
        img.width().saturating_mul(factor),
        img.height().saturating_mul(factor),
    )?;
    for y in 0..img.height() {
        for x in 0..img.width() {
            let Some(px) = img.get(x, y) else {
                continue;
            };
            for dy in 0..factor {
                for dx in 0..factor {
                    out.put(x * factor + dx, y * factor + dy, px)?;
                }
            }
        }
    }
    Ok(out)
}
