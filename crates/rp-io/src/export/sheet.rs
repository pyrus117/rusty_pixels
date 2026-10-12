use rp_core::{AniDir, BlendMode, Layer, LayerKind, RgbaImage, Tag};

use super::ExportError;

/// Our app URL in place of Aseprite's in the JSON meta.
const APP: &str = "https://github.com/pyrus117/rusty_pixels";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetType {
    Horizontal,
    Vertical,
    Rows { columns: u32 },
    Columns { rows: u32 },
}

#[derive(Debug, Clone)]
pub struct SheetFrame<'a> {
    pub image: &'a RgbaImage,
    pub name: String,
    pub duration_ms: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct SheetRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone)]
pub struct Sheet {
    pub image: RgbaImage,
    pub rects: Vec<SheetRect>,
}

/// Places equal-sized frames on one canvas with no padding. Unused cells stay
/// transparent. A row or column count above the frame count is clamped to it.
pub fn layout_sheet(frames: &[SheetFrame], ty: SheetType) -> Result<Sheet, ExportError> {
    let first = frames.first().ok_or(ExportError::Empty)?;
    let (fw, fh) = (first.image.width(), first.image.height());
    if frames
        .iter()
        .any(|f| f.image.width() != fw || f.image.height() != fh)
    {
        return Err(ExportError::MixedSizes);
    }
    let count = u32::try_from(frames.len())
        .map_err(|_| ExportError::BadLayout("too many frames".to_string()))?;
    let (cols, rows) = match ty {
        SheetType::Horizontal => (count, 1),
        SheetType::Vertical => (1, count),
        SheetType::Rows { columns } => {
            if columns == 0 {
                return Err(ExportError::BadLayout(
                    "columns must be at least 1".to_string(),
                ));
            }
            let cols = columns.min(count);
            (cols, count.div_ceil(cols))
        }
        SheetType::Columns { rows } => {
            if rows == 0 {
                return Err(ExportError::BadLayout(
                    "rows must be at least 1".to_string(),
                ));
            }
            let rows = rows.min(count);
            (count.div_ceil(rows), rows)
        }
    };
    let sw = cols
        .checked_mul(fw)
        .ok_or_else(|| ExportError::BadLayout("sheet width overflows".to_string()))?;
    let sh = rows
        .checked_mul(fh)
        .ok_or_else(|| ExportError::BadLayout("sheet height overflows".to_string()))?;
    let mut data = RgbaImage::new(sw, sh)?.into_raw();
    let column_major = matches!(ty, SheetType::Vertical | SheetType::Columns { .. });
    let mut rects = Vec::with_capacity(frames.len());
    for (i, frame) in (0..count).zip(frames) {
        let (cx, cy) = if column_major {
            (i / rows, i % rows)
        } else {
            (i % cols, i / cols)
        };
        let rect = SheetRect {
            x: cx * fw,
            y: cy * fh,
            w: fw,
            h: fh,
        };
        blit(&mut data, sw, frame.image, &rect)?;
        rects.push(rect);
    }
    Ok(Sheet {
        image: RgbaImage::from_raw(sw, sh, data)?,
        rects,
    })
}

