#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

fn read1(h: &Header, chunks: &[Vec<u8>]) -> Result<Sprite, AseError> {
    rp_io::read(&file(h, &[frame(100, chunks)]))
}

#[test]
fn layer_tree_from_child_levels() {
    // NOTE.1 example from the spec.
    let chunks = vec![
        layer_chunk(1, 1, 0, 0, 255, "Background"),
        layer_chunk(1, 0, 1, 0, 255, "Layer1"),
        layer_chunk(1, 1, 0, 0, 255, "Foreground"),
        layer_chunk(1, 1, 1, 0, 255, "My set1"),
        layer_chunk(1, 0, 2, 0, 255, "Layer2"),
        layer_chunk(1, 0, 1, 0, 255, "Layer3"),
    ];
    let s = read1(&Header::default(), &chunks).unwrap();
    let parents: Vec<Option<u32>> = s.layers.iter().map(|l| l.parent.map(|p| p.0)).collect();
    assert_eq!(
        parents,
        vec![None, Some(0), None, Some(2), Some(3), Some(2)]
    );
    let ids: Vec<u32> = s.layers.iter().map(|l| l.id.0).collect();
    assert_eq!(ids, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(s.layers[3].name, "My set1");
    assert_eq!(s.layers[0].kind, LayerKind::Group);
}

#[test]
fn layer_fields_and_uuid() {
    let tile = chunk(
        0x2004,
        &B::new()
            .u16(3)
            .u16(2)
            .u16(0)
            .u16(0)
            .u16(0)
            .u16(16)
            .u8(128)
            .zeros(3)
            .string("Tiles")
            .u32(4)
            .bytes(&[7; 16])
            .done(),
    );
    let s = read1(
        &Header {
            flags: 1 | 4,
            ..Header::default()
        },
        &[tile],
    )
    .unwrap();
    let l = &s.layers[0];
    assert_eq!(l.kind, LayerKind::Tilemap { tileset_index: 4 });
    assert_eq!(l.flags, LayerFlags(3));
    assert_eq!(l.blend_mode, BlendMode::Addition);
    assert_eq!(l.opacity, 128);
    assert_eq!(l.uuid, Some([7; 16]));
}

#[test]
fn bad_layers_are_errors() {
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 99, 255, "x")]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "blend mode",
            ..
        })
    ));
    let r = read1(&Header::default(), &[layer_chunk(1, 9, 0, 0, 255, "x")]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "layer type",
            ..
        })
    ));
    // Child level jumps from 0 to 2.
    let r = read1(
        &Header::default(),
        &[
            layer_chunk(1, 1, 0, 0, 255, "g"),
            layer_chunk(1, 0, 2, 0, 255, "x"),
        ],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "layer tree",
            ..
        })
    ));
    // Parent is an image layer, not a group.
    let r = read1(
        &Header::default(),
        &[
            layer_chunk(1, 0, 0, 0, 255, "i"),
            layer_chunk(1, 0, 1, 0, 255, "x"),
        ],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "layer tree",
            ..
        })
    ));
}

#[test]
fn raw_and_zlib_cels() {
    let raw = chunk(
        0x2005,
        &cel_head(0, 1, 2, 200, 0, -1)
            .u16(2)
            .u16(1)
            .bytes(&[1, 2, 3, 4, 5, 6, 7, 8])
            .done(),
    );
    let s = read1(
        &Header::default(),
        &[layer_chunk(1, 0, 0, 0, 255, "a"), raw],
    )
    .unwrap();
    let cel = s.cel(LayerId(0), 0).unwrap();
    assert_eq!((cel.x, cel.y, cel.opacity, cel.z_index), (1, 2, 200, -1));
    match &cel.content {
        CelContent::Image(img) => {
            assert_eq!(
                (img.width(), img.height(), img.mode()),
                (2, 1, ColorMode::Rgba)
            );
            assert_eq!(img.data(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        }
        other => panic!("expected image, got {other:?}"),
    }

    let gray = zlib(&[10, 255, 20, 128]);
    let z = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 2, 0)
            .u16(1)
            .u16(2)
            .bytes(&gray)
            .done(),
    );
    let s = read1(
        &Header {
            depth: 16,
            ..Header::default()
        },
        &[layer_chunk(1, 0, 0, 0, 255, "a"), z],
    )
    .unwrap();
    match &s.cel(LayerId(0), 0).unwrap().content {
        CelContent::Image(img) => assert_eq!(img.data(), &[10, 255, 20, 128]),
        other => panic!("expected image, got {other:?}"),
    }
}

#[test]
fn zlib_cel_with_wrong_size_is_an_error() {
    let short = zlib(&[1, 2, 3]);
    let z = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 2, 0)
            .u16(1)
            .u16(1)
            .bytes(&short)
            .done(),
    );
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), z]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "cel image",
            ..
        })
    ));
    let long = zlib(&[0; 400]);
    let z = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 2, 0)
            .u16(1)
            .u16(1)
            .bytes(&long)
            .done(),
    );
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), z]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "cel image",
            ..
        })
    ));
}

