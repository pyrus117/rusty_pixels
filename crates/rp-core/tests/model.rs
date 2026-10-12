#![allow(clippy::unwrap_used, clippy::expect_used)]
use rp_core::*;

fn layer(id: u32, parent: Option<u32>, kind: LayerKind) -> Layer {
    Layer {
        id: LayerId(id),
        parent: parent.map(LayerId),
        name: format!("L{id}"),
        kind,
        flags: LayerFlags(LayerFlags::VISIBLE | LayerFlags::EDITABLE),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    }
}

/// G0 { I1, G2 { I3 } }, I4
fn tree() -> Sprite {
    let mut s = Sprite::new(8, 8, ColorMode::Rgba);
    s.layers = vec![
        layer(0, None, LayerKind::Group),
        layer(1, Some(0), LayerKind::Image),
        layer(2, Some(0), LayerKind::Group),
        layer(3, Some(2), LayerKind::Image),
        layer(4, None, LayerKind::Image),
    ];
    s
}

#[test]
fn color_mode_depths_round_trip() {
    for m in [ColorMode::Rgba, ColorMode::Grayscale, ColorMode::Indexed] {
        assert_eq!(ColorMode::from_depth_bits(m.depth_bits()), Some(m));
    }
    assert_eq!(ColorMode::Rgba.bytes_per_pixel(), 4);
    assert_eq!(ColorMode::Grayscale.bytes_per_pixel(), 2);
    assert_eq!(ColorMode::Indexed.bytes_per_pixel(), 1);
    assert_eq!(ColorMode::from_depth_bits(24), None);
}

#[test]
fn pixel_bytes_are_in_file_order() {
    assert_eq!(
        PixelValue::Rgba {
            r: 1,
            g: 2,
            b: 3,
            a: 4
        }
        .to_bytes(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(PixelValue::Grayscale { v: 9, a: 8 }.to_bytes(), vec![9, 8]);
    assert_eq!(PixelValue::Indexed { index: 7 }.to_bytes(), vec![7]);
}

#[test]
fn image_put_get_and_errors() {
    let mut img = Image::new(3, 2, ColorMode::Rgba);
    assert_eq!(img.data().len(), 24);
    img.put(
        2,
        1,
        PixelValue::Rgba {
            r: 10,
            g: 20,
            b: 30,
            a: 40,
        },
    )
    .unwrap();
    assert_eq!(img.get(2, 1), Some(&[10u8, 20, 30, 40][..]));
    assert_eq!(&img.data()[20..24], &[10, 20, 30, 40]);
    assert_eq!(img.get(3, 0), None);
    assert!(matches!(
        img.put(0, 0, PixelValue::Indexed { index: 1 }),
        Err(CoreError::ModeMismatch {
            image: ColorMode::Rgba,
            pixel: ColorMode::Indexed
        })
    ));
    assert!(matches!(
        img.put(
            0,
            2,
            PixelValue::Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 0
            }
        ),
        Err(CoreError::OutOfBounds { .. })
    ));
}

#[test]
fn image_from_raw_checks_length_without_overflow() {
    assert!(Image::from_raw(2, 2, ColorMode::Grayscale, vec![0; 8]).is_ok());
    assert_eq!(
        Image::from_raw(2, 2, ColorMode::Grayscale, vec![0; 7]),
        Err(CoreError::DataLength {
            expected: 8,
            actual: 7
        })
    );
    assert!(matches!(
        Image::from_raw(u32::MAX, u32::MAX, ColorMode::Rgba, vec![]),
        Err(CoreError::DataLength { .. })
    ));
}

#[test]
fn sprite_new_defaults() {
    let s = Sprite::new(16, 9, ColorMode::Indexed);
    assert_eq!(
        (s.width, s.height, s.color_mode),
        (16, 9, ColorMode::Indexed)
    );
    assert_eq!(s.frames, vec![Frame { duration_ms: 100 }]);
    assert_eq!(s.flags, HeaderFlags(HeaderFlags::LAYER_OPACITY_VALID));
    assert_eq!(
        s.grid,
        Grid {
            x: 0,
            y: 0,
            width: 16,
            height: 16
        }
    );
    assert_eq!((s.pixel_width, s.pixel_height), (1, 1));
    assert!(s.layers.is_empty() && s.cels.is_empty() && s.palette.entries.is_empty());
    assert_eq!(
        s.color_profile,
        Some(ColorProfile {
            kind: 1,
            flags: 0,
            gamma: 0,
            icc: vec![]
        })
    );
}

#[test]
fn subtree_ranges_and_depths() {
    let s = tree();
    assert_eq!(s.subtree_range(LayerId(0)), Some(0..4));
    assert_eq!(s.subtree_range(LayerId(2)), Some(2..4));
    assert_eq!(s.subtree_range(LayerId(3)), Some(3..4));
    assert_eq!(s.subtree_range(LayerId(4)), Some(4..5));
    assert_eq!(s.subtree_range(LayerId(99)), None);
    assert_eq!(s.depth(LayerId(0)), Some(0));
    assert_eq!(s.depth(LayerId(3)), Some(2));
    assert_eq!(s.layer_index(LayerId(4)), Some(4));
}

#[test]
fn resolve_cel_follows_one_link() {
    let mut s = tree();
    let img = Image::new(1, 1, ColorMode::Rgba);
    let base = Cel {
        x: 0,
        y: 0,
        opacity: 255,
        z_index: 0,
        content: CelContent::Image(img),
        extra: None,
        user_data: UserData::default(),
    };
    let mut linked = base.clone();
    linked.content = CelContent::Linked { frame: 0 };
    s.frames.push(Frame { duration_ms: 100 });
    s.cels.insert((LayerId(1), 0), base);
    s.cels.insert((LayerId(1), 1), linked);
    let (frame, cel) = s.resolve_cel(LayerId(1), 1).unwrap();
    assert_eq!(frame, 0);
    assert!(matches!(cel.content, CelContent::Image(_)));
    assert!(s.resolve_cel(LayerId(4), 0).is_none());
}

#[test]
fn user_data_emptiness_and_prop_type_codes() {
    assert!(UserData::default().is_empty());
    let ud = UserData {
        text: Some(String::new()),
        ..UserData::default()
    };
    assert!(!ud.is_empty());
    assert_eq!(PropValue::Bool(true).type_code(), 0x01);
    assert_eq!(PropValue::Fixed(0).type_code(), 0x0A);
    assert_eq!(
        PropValue::Vector {
            element_type: 0,
            items: vec![]
        }
        .type_code(),
        0x11
    );
    assert_eq!(PropValue::Map(vec![]).type_code(), 0x12);
    assert_eq!(PropValue::Uuid([0; 16]).type_code(), 0x13);
}

#[test]
fn enums_map_to_file_values() {
    assert_eq!(BlendMode::from_u16(18), Some(BlendMode::Divide));
    assert_eq!(BlendMode::Screen.as_u16(), 2);
    assert_eq!(BlendMode::from_u16(19), None);
    assert_eq!(AniDir::from_u8(3), Some(AniDir::PingPongReverse));
    assert_eq!(AniDir::from_u8(4), None);
}

#[test]
fn command_facing_types_serialise_as_documented() {
    let v = serde_json::to_value(PixelValue::Rgba {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    })
    .unwrap();
    assert_eq!(
        v,
        serde_json::json!({"mode": "rgba", "r": 1, "g": 2, "b": 3, "a": 4})
    );
    assert_eq!(
        serde_json::to_value(LayerId(5)).unwrap(),
        serde_json::json!(5)
    );
}