/// Copies one frame into the sheet buffer row by row. The geometry above keeps
/// every slice in bounds, but the copy checks anyway rather than panicking.
fn blit(data: &mut [u8], sw: u32, image: &RgbaImage, rect: &SheetRect) -> Result<(), ExportError> {
    let src = image.data();
    let row_bytes = rect.w as usize * 4;
    for row in 0..rect.h as usize {
        let src_off = row * row_bytes;
        let dst_off = (rect.y as usize + row) * (sw as usize * 4) + rect.x as usize * 4;
        let (dst, s) = data
            .get_mut(dst_off..dst_off + row_bytes)
            .zip(src.get(src_off..src_off + row_bytes))
            .ok_or_else(|| ExportError::BadLayout("frame does not fit the sheet".to_string()))?;
        dst.copy_from_slice(s);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonFormat {
    Hash,
    Array,
}

pub struct SheetMeta<'a> {
    pub image_name: &'a str,
    pub tags: &'a [Tag],
    pub layers: Option<&'a [Layer]>,
}

/// Aseprite's `--data` layout, key for key. serde_json's Map sorts keys, so the
/// JSON is written in order here and `serde_json` only escapes strings.
pub fn sheet_json(
    frames: &[SheetFrame],
    sheet: &Sheet,
    format: JsonFormat,
    meta: &SheetMeta,
) -> Result<String, ExportError> {
    if frames.len() != sheet.rects.len() {
        return Err(ExportError::BadLayout(format!(
            "{} frames for {} sheet rects",
            frames.len(),
            sheet.rects.len()
        )));
    }
    let mut out = String::from("{\"frames\":");
    match format {
        JsonFormat::Array => {
            out.push('[');
            for (i, (frame, rect)) in frames.iter().zip(&sheet.rects).enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str("{\"filename\":");
                out.push_str(&esc(&frame.name)?);
                out.push(',');
                push_frame(&mut out, frame, rect);
                out.push('}');
            }
            out.push(']');
        }
        JsonFormat::Hash => {
            out.push('{');
            for (i, (frame, rect)) in frames.iter().zip(&sheet.rects).enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&esc(&frame.name)?);
                out.push_str(":{");
                push_frame(&mut out, frame, rect);
                out.push('}');
            }
            out.push('}');
        }
    }
    out.push_str(",\"meta\":{\"app\":");
    out.push_str(&esc(APP)?);
    out.push_str(",\"version\":");
    out.push_str(&esc(env!("CARGO_PKG_VERSION"))?);
    out.push_str(",\"image\":");
    out.push_str(&esc(meta.image_name)?);
    out.push_str(&format!(
        ",\"format\":\"RGBA8888\",\"size\":{{\"w\":{},\"h\":{}}},\"scale\":\"1\"",
        sheet.image.width(),
        sheet.image.height()
    ));
    if !meta.tags.is_empty() {
        out.push_str(",\"frameTags\":[");
        for (i, tag) in meta.tags.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":");
            out.push_str(&esc(&tag.name)?);
            out.push_str(&format!(
                ",\"from\":{},\"to\":{},\"direction\":",
                tag.from, tag.to
            ));
            out.push_str(&esc(direction_name(tag.direction))?);
            out.push('}');
        }
        out.push(']');
    }
    if let Some(layers) = meta.layers {
        out.push_str(",\"layers\":[");
        for (i, layer) in layers.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            push_layer(&mut out, layers, layer)?;
        }
        out.push(']');
    }
    out.push_str("}}");
    Ok(out)
}

/// The frame object's keys, without the enclosing braces. The name is the key
/// in Hash format and `filename` before these in Array format.
fn push_frame(out: &mut String, frame: &SheetFrame, rect: &SheetRect) {
    let (w, h) = (frame.image.width(), frame.image.height());
    out.push_str(&format!(
        "\"frame\":{{\"x\":{},\"y\":{},\"w\":{},\"h\":{}}},\"rotated\":false,\"trimmed\":false,\
         \"spriteSourceSize\":{{\"x\":0,\"y\":0,\"w\":{w},\"h\":{h}}},\
         \"sourceSize\":{{\"w\":{w},\"h\":{h}}},\"duration\":{}",
        rect.x, rect.y, rect.w, rect.h, frame.duration_ms
    ));
}

/// A group layer is name (and parent group) only; an image layer adds opacity
/// and blend mode. `group` names the parent only when the parent is listed too.
fn push_layer(out: &mut String, layers: &[Layer], layer: &Layer) -> Result<(), ExportError> {
    out.push_str("{\"name\":");
    out.push_str(&esc(&layer.name)?);
    if let Some(parent) = layer
        .parent
        .and_then(|pid| layers.iter().find(|l| l.id == pid))
    {
        out.push_str(",\"group\":");
        out.push_str(&esc(&parent.name)?);
    }
    if layer.kind != LayerKind::Group {
        out.push_str(&format!(",\"opacity\":{},\"blendMode\":", layer.opacity));
        out.push_str(&esc(blend_name(layer.blend_mode))?);
    }
    out.push('}');
    Ok(())
}

fn esc(s: &str) -> Result<String, ExportError> {
    serde_json::to_string(s).map_err(|e| ExportError::Encode(e.to_string()))
}

fn direction_name(d: AniDir) -> &'static str {
    match d {
        AniDir::Forward => "forward",
        AniDir::Reverse => "reverse",
        AniDir::PingPong => "pingpong",
        AniDir::PingPongReverse => "pingpong_reverse",
    }
}

fn blend_name(m: BlendMode) -> &'static str {
    match m {
        BlendMode::Normal => "normal",
        BlendMode::Multiply => "multiply",
        BlendMode::Screen => "screen",
        BlendMode::Overlay => "overlay",
        BlendMode::Darken => "darken",
        BlendMode::Lighten => "lighten",
        BlendMode::ColorDodge => "color_dodge",
        BlendMode::ColorBurn => "color_burn",
        BlendMode::HardLight => "hard_light",
        BlendMode::SoftLight => "soft_light",
        BlendMode::Difference => "difference",
        BlendMode::Exclusion => "exclusion",
        BlendMode::Hue => "hsl_hue",
        BlendMode::Saturation => "hsl_saturation",
        BlendMode::Color => "hsl_color",
        BlendMode::Luminosity => "hsl_luminosity",
        BlendMode::Addition => "addition",
        BlendMode::Subtract => "subtract",
        BlendMode::Divide => "divide",
    }
}
