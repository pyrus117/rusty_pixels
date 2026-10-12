#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Hand-built cases the oracle cannot express: filters, errors, hostile input.
//! Images are built with Image::from_raw so these tests survive Image::new turning fallible.
use rp_core::*;
use rp_render::*;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

fn layer(id: u32, parent: Option<u32>, kind: LayerKind, visible: bool) -> Layer {
    let mut flags = LayerFlags::EDITABLE;
    if visible {
        flags |= LayerFlags::VISIBLE;
    }
    Layer {
        id: LayerId(id),
        parent: parent.map(LayerId),
        name: format!("l{id}"),
        kind,
        flags: LayerFlags(flags),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    }
}

fn rgba_cel(x: i32, y: i32, w: u32, h: u32, px: &[[u8; 4]]) -> Cel {
    let data: Vec<u8> = px.iter().flatten().copied().collect();
    Cel {
        x,
        y,
        opacity: 255,
        z_index: 0,
        content: CelContent::Image(Image::from_raw(w, h, ColorMode::Rgba, data).unwrap()),
        extra: None,
        user_data: UserData::default(),
    }
}

fn opts(layers: LayerFilter) -> RenderOptions {
    RenderOptions { layers }
}

fn pixels(img: &RgbaImage) -> Vec<[u8; 4]> {
    img.data()
        .chunks_exact(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect()
}

/// Red layer 0 visible at (0,0); blue layer 1 hidden at (1,0); 2x1 canvas.
fn two_layers() -> Sprite {
    let mut s = Sprite::new(2, 1, ColorMode::Rgba);
    s.layers = vec![
        layer(0, None, LayerKind::Image, true),
        layer(1, None, LayerKind::Image, false),
    ];
    s.cels.insert((LayerId(0), 0), rgba_cel(0, 0, 1, 1, &[RED]));
    s.cels
        .insert((LayerId(1), 0), rgba_cel(1, 0, 1, 1, &[BLUE]));
    s
}

#[test]
fn default_options_are_visible_layers() {
    let d = RenderOptions::default();
    assert!(matches!(d.layers, LayerFilter::Visible));
}

#[test]
fn layer_filters() {
    let s = two_layers();
    let vis = render_frame(&s, 0, &opts(LayerFilter::Visible)).unwrap();
    assert_eq!(pixels(&vis), vec![RED, CLEAR]);
    let all = render_frame(&s, 0, &opts(LayerFilter::All)).unwrap();
    assert_eq!(pixels(&all), vec![RED, BLUE]);
    let only = render_frame(&s, 0, &opts(LayerFilter::Only(vec![LayerId(1)]))).unwrap();
    assert_eq!(pixels(&only), vec![CLEAR, BLUE], "Only ignores visibility");
    assert!(matches!(
        render_frame(&s, 0, &opts(LayerFilter::Only(vec![LayerId(99)]))),
        Err(RenderError::UnknownLayer(LayerId(99)))
    ));
}

#[test]
fn hidden_group_hides_children_unless_selected() {
    let mut s = Sprite::new(1, 1, ColorMode::Rgba);
    s.layers = vec![
        layer(0, None, LayerKind::Group, false),
        layer(1, Some(0), LayerKind::Image, true),
    ];
    s.cels.insert((LayerId(1), 0), rgba_cel(0, 0, 1, 1, &[RED]));
    let vis = render_frame(&s, 0, &RenderOptions::default()).unwrap();
    assert_eq!(pixels(&vis), vec![CLEAR]);
    let only = render_frame(&s, 0, &opts(LayerFilter::Only(vec![LayerId(0)]))).unwrap();
    assert_eq!(
        pixels(&only),
        vec![RED],
        "selecting a group selects its subtree"
    );
}

#[test]
fn unknown_and_empty_frames() {
    let s = two_layers();
    assert!(matches!(
        render_frame(&s, 1, &RenderOptions::default()),
        Err(RenderError::UnknownFrame(1))
    ));
    let mut s = Sprite::new(3, 2, ColorMode::Rgba);
    s.layers = vec![layer(0, None, LayerKind::Image, true)];
    let img = render_frame(&s, 0, &RenderOptions::default()).unwrap();
    assert_eq!((img.width(), img.height()), (3, 2));
    assert!(img.data().iter().all(|&b| b == 0));
}

#[test]
fn cels_are_clipped_to_the_canvas() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.layers = vec![layer(0, None, LayerKind::Image, true)];
    let px: Vec<[u8; 4]> = (0..9u8).map(|i| [i * 10, 0, 0, 255]).collect();
    s.cels.insert((LayerId(0), 0), rgba_cel(-1, -1, 3, 3, &px));
    let img = render_frame(&s, 0, &RenderOptions::default()).unwrap();
    // The canvas sees the cel's bottom-right 2x2: indices 4, 5, 7, 8.
    assert_eq!(
        pixels(&img),
        vec![
            [40, 0, 0, 255],
            [50, 0, 0, 255],
            [70, 0, 0, 255],
            [80, 0, 0, 255]
        ]
    );
    let mut far = s.clone();
    far.cels
        .insert((LayerId(0), 0), rgba_cel(i32::MAX - 1, i32::MIN, 3, 3, &px));
    let img = render_frame(&far, 0, &RenderOptions::default()).unwrap();
    assert!(
        img.data().iter().all(|&b| b == 0),
        "far-away cel draws nothing"
    );
}

