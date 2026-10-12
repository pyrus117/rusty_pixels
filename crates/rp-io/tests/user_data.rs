#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

fn read1(chunks: &[Vec<u8>]) -> Result<Sprite, AseError> {
    rp_io::read(&file(&Header::default(), &[frame(100, chunks)]))
}

fn pal() -> Vec<u8> {
    chunk(
        0x2019,
        &B::new()
            .u32(1)
            .u32(0)
            .u32(0)
            .zeros(8)
            .u16(0)
            .bytes(&[1, 2, 3, 255])
            .done(),
    )
}

#[test]
fn attaches_to_sprite_layer_cel_slice() {
    let cel = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 0, 0)
            .u16(1)
            .u16(1)
            .bytes(&[0; 4])
            .done(),
    );
    let extra = chunk(0x2006, &B::new().u32(0).zeros(16).zeros(16).done());
    let slice = chunk(
        0x2022,
        &B::new()
            .u32(1)
            .u32(0)
            .u32(0)
            .string("s")
            .u32(0)
            .i32(0)
            .i32(0)
            .u32(1)
            .u32(1)
            .done(),
    );
    let s = read1(&[
        pal(),
        ud_chunk(Some("sprite"), None),
        layer_chunk(1, 0, 0, 0, 255, "a"),
        ud_chunk(Some("layer"), Some([1, 2, 3, 4])),
        cel,
        extra,
        ud_chunk(Some("cel"), None),
        slice,
        ud_chunk(None, Some([9, 9, 9, 9])),
    ])
    .unwrap();
    assert_eq!(s.user_data.text.as_deref(), Some("sprite"));
    assert_eq!(s.layers[0].user_data.text.as_deref(), Some("layer"));
    assert_eq!(s.layers[0].user_data.color, Some([1, 2, 3, 4]));
    assert_eq!(
        s.cel(LayerId(0), 0).unwrap().user_data.text.as_deref(),
        Some("cel")
    );
    assert_eq!(s.slices[0].user_data.color, Some([9, 9, 9, 9]));
}

#[test]
fn tags_take_user_data_in_order() {
    let tags = chunk(
        0x2018,
        &B::new()
            .u16(2)
            .zeros(8)
            .u16(0)
            .u16(0)
            .u8(0)
            .u16(0)
            .zeros(6)
            .bytes(&[0; 3])
            .u8(0)
            .string("a")
            .u16(0)
            .u16(0)
            .u8(0)
            .u16(0)
            .zeros(6)
            .bytes(&[0; 3])
            .u8(0)
            .string("b")
            .done(),
    );
    let s = read1(&[
        tags.clone(),
        ud_chunk(None, Some([1, 0, 0, 255])),
        ud_chunk(None, Some([0, 2, 0, 255])),
    ])
    .unwrap();
    assert_eq!(s.tags[0].user_data.color, Some([1, 0, 0, 255]));
    assert_eq!(s.tags[1].user_data.color, Some([0, 2, 0, 255]));
    let r = read1(&[
        tags,
        ud_chunk(None, None),
        ud_chunk(None, None),
        ud_chunk(None, None),
    ]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "orphan user data",
            ..
        })
    ));
}

// Revised 2026-10-12 (R9): the start of frame 0 belongs to the sprite, so the ownerless case
// is the start of a later frame. That chunk is kept raw rather than rejected.
#[test]
fn ownerless_user_data_in_a_later_frame_is_kept_raw() {
    let h = Header {
        frames: 2,
        ..Header::default()
    };
    let s = rp_io::read(&file(
        &h,
        &[frame(100, &[]), frame(100, &[ud_chunk(Some("x"), None)])],
    ))
    .unwrap();
    assert_eq!(s.user_data, UserData::default());
    let raw: Vec<(u32, u16)> = s.raw_chunks.iter().map(|c| (c.frame, c.kind)).collect();
    assert_eq!(raw, vec![(1, 0x2020)]);
    assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
}

/// One properties map (key 0) with one property, raw-encoded by hand.
fn props_ud(name: &str, type_code: u16, value: &[u8]) -> Vec<u8> {
    let prop = B::new().string(name).u16(type_code).bytes(value).done();
    let maps = B::new().u32(0).u32(1).bytes(&prop).done();
    let size = 8 + maps.len() as u32;
    chunk(
        0x2020,
        &B::new().u32(4).u32(size).u32(1).bytes(&maps).done(),
    )
}

