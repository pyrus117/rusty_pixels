#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Export encoders. Sheet JSON is compared against Aseprite's own --data output
//! for the same geometry (tests/fixtures/export/, observed 2026-10-12).
use std::path::Path;

use rp_core::*;
use rp_io::*;
use serde_json::Value;

const APP: &str = "https://github.com/pyrus117/rusty_pixels";

fn solid(w: u32, h: u32, rgba: [u8; 4]) -> RgbaImage {
    RgbaImage::from_raw(w, h, rgba.repeat((w * h) as usize)).unwrap()
}

fn expected_json(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/export")
        .join(name);
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    // Ours, not Aseprite's: app and version. Tag colour is left out in M1.
    v["meta"]["app"] = Value::from(APP);
    v["meta"]["version"] = Value::from(env!("CARGO_PKG_VERSION"));
    if let Some(tags) = v["meta"]["frameTags"].as_array_mut() {
        for t in tags {
            t.as_object_mut().unwrap().remove("color");
        }
    }
    v
}

/// Asserts each key appears, in this order, after the previous one.
fn assert_key_order(json: &str, keys: &[&str]) {
    let mut from = 0;
    for k in keys {
        let needle = format!("\"{k}\"");
        let at = json[from..]
            .find(&needle)
            .unwrap_or_else(|| panic!("key {k} missing or out of order in {json}"));
        from += at + needle.len();
    }
}

/// Three 6x6 frames named like Aseprite names frames of "<title>.aseprite".
fn dur_frames<'a>(images: &'a [RgbaImage], title: &str, ms: [u16; 3]) -> Vec<SheetFrame<'a>> {
    images
        .iter()
        .zip(ms)
        .enumerate()
        .map(|(i, (image, duration_ms))| SheetFrame {
            image,
            name: format!("{title} {i}.aseprite"),
            duration_ms,
        })
        .collect()
}

fn three_images() -> Vec<RgbaImage> {
    vec![
        solid(6, 6, [255, 0, 0, 255]),
        solid(6, 6, [0, 255, 0, 255]),
        solid(6, 6, [0, 0, 255, 255]),
    ]
}

fn rects(sheet: &Sheet) -> Vec<(u32, u32, u32, u32)> {
    sheet.rects.iter().map(|r| (r.x, r.y, r.w, r.h)).collect()
}

// ---- PNG ----

#[test]
fn png_round_trips_rgba8() {
    let mut img = RgbaImage::new(3, 2).unwrap();
    img.put(0, 0, [1, 2, 3, 4]).unwrap();
    img.put(2, 1, [250, 251, 252, 0]).unwrap();
    let bytes = encode_png(&img).unwrap();
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info();
    assert_eq!(
        (info.color_type, info.bit_depth, info.interlaced),
        (png::ColorType::Rgba, png::BitDepth::Eight, false)
    );
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut buf).unwrap();
    assert_eq!((frame.width, frame.height), (3, 2));
    assert_eq!(&buf[..frame.buffer_size()], img.data());
}

// ---- GIF ----

struct Decoded {
    delay: u16,
    dispose: gif::DisposalMethod,
    rect: (u16, u16, u16, u16),
    rgba: Vec<u8>,
}

fn decode_gif(bytes: &[u8]) -> (u16, u16, gif::Repeat, Vec<Decoded>) {
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::RGBA);
    let mut d = opts.read_info(std::io::Cursor::new(bytes)).unwrap();
    let mut frames = Vec::new();
    while let Some(f) = d.read_next_frame().unwrap() {
        frames.push(Decoded {
            delay: f.delay,
            dispose: f.dispose,
            rect: (f.left, f.top, f.width, f.height),
            rgba: f.buffer.to_vec(),
        });
    }
    (d.width(), d.height(), d.repeat(), frames)
}