#[test]
fn layer_opacity_needs_the_header_flag() {
    let mut s = Sprite::new(1, 1, ColorMode::Rgba);
    let mut l = layer(0, None, LayerKind::Image, true);
    l.opacity = 0;
    s.layers = vec![l];
    s.cels.insert((LayerId(0), 0), rgba_cel(0, 0, 1, 1, &[RED]));
    s.flags = HeaderFlags(HeaderFlags::LAYER_OPACITY_VALID);
    let img = render_frame(&s, 0, &RenderOptions::default()).unwrap();
    assert_eq!(img.get(0, 0).unwrap()[3], 0, "opacity 0 hides the layer");
    s.flags = HeaderFlags(0);
    let img = render_frame(&s, 0, &RenderOptions::default()).unwrap();
    assert_eq!(
        img.get(0, 0),
        Some(RED),
        "without the flag, layer opacity is 255"
    );
}

#[test]
fn linked_cel_renders_its_target() {
    let mut s = two_layers();
    s.frames.push(Frame { duration_ms: 100 });
    s.cels.insert(
        (LayerId(0), 1),
        Cel {
            content: CelContent::Linked { frame: 0 },
            ..rgba_cel(0, 0, 1, 1, &[BLUE])
        },
    );
    let f0 = render_frame(&s, 0, &RenderOptions::default()).unwrap();
    let f1 = render_frame(&s, 1, &RenderOptions::default()).unwrap();
    assert_eq!(f0, f1);
}

#[test]
fn indexed_and_grayscale_lookup() {
    let mut s = Sprite::new(3, 1, ColorMode::Indexed);
    s.transparent_index = 0;
    s.palette.entries = vec![
        PaletteEntry {
            rgba: [9, 9, 9, 255],
            name: None,
        },
        PaletteEntry {
            rgba: [1, 2, 3, 255],
            name: None,
        },
    ];
    s.layers = vec![layer(0, None, LayerKind::Image, true)];
    s.cels.insert(
        (LayerId(0), 0),
        Cel {
            content: CelContent::Image(
                Image::from_raw(3, 1, ColorMode::Indexed, vec![0, 1, 200]).unwrap(),
            ),
            ..rgba_cel(0, 0, 1, 1, &[RED])
        },
    );
    let img = render_frame(&s, 0, &RenderOptions::default()).unwrap();
    assert_eq!(img.get(0, 0).unwrap()[3], 0, "transparent index");
    assert_eq!(img.get(1, 0), Some([1, 2, 3, 255]));
    assert_eq!(img.get(2, 0).unwrap()[3], 0, "index past the palette");

    let mut g = Sprite::new(1, 1, ColorMode::Grayscale);
    g.layers = vec![layer(0, None, LayerKind::Image, true)];
    g.cels.insert(
        (LayerId(0), 0),
        Cel {
            content: CelContent::Image(
                Image::from_raw(1, 1, ColorMode::Grayscale, vec![77, 255]).unwrap(),
            ),
            ..rgba_cel(0, 0, 1, 1, &[RED])
        },
    );
    let img = render_frame(&g, 0, &RenderOptions::default()).unwrap();
    assert_eq!(img.get(0, 0), Some([77, 77, 77, 255]));
}

