//! Model and pixel comparison for the corpus round-trip tool.

use anyhow::{Context, Result, bail};
use rp_core::Sprite;

/// An RGBA8 image. `pixels` is row-major, four bytes per pixel.
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Names the first top-level field where the models differ. The order matches
/// the `Sprite` declaration order, so one report names one field. The
/// destructure is exhaustive on purpose: a new `Sprite` field must be added
/// here before this compiles again.
pub fn first_diff(a: &Sprite, b: &Sprite) -> Option<&'static str> {
    let Sprite {
        width,
        height,
        color_mode,
        flags,
        transparent_index,
        pixel_width,
        pixel_height,
        grid,
        layers,
        frames,
        cels,
        tags,
        palette,
        slices,
        tilesets,
        color_profile,
        external_files,
        user_data,
        raw_chunks,
    } = a;
    let diffs = [
        ("width", *width != b.width),
        ("height", *height != b.height),
        ("color_mode", *color_mode != b.color_mode),
        ("flags", *flags != b.flags),
        (
            "transparent_index",
            *transparent_index != b.transparent_index,
        ),
        ("pixel_width", *pixel_width != b.pixel_width),
        ("pixel_height", *pixel_height != b.pixel_height),
        ("grid", *grid != b.grid),
        ("layers", *layers != b.layers),
        ("frames", *frames != b.frames),
        ("cels", *cels != b.cels),
        ("tags", *tags != b.tags),
        ("palette", *palette != b.palette),
        ("slices", *slices != b.slices),
        ("tilesets", *tilesets != b.tilesets),
        ("color_profile", *color_profile != b.color_profile),
        ("external_files", *external_files != b.external_files),
        ("user_data", *user_data != b.user_data),
        ("raw_chunks", *raw_chunks != b.raw_chunks),
    ];
    diffs
        .into_iter()
        .find(|(_, differ)| *differ)
        .map(|(name, _)| name)
}

/// Decodes a PNG to RGBA8, expanding indexed and greyscale images on the way.
pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage> {
    // png 0.18 seeks while reading; a cursor gives it Seek over the buffer.
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // A single sample layout keeps the pixel compare down to byte equality.
    decoder.set_transformations(
        png::Transformations::EXPAND | png::Transformations::ALPHA | png::Transformations::STRIP_16,
    );
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size().context("image too large")?];
    let info = reader.next_frame(&mut buf)?;
    let color_type = info.color_type;
    let samples = color_type.samples();
    let mut pixels = Vec::with_capacity(buf.len() / samples * 4);
    for src in buf.chunks_exact(samples) {
        let rgba = match (color_type, src) {
            (png::ColorType::Rgba, [r, g, b, a]) => [*r, *g, *b, *a],
            (png::ColorType::Rgb, [r, g, b]) => [*r, *g, *b, 255],
            (png::ColorType::GrayscaleAlpha, [v, a]) => [*v, *v, *v, *a],
            (png::ColorType::Grayscale, [v]) => [*v, *v, *v, 255],
            (color_type, _) => bail!("colour type {color_type:?} survived expansion"),
        };
        pixels.extend_from_slice(&rgba);
    }
    Ok(RgbaImage {
        width: info.width,
        height: info.height,
        pixels,
    })
}

/// Counts the pixel positions that differ. Positions outside the shared corner
/// exist in only one image, so they all differ.
pub fn diff_pixels(a: &RgbaImage, b: &RgbaImage) -> usize {
    let mut count = 0;
    for y in 0..a.height.max(b.height) {
        for x in 0..a.width.max(b.width) {
            if pixel(a, x, y) != pixel(b, x, y) {
                count += 1;
            }
        }
    }
    count
}