#[test]
fn gif_frames_stand_alone_with_observed_delays_and_alpha_rule() {
    // Alpha 0 is transparent; any other alpha is opaque with RGB kept (observed).
    let mut a = RgbaImage::new(2, 2).unwrap();
    a.put(0, 0, [10, 20, 30, 255]).unwrap();
    a.put(1, 0, [40, 50, 60, 2]).unwrap();
    a.put(0, 1, [70, 80, 90, 127]).unwrap();
    a.put(1, 1, [99, 99, 99, 0]).unwrap();
    let b = solid(2, 2, [200, 100, 0, 255]);
    let c = RgbaImage::new(2, 2).unwrap();
    let ms = [100u16, 55, 5];
    let frames: Vec<GifFrame> = [&a, &b, &c]
        .iter()
        .zip(ms)
        .map(|(image, duration_ms)| GifFrame { image, duration_ms })
        .collect();
    let bytes = encode_gif(&frames, true).unwrap();
    let (w, h, repeat, out) = decode_gif(&bytes);
    assert_eq!((w, h, repeat), (2, 2, gif::Repeat::Infinite));
    assert_eq!(
        out.iter().map(|f| f.delay).collect::<Vec<_>>(),
        vec![10, 5, 0]
    );
    for f in &out {
        assert_eq!(f.rect, (0, 0, 2, 2), "every frame covers the canvas");
        assert_eq!(
            f.dispose,
            gif::DisposalMethod::Background,
            "each frame stands alone"
        );
    }
    let px = |f: &Decoded, i: usize| -> [u8; 4] {
        let p = &f.rgba[i * 4..i * 4 + 4];
        [p[0], p[1], p[2], p[3]]
    };
    assert_eq!(px(&out[0], 0), [10, 20, 30, 255]);
    assert_eq!(px(&out[0], 1), [40, 50, 60, 255]);
    assert_eq!(px(&out[0], 2), [70, 80, 90, 255]);
    assert_eq!(px(&out[0], 3)[3], 0);
    assert!((0..4).all(|i| px(&out[1], i) == [200, 100, 0, 255]));
    assert!((0..4).all(|i| px(&out[2], i)[3] == 0));

    let once = encode_gif(&frames, false).unwrap();
    assert_eq!(decode_gif(&once).2, gif::Repeat::Finite(0));
}

#[test]
fn gif_with_many_colours_still_encodes() {
    let mut img = RgbaImage::new(32, 32).unwrap();
    for y in 0..32u32 {
        for x in 0..32u32 {
            img.put(
                x,
                y,
                [(x * 8) as u8, (y * 8) as u8, ((x + y) * 4) as u8, 255],
            )
            .unwrap();
        }
    }
    let bytes = encode_gif(
        &[GifFrame {
            image: &img,
            duration_ms: 100,
        }],
        true,
    )
    .unwrap();
    let (w, h, _, out) = decode_gif(&bytes);
    assert_eq!((w, h, out.len()), (32, 32, 1));
}

#[test]
fn gif_rejects_bad_input() {
    assert!(matches!(encode_gif(&[], true), Err(ExportError::Empty)));
    let a = solid(2, 2, [0, 0, 0, 255]);
    let b = solid(3, 2, [0, 0, 0, 255]);
    let mixed = [
        GifFrame {
            image: &a,
            duration_ms: 10,
        },
        GifFrame {
            image: &b,
            duration_ms: 10,
        },
    ];
    assert!(matches!(
        encode_gif(&mixed, true),
        Err(ExportError::MixedSizes)
    ));
    let wide = RgbaImage::new(65536, 1).unwrap();
    assert!(matches!(
        encode_gif(
            &[GifFrame {
                image: &wide,
                duration_ms: 10
            }],
            true
        ),
        Err(ExportError::Encode(_))
    ));
}

// ---- sheet layout ----