#[test]
fn hostile_sprites_error_instead_of_panicking() {
    // A cel image in the wrong colour mode for the sprite.
    let mut s = Sprite::new(1, 1, ColorMode::Rgba);
    s.layers = vec![layer(0, None, LayerKind::Image, true)];
    s.cels.insert(
        (LayerId(0), 0),
        Cel {
            content: CelContent::Image(Image::from_raw(1, 1, ColorMode::Indexed, vec![1]).unwrap()),
            ..rgba_cel(0, 0, 1, 1, &[RED])
        },
    );
    assert!(matches!(
        render_frame(&s, 0, &RenderOptions::default()),
        Err(RenderError::BadCel { .. })
    ));

    // A parent cycle must terminate.
    let mut c = Sprite::new(1, 1, ColorMode::Rgba);
    c.layers = vec![
        layer(0, Some(1), LayerKind::Group, true),
        layer(1, Some(0), LayerKind::Group, true),
    ];
    let _ = render_frame(&c, 0, &RenderOptions::default());

    // A tilemap layer whose tileset does not exist.
    let mut t = Sprite::new(4, 4, ColorMode::Rgba);
    t.layers = vec![layer(
        0,
        None,
        LayerKind::Tilemap { tileset_index: 7 },
        true,
    )];
    t.cels.insert(
        (LayerId(0), 0),
        Cel {
            content: CelContent::Tilemap(Tilemap {
                width: 1,
                height: 1,
                bits_per_tile: 32,
                tile_id_mask: 0x1fff_ffff,
                x_flip_mask: 0x2000_0000,
                y_flip_mask: 0x4000_0000,
                diagonal_flip_mask: 0x8000_0000,
                tiles: vec![1],
            }),
            ..rgba_cel(0, 0, 1, 1, &[RED])
        },
    );
    assert!(matches!(
        render_frame(&t, 0, &RenderOptions::default()),
        Err(RenderError::BadCel { .. })
    ));

    // A Linked cel pointing at a frame with no cel draws nothing rather than failing hard.
    let mut l = Sprite::new(1, 1, ColorMode::Rgba);
    l.layers = vec![layer(0, None, LayerKind::Image, true)];
    l.cels.insert(
        (LayerId(0), 0),
        Cel {
            content: CelContent::Linked { frame: 5 },
            ..rgba_cel(0, 0, 1, 1, &[RED])
        },
    );
    let _ = render_frame(&l, 0, &RenderOptions::default());
}

#[test]
fn scale_nearest_blocks_and_bounds() {
    let src = RgbaImage::from_raw(2, 1, [RED, BLUE].concat()).unwrap();
    let big = scale_nearest(&src, 3).unwrap();
    assert_eq!((big.width(), big.height()), (6, 3));
    for y in 0..3 {
        for x in 0..6 {
            assert_eq!(big.get(x, y), Some(if x < 3 { RED } else { BLUE }));
        }
    }
    assert_eq!(scale_nearest(&src, 1).unwrap(), src);
    assert!(matches!(
        scale_nearest(&src, 0),
        Err(RenderError::BadFactor(0))
    ));
    assert!(matches!(
        scale_nearest(&src, 65),
        Err(RenderError::BadFactor(65))
    ));
    // 4096² ×64 is far past MAX_RGBA_BYTES.
    let large = RgbaImage::new(4096, 4096).unwrap();
    assert!(matches!(
        scale_nearest(&large, 64),
        Err(RenderError::Image(_))
    ));
}