fn pixel(img: &RgbaImage, x: u32, y: u32) -> Option<&[u8]> {
    if x >= img.width || y >= img.height {
        return None;
    }
    let i = (y as usize * img.width as usize + x as usize) * 4;
    img.pixels.get(i..i + 4)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use rp_core::{
        BlendMode, Cel, CelContent, ColorMode, Image, Layer, LayerFlags, LayerId, LayerKind,
        Sprite, UserData,
    };

    use super::*;

    fn layer(id: u32) -> Layer {
        Layer {
            id: LayerId(id),
            parent: None,
            name: format!("L{id}"),
            kind: LayerKind::Image,
            flags: LayerFlags(LayerFlags::VISIBLE),
            blend_mode: BlendMode::Normal,
            opacity: 255,
            uuid: None,
            user_data: UserData::default(),
        }
    }

    fn cel() -> Cel {
        Cel {
            x: 0,
            y: 0,
            opacity: 255,
            z_index: 0,
            content: CelContent::Image(Image::new(1, 1, ColorMode::Rgba).unwrap()),
            extra: None,
            user_data: UserData::default(),
        }
    }

    fn encode_with(
        color: png::ColorType,
        width: u32,
        height: u32,
        data: &[u8],
        palette: Option<Vec<u8>>,
        trns: Option<Vec<u8>>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(color);
        enc.set_depth(png::BitDepth::Eight);
        if let Some(palette) = palette {
            enc.set_palette(palette);
        }
        if let Some(trns) = trns {
            enc.set_trns(trns);
        }
        enc.write_header().unwrap().write_image_data(data).unwrap();
        out
    }

    fn encode(color: png::ColorType, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
        encode_with(color, width, height, data, None, None)
    }

    #[test]
    fn first_diff_names_the_first_differing_field() {
        let a = Sprite::new(4, 4, ColorMode::Rgba);
        assert_eq!(first_diff(&a, &a.clone()), None);

        // width precedes layers, so the report names width alone.
        let mut both = a.clone();
        both.width = 5;
        both.layers.push(layer(0));
        assert_eq!(first_diff(&a, &both), Some("width"));

        let mut by_layers = a.clone();
        by_layers.layers.push(layer(0));
        assert_eq!(first_diff(&a, &by_layers), Some("layers"));

        let mut by_cels = a.clone();
        by_cels.cels.insert((LayerId(0), 0), cel());
        assert_eq!(first_diff(&a, &by_cels), Some("cels"));
    }

    #[test]
    fn decode_png_expands_to_rgba8() {
        let rgba = decode_png(&encode(
            png::ColorType::Rgba,
            2,
            1,
            &[1, 2, 3, 4, 5, 6, 7, 8],
        ))
        .unwrap();
        assert_eq!((rgba.width, rgba.height), (2, 1));
        assert_eq!(rgba.pixels, [1, 2, 3, 4, 5, 6, 7, 8]);

        let rgb = decode_png(&encode(png::ColorType::Rgb, 1, 1, &[9, 10, 11])).unwrap();
        assert_eq!(rgb.pixels, [9, 10, 11, 255]);

        let grey = decode_png(&encode(png::ColorType::Grayscale, 2, 1, &[7, 200])).unwrap();
        assert_eq!(grey.pixels, [7, 7, 7, 255, 200, 200, 200, 255]);

        let grey_a = decode_png(&encode(png::ColorType::GrayscaleAlpha, 1, 1, &[50, 60])).unwrap();
        assert_eq!(grey_a.pixels, [50, 50, 50, 60]);
    }

    #[test]
    fn decode_png_expands_the_palette_to_rgba8() {
        let palette = vec![255, 0, 0, 0, 255, 0];
        let opaque = decode_png(&encode_with(
            png::ColorType::Indexed,
            2,
            1,
            &[0, 1],
            Some(palette.clone()),
            None,
        ))
        .unwrap();
        assert_eq!(opaque.pixels, [255, 0, 0, 255, 0, 255, 0, 255]);

        let keyed = decode_png(&encode_with(
            png::ColorType::Indexed,
            2,
            1,
            &[0, 1],
            Some(palette),
            Some(vec![0, 128]),
        ))
        .unwrap();
        assert_eq!(keyed.pixels, [255, 0, 0, 0, 0, 255, 0, 128]);
    }

    #[test]
    fn diff_pixels_counts_mismatched_positions() {
        let a = RgbaImage {
            width: 2,
            height: 1,
            pixels: vec![1, 1, 1, 1, 2, 2, 2, 2],
        };
        let same = RgbaImage {
            width: 2,
            height: 1,
            pixels: vec![1, 1, 1, 1, 2, 2, 2, 2],
        };
        assert_eq!(diff_pixels(&a, &same), 0);

        let one_off = RgbaImage {
            width: 2,
            height: 1,
            pixels: vec![1, 1, 1, 1, 9, 9, 9, 9],
        };
        assert_eq!(diff_pixels(&a, &one_off), 1);

        // The pixel only the wider image holds counts as a difference.
        let wide = RgbaImage {
            width: 3,
            height: 1,
            pixels: vec![1, 1, 1, 1, 2, 2, 2, 2, 0, 0, 0, 0],
        };
        assert_eq!(diff_pixels(&a, &wide), 1);
    }
}