#[test]
fn sheet_layouts_match_aseprite_geometry() {
    let imgs = three_images();
    let frames = dur_frames(&imgs, "dur", [100, 55, 33]);

    let h = layout_sheet(&frames, SheetType::Horizontal).unwrap();
    assert_eq!((h.image.width(), h.image.height()), (18, 6));
    assert_eq!(rects(&h), vec![(0, 0, 6, 6), (6, 0, 6, 6), (12, 0, 6, 6)]);
    assert_eq!(h.image.get(13, 5), Some([0, 0, 255, 255]));

    let v = layout_sheet(&frames, SheetType::Vertical).unwrap();
    assert_eq!((v.image.width(), v.image.height()), (6, 18));
    assert_eq!(rects(&v), vec![(0, 0, 6, 6), (0, 6, 6, 6), (0, 12, 6, 6)]);

    let r = layout_sheet(&frames, SheetType::Rows { columns: 2 }).unwrap();
    assert_eq!((r.image.width(), r.image.height()), (12, 12));
    assert_eq!(rects(&r), vec![(0, 0, 6, 6), (6, 0, 6, 6), (0, 6, 6, 6)]);
    assert_eq!(
        r.image.get(9, 9),
        Some([0, 0, 0, 0]),
        "unused cell is transparent"
    );

    let c = layout_sheet(&frames, SheetType::Columns { rows: 2 }).unwrap();
    assert_eq!((c.image.width(), c.image.height()), (12, 12));
    assert_eq!(rects(&c), vec![(0, 0, 6, 6), (0, 6, 6, 6), (6, 0, 6, 6)]);
    assert_eq!(c.image.get(7, 1), Some([0, 0, 255, 255]));

    // Observed: a column/row count above the frame count is clamped.
    let r5 = layout_sheet(&frames, SheetType::Rows { columns: 5 }).unwrap();
    assert_eq!((r5.image.width(), r5.image.height()), (18, 6));
    let c5 = layout_sheet(&frames, SheetType::Columns { rows: 5 }).unwrap();
    assert_eq!((c5.image.width(), c5.image.height()), (6, 18));
}

#[test]
fn sheet_layout_rejects_bad_input() {
    assert!(matches!(
        layout_sheet(&[], SheetType::Horizontal),
        Err(ExportError::Empty)
    ));
    let imgs = three_images();
    let frames = dur_frames(&imgs, "dur", [1, 1, 1]);
    assert!(matches!(
        layout_sheet(&frames, SheetType::Rows { columns: 0 }),
        Err(ExportError::BadLayout(_))
    ));
    assert!(matches!(
        layout_sheet(&frames, SheetType::Columns { rows: 0 }),
        Err(ExportError::BadLayout(_))
    ));
    let odd = solid(5, 6, [0; 4]);
    let mixed = vec![
        frames[0].clone(),
        SheetFrame {
            image: &odd,
            name: "x".into(),
            duration_ms: 1,
        },
    ];
    assert!(matches!(
        layout_sheet(&mixed, SheetType::Horizontal),
        Err(ExportError::MixedSizes)
    ));
}

// ---- sheet JSON against Aseprite's --data output ----

fn no_meta<'a>(image_name: &'a str) -> SheetMeta<'a> {
    SheetMeta {
        image_name,
        tags: &[],
        layers: None,
    }
}

#[test]
fn sheet_json_array_and_hash_match_aseprite() {
    let imgs = three_images();
    let frames = dur_frames(&imgs, "dur", [100, 55, 33]);
    let cases = [
        (
            "sheet_h.json",
            "sheet_h.png",
            SheetType::Horizontal,
            JsonFormat::Array,
        ),
        (
            "sheet_v.json",
            "v.png",
            SheetType::Vertical,
            JsonFormat::Array,
        ),
        (
            "sheet_cols.json",
            "c.png",
            SheetType::Columns { rows: 2 },
            JsonFormat::Array,
        ),
        (
            "sheet_hash.json",
            "sheet_hash.png",
            SheetType::Rows { columns: 2 },
            JsonFormat::Hash,
        ),
    ];
    for (file, image, ty, format) in cases {
        let sheet = layout_sheet(&frames, ty).unwrap();
        let json = sheet_json(&frames, &sheet, format, &no_meta(image)).unwrap();
        let actual: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(actual, expected_json(file), "{file}");
        let frame_keys = [
            "frame",
            "rotated",
            "trimmed",
            "spriteSourceSize",
            "sourceSize",
            "duration",
        ];
        assert_key_order(&json, &frame_keys);
        assert_key_order(
            &json,
            &[
                "frames", "meta", "app", "version", "image", "format", "size", "scale",
            ],
        );
        if matches!(format, JsonFormat::Array) {
            assert_key_order(&json, &["filename", "frame"]);
        } else {
            assert!(!json.contains("\"filename\""));
        }
    }
}