#[test]
fn linked_cels_and_frames() {
    let raw = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 0, 0).u16(1).u16(1).u8(5).done(),
    );
    let link = chunk(0x2005, &cel_head(0, 3, 4, 255, 1, 0).u16(0).done());
    let h = Header {
        frames: 2,
        depth: 8,
        ..Header::default()
    };
    let b = file(
        &h,
        &[
            frame(100, &[layer_chunk(1, 0, 0, 0, 255, "a"), raw]),
            frame(100, &[link]),
        ],
    );
    let s = rp_io::read(&b).unwrap();
    assert_eq!(
        s.cel(LayerId(0), 1).unwrap().content,
        CelContent::Linked { frame: 0 }
    );
    assert_eq!(s.resolve_cel(LayerId(0), 1).unwrap().0, 0);
}

#[test]
fn cel_extra_attaches_to_previous_cel() {
    let raw = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 0, 0)
            .u16(1)
            .u16(1)
            .bytes(&[0; 4])
            .done(),
    );
    let extra = chunk(
        0x2006,
        &B::new()
            .u32(1)
            .i32(0x18000)
            .i32(-0x10000)
            .i32(0x20000)
            .i32(0x30000)
            .zeros(16)
            .done(),
    );
    let s = read1(
        &Header::default(),
        &[layer_chunk(1, 0, 0, 0, 255, "a"), raw, extra.clone()],
    )
    .unwrap();
    assert_eq!(
        s.cel(LayerId(0), 0).unwrap().extra,
        Some(CelExtra {
            flags: 1,
            x: 0x18000,
            y: -0x10000,
            width: 0x20000,
            height: 0x30000
        })
    );
    let r = read1(
        &Header::default(),
        &[layer_chunk(1, 0, 0, 0, 255, "a"), extra],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "orphan cel extra",
            ..
        })
    ));
}

#[test]
fn bad_cels_are_errors() {
    let c = chunk(
        0x2005,
        &cel_head(3, 0, 0, 255, 0, 0)
            .u16(1)
            .u16(1)
            .bytes(&[0; 4])
            .done(),
    );
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), c]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "cel layer",
            ..
        })
    ));
    let c = chunk(0x2005, &cel_head(0, 0, 0, 255, 9, 0).done());
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), c]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "cel type",
            ..
        })
    ));
    let c = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 0, 0)
            .u16(1)
            .u16(1)
            .bytes(&[0; 4])
            .done(),
    );
    let r = read1(
        &Header::default(),
        &[layer_chunk(1, 0, 0, 0, 255, "a"), c.clone(), c],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "duplicate cel",
            ..
        })
    ));
}

fn img_layer(id: u32, parent: Option<u32>, kind: LayerKind) -> Layer {
    Layer {
        id: LayerId(id),
        parent: parent.map(LayerId),
        name: format!("n{id}"),
        kind,
        flags: LayerFlags(1 | 2),
        blend_mode: BlendMode::Multiply,
        opacity: 77,
        uuid: None,
        user_data: UserData::default(),
    }
}

fn rich() -> Sprite {
    let mut s = Sprite::new(6, 5, ColorMode::Rgba);
    s.frames = vec![
        Frame { duration_ms: 80 },
        Frame { duration_ms: 90 },
        Frame { duration_ms: 100 },
    ];
    s.layers = vec![
        img_layer(0, None, LayerKind::Group),
        img_layer(1, Some(0), LayerKind::Image),
        img_layer(2, None, LayerKind::Image),
    ];
    let mut img = Image::new(2, 3, ColorMode::Rgba);
    img.put(
        1,
        2,
        PixelValue::Rgba {
            r: 9,
            g: 8,
            b: 7,
            a: 6,
        },
    )
    .unwrap();
    let cel = Cel {
        x: -2,
        y: 3,
        opacity: 99,
        z_index: 2,
        content: CelContent::Image(img),
        extra: None,
        user_data: UserData::default(),
    };
    s.cels.insert((LayerId(1), 0), cel.clone());
    s.cels.insert(
        (LayerId(1), 2),
        Cel {
            content: CelContent::Linked { frame: 0 },
            ..cel.clone()
        },
    );
    s.cels.insert(
        (LayerId(2), 1),
        Cel {
            extra: Some(CelExtra {
                flags: 1,
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            }),
            ..cel
        },
    );
    s
}

#[test]
fn layers_and_cels_round_trip() {
    let s = rich();
    assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
    let mut u = rich();
    u.flags = HeaderFlags(1 | 4);
    for (i, l) in u.layers.iter_mut().enumerate() {
        l.uuid = Some([i as u8; 16]);
    }
    assert_eq!(rp_io::read(&rp_io::write(&u).unwrap()).unwrap(), u);
}

#[test]
fn writer_validates_tree_ranges_and_uuids() {
    let mut s = rich();
    s.layers.swap(0, 1); // child before parent
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "layer tree",
            ..
        })
    ));
    let mut s = rich();
    s.cels.get_mut(&(LayerId(1), 0)).unwrap().x = 40_000;
    assert!(matches!(rp_io::write(&s), Err(AseError::OutOfRange { .. })));
    let mut s = rich();
    s.flags = HeaderFlags(1 | 4);
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "layer uuid",
            ..
        })
    ));
}