#[test]
fn every_scalar_property_type_decodes() {
    let cases: Vec<(u16, Vec<u8>, PropValue)> = vec![
        (0x01, vec![2], PropValue::Bool(true)),
        (0x02, vec![0xFF], PropValue::I8(-1)),
        (0x03, vec![200], PropValue::U8(200)),
        (0x04, B::new().i16(-300).done(), PropValue::I16(-300)),
        (0x05, B::new().u16(60000).done(), PropValue::U16(60000)),
        (0x06, B::new().i32(-70000).done(), PropValue::I32(-70000)),
        (
            0x07,
            B::new().u32(4_000_000_000).done(),
            PropValue::U32(4_000_000_000),
        ),
        (0x08, B::new().i64(-5).done(), PropValue::I64(-5)),
        (
            0x09,
            B::new().u64(u64::MAX).done(),
            PropValue::U64(u64::MAX),
        ),
        (
            0x0A,
            B::new().i32(0x18000).done(),
            PropValue::Fixed(0x18000),
        ),
        (0x0B, B::new().f32(1.5).done(), PropValue::F32(1.5)),
        (0x0C, B::new().f64(-2.25).done(), PropValue::F64(-2.25)),
        (
            0x0D,
            B::new().string("hi").done(),
            PropValue::String("hi".into()),
        ),
        (
            0x0E,
            B::new().i32(1).i32(-2).done(),
            PropValue::Point { x: 1, y: -2 },
        ),
        (
            0x0F,
            B::new().i32(3).i32(4).done(),
            PropValue::Size {
                width: 3,
                height: 4,
            },
        ),
        (
            0x10,
            B::new().i32(1).i32(2).i32(3).i32(4).done(),
            PropValue::Rect {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            },
        ),
        (0x13, vec![5; 16], PropValue::Uuid([5; 16])),
    ];
    for (code, raw, expected) in cases {
        let s = read1(&[pal(), props_ud("p", code, &raw)]).unwrap();
        assert_eq!(
            s.user_data.properties,
            vec![PropertiesMap {
                key: 0,
                properties: vec![("p".into(), expected)]
            }],
            "type {code:#x}"
        );
    }
}

#[test]
fn vectors_and_nested_maps_decode() {
    let uniform = B::new().u32(2).u16(0x03).u8(1).u8(2).done();
    let s = read1(&[pal(), props_ud("v", 0x11, &uniform)]).unwrap();
    assert_eq!(
        s.user_data.properties[0].properties[0].1,
        PropValue::Vector {
            element_type: 0x03,
            items: vec![PropValue::U8(1), PropValue::U8(2)]
        }
    );

    let mixed = B::new()
        .u32(2)
        .u16(0)
        .u16(0x01)
        .u8(0)
        .u16(0x0D)
        .string("z")
        .done();
    let s = read1(&[pal(), props_ud("m", 0x11, &mixed)]).unwrap();
    assert_eq!(
        s.user_data.properties[0].properties[0].1,
        PropValue::Vector {
            element_type: 0,
            items: vec![PropValue::Bool(false), PropValue::String("z".into())]
        }
    );

    let nested = B::new().u32(1).string("inner").u16(0x06).i32(42).done();
    let s = read1(&[pal(), props_ud("n", 0x12, &nested)]).unwrap();
    assert_eq!(
        s.user_data.properties[0].properties[0].1,
        PropValue::Map(vec![("inner".into(), PropValue::I32(42))])
    );
}

#[test]
fn bad_properties_are_errors() {
    let r = read1(&[pal(), props_ud("x", 0x55, &[])]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "property type",
            ..
        })
    ));

    let prop = B::new().string("p").u16(0x01).u8(1).done();
    let maps = B::new().u32(0).u32(1).bytes(&prop).done();
    let wrong = chunk(0x2020, &B::new().u32(4).u32(999).u32(1).bytes(&maps).done());
    assert!(matches!(
        read1(&[pal(), wrong]),
        Err(AseError::Invalid {
            what: "properties size",
            ..
        })
    ));

    // 129 nested maps.
    let mut inner = B::new().u32(0).done();
    for _ in 0..129 {
        inner = B::new().u32(1).string("d").u16(0x12).bytes(&inner).done();
    }
    let r = read1(&[pal(), props_ud("deep", 0x12, &inner)]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "properties depth",
            ..
        })
    ));
}