#[test]
fn sheet_json_tags_and_layers_match_aseprite() {
    let imgs = three_images();
    let frames = dur_frames(&imgs, "dur2", [5, 56, 19]);
    let sheet = layout_sheet(&frames, SheetType::Horizontal).unwrap();
    let tags = vec![Tag {
        from: 0,
        to: 1,
        direction: AniDir::Forward,
        repeat: 0,
        legacy_rgb: [0, 0, 0],
        name: "walk".into(),
        user_data: UserData::default(),
    }];
    let mut ink = Sprite::new(1, 1, ColorMode::Rgba);
    ink.layers = vec![Layer {
        id: LayerId(0),
        parent: None,
        name: "ink".into(),
        kind: LayerKind::Image,
        flags: LayerFlags(LayerFlags::VISIBLE | LayerFlags::EDITABLE),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    }];
    let meta = SheetMeta {
        image_name: "t.png",
        tags: &tags,
        layers: Some(&ink.layers),
    };
    let json = sheet_json(&frames, &sheet, JsonFormat::Array, &meta).unwrap();
    let actual: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(actual, expected_json("sheet_tags_layers.json"));
    assert_key_order(
        &json,
        &[
            "scale",
            "frameTags",
            "name",
            "from",
            "to",
            "direction",
            "layers",
        ],
    );
}

#[test]
fn sheet_json_names_every_blend_mode_and_omits_group_blend() {
    let modes = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
        BlendMode::Addition,
        BlendMode::Subtract,
        BlendMode::Divide,
    ];
    let mk = |id: u32, name: String, kind: LayerKind, blend_mode: BlendMode| Layer {
        id: LayerId(id),
        parent: None,
        name,
        kind,
        flags: LayerFlags(LayerFlags::VISIBLE | LayerFlags::EDITABLE),
        blend_mode,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    };
    let mut layers: Vec<Layer> = modes
        .iter()
        .enumerate()
        .map(|(i, m)| mk(i as u32, format!("m{i}"), LayerKind::Image, *m))
        .collect();
    layers.push(mk(19, "grp".into(), LayerKind::Group, BlendMode::Normal));
    let tags = vec![Tag {
        from: 0,
        to: 0,
        direction: AniDir::PingPongReverse,
        repeat: 0,
        legacy_rgb: [0, 0, 0],
        name: "t".into(),
        user_data: UserData::default(),
    }];
    let img = solid(2, 2, [0; 4]);
    let frames = vec![SheetFrame {
        image: &img,
        name: "modes.aseprite".into(),
        duration_ms: 100,
    }];
    let sheet = layout_sheet(&frames, SheetType::Horizontal).unwrap();
    let meta = SheetMeta {
        image_name: "m.png",
        tags: &tags,
        layers: Some(&layers),
    };
    let json = sheet_json(&frames, &sheet, JsonFormat::Array, &meta).unwrap();
    let actual: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(actual, expected_json("sheet_modes.json"));
}

#[test]
fn sheet_json_nested_layers_name_their_group() {
    // Aseprite's listing of layers_tree.aseprite: hidden layers are left out by the
    // caller (no --all-layers), children carry "group", groups carry only a name.
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/layers_tree.aseprite"),
    )
    .unwrap();
    let sprite = read(&bytes).unwrap();
    let visible: Vec<Layer> = sprite
        .layers
        .iter()
        .filter(|l| l.flags.contains(LayerFlags::VISIBLE))
        .cloned()
        .collect();
    let img = solid(u32::from(sprite.width), u32::from(sprite.height), [0; 4]);
    let frames = vec![SheetFrame {
        image: &img,
        name: "layers_tree.aseprite".into(),
        duration_ms: sprite.frames[0].duration_ms,
    }];
    let sheet = layout_sheet(&frames, SheetType::Horizontal).unwrap();
    let meta = SheetMeta {
        image_name: "lt.png",
        tags: &sprite.tags,
        layers: Some(&visible),
    };
    let json = sheet_json(&frames, &sheet, JsonFormat::Array, &meta).unwrap();
    let actual: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(actual, expected_json("sheet_layers_tree.json"));
    assert_key_order(&json, &["arm", "group", "opacity", "blendMode"]);
}