fn every_prop() -> Vec<(String, PropValue)> {
    vec![
        ("b".into(), PropValue::Bool(true)),
        ("i8".into(), PropValue::I8(-8)),
        ("u8".into(), PropValue::U8(8)),
        ("i16".into(), PropValue::I16(-16)),
        ("u16".into(), PropValue::U16(16)),
        ("i32".into(), PropValue::I32(-32)),
        ("u32".into(), PropValue::U32(32)),
        ("i64".into(), PropValue::I64(-64)),
        ("u64".into(), PropValue::U64(64)),
        ("fx".into(), PropValue::Fixed(0x10000)),
        ("f".into(), PropValue::F32(0.5)),
        ("d".into(), PropValue::F64(0.25)),
        ("s".into(), PropValue::String("text".into())),
        ("pt".into(), PropValue::Point { x: 1, y: 2 }),
        (
            "sz".into(),
            PropValue::Size {
                width: 3,
                height: 4,
            },
        ),
        (
            "r".into(),
            PropValue::Rect {
                x: 5,
                y: 6,
                width: 7,
                height: 8,
            },
        ),
        (
            "vu".into(),
            PropValue::Vector {
                element_type: 0x06,
                items: vec![PropValue::I32(1), PropValue::I32(2)],
            },
        ),
        (
            "vm".into(),
            PropValue::Vector {
                element_type: 0,
                items: vec![PropValue::U8(1), PropValue::String("x".into())],
            },
        ),
        (
            "map".into(),
            PropValue::Map(vec![(
                "k".into(),
                PropValue::Map(vec![("deeper".into(), PropValue::Bool(false))]),
            )]),
        ),
        ("id".into(), PropValue::Uuid([3; 16])),
    ]
}

#[test]
fn user_data_everywhere_round_trips() {
    let ud = |t: &str| UserData {
        text: Some(t.into()),
        color: Some([1, 2, 3, 4]),
        properties: vec![
            PropertiesMap {
                key: 0,
                properties: every_prop(),
            },
            PropertiesMap {
                key: 1,
                properties: vec![("ext".into(), PropValue::U8(1))],
            },
        ],
    };
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.external_files = vec![ExternalFile {
        id: 1,
        kind: 2,
        name: "pub/ext".into(),
    }];
    s.palette.entries = vec![PaletteEntry {
        rgba: [0, 0, 0, 255],
        name: None,
    }];
    s.user_data = ud("sprite");
    s.layers = vec![Layer {
        id: LayerId(0),
        parent: None,
        name: "l".into(),
        kind: LayerKind::Image,
        flags: LayerFlags(3),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: ud("layer"),
    }];
    s.cels.insert(
        (LayerId(0), 0),
        Cel {
            x: 0,
            y: 0,
            opacity: 255,
            z_index: 0,
            content: CelContent::Image(Image::new(1, 1, ColorMode::Rgba).unwrap()),
            extra: None,
            user_data: ud("cel"),
        },
    );
    s.tags = vec![
        Tag {
            from: 0,
            to: 0,
            direction: AniDir::Forward,
            repeat: 0,
            legacy_rgb: [0; 3],
            name: "t0".into(),
            user_data: UserData::default(),
        },
        Tag {
            from: 0,
            to: 0,
            direction: AniDir::Forward,
            repeat: 0,
            legacy_rgb: [0; 3],
            name: "t1".into(),
            user_data: ud("tag"),
        },
    ];
    s.slices = vec![Slice {
        name: "s".into(),
        flags: 0,
        user_data: ud("slice"),
        keys: vec![SliceKey {
            frame: 0,
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            center: None,
            pivot: None,
        }],
    }];
    assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
}

// Revised 2026-10-12 from black-box observation: Aseprite gives user data at the start of
// frame 0 to the sprite, so no palette anchor is needed.
#[test]
fn sprite_user_data_without_palette_round_trips() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.user_data = UserData {
        text: Some("hello".into()),
        ..UserData::default()
    };
    let b = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&b).unwrap(), s);
    assert_eq!(kinds(&b, 0).first(), Some(&0x2020));
}

#[test]
fn writer_rejects_mistyped_uniform_vector() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.palette.entries = vec![PaletteEntry {
        rgba: [0, 0, 0, 255],
        name: None,
    }];
    s.user_data.properties = vec![PropertiesMap {
        key: 0,
        properties: vec![(
            "v".into(),
            PropValue::Vector {
                element_type: 0x03,
                items: vec![PropValue::Bool(true)],
            },
        )],
    }];
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "property vector",
            ..
        })
    ));
}
