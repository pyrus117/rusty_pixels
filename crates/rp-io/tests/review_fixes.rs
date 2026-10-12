#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Regression tests for the M0 independent review (findings H1, H2, M1, M2, M5, M6, M7, L1, L3, L4, L11, L12).
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

fn read1(h: &Header, chunks: &[Vec<u8>]) -> Result<Sprite, AseError> {
    rp_io::read(&file(h, &[frame(100, chunks)]))
}

fn image_layer(id: u32, parent: Option<u32>, kind: LayerKind) -> Layer {
    Layer {
        id: LayerId(id),
        parent: parent.map(LayerId),
        name: format!("l{id}"),
        kind,
        flags: LayerFlags(3),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    }
}

fn cel_with(content: CelContent) -> Cel {
    Cel {
        x: 0,
        y: 0,
        opacity: 255,
        z_index: 0,
        content,
        extra: None,
        user_data: UserData::default(),
    }
}

// A tiny zlib stream; the budget checks must reject the chunk before it is inflated.
const JUNK_ZLIB: [u8; 4] = [0x78, 0x9c, 0x03, 0x00];

fn is_budget(r: &Result<Sprite, AseError>) -> bool {
    matches!(
        r,
        Err(AseError::Invalid {
            what: "decode budget",
            ..
        })
    )
}

// H1
#[test]
fn huge_tile_count_without_tiles_writes_only_existing_tile_user_data() {
    let ts = chunk(
        0x2023,
        &B::new()
            .u32(0)
            .u32(0)
            .u32(3_000_000)
            .u16(8)
            .u16(8)
            .i16(1)
            .zeros(14)
            .string("t")
            .done(),
    );
    let s = read1(
        &Header::default(),
        &[
            ts,
            ud_chunk(Some("set"), None),
            ud_chunk(Some("tile0"), None),
        ],
    )
    .unwrap();
    assert_eq!(s.tilesets[0].tile_count, 3_000_000);
    let b = rp_io::write(&s).unwrap();
    assert!(b.len() < 4096, "wrote {} bytes", b.len());
    assert_eq!(rp_io::read(&b).unwrap(), s);
}

// H2
#[test]
fn tileset_tiles_are_charged_to_the_budget_before_inflating() {
    let count = (rp_io::DECODE_BUDGET / (1 + rp_io::OBJECT_OVERHEAD) + 1) as u32;
    let c = chunk(
        0x2023,
        &B::new()
            .u32(0)
            .u32(2 | 4)
            .u32(count)
            .u16(1)
            .u16(1)
            .i16(1)
            .zeros(14)
            .string("t")
            .u32(JUNK_ZLIB.len() as u32)
            .bytes(&JUNK_ZLIB)
            .done(),
    );
    assert!(is_budget(&read1(
        &Header {
            depth: 8,
            ..Header::default()
        },
        &[c]
    )));
}

// M1
#[test]
fn tilemap_cels_are_charged_to_the_budget_before_inflating() {
    let layer = chunk(
        0x2004,
        &B::new()
            .u16(3)
            .u16(2)
            .u16(0)
            .u16(0)
            .u16(0)
            .u16(0)
            .u8(255)
            .zeros(3)
            .string("map")
            .u32(0)
            .done(),
    );
    let cel = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 3, 0)
            .u16(65535)
            .u16(65535)
            .u16(8)
            .u32(0x1fff_ffff)
            .u32(0x8000_0000)
            .u32(0x4000_0000)
            .u32(0x2000_0000)
            .zeros(10)
            .bytes(&JUNK_ZLIB)
            .done(),
    );
    assert!(is_budget(&read1(&Header::default(), &[layer, cel])));
}

#[test]
fn image_cels_are_charged_to_the_budget_before_inflating() {
    let cel = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 2, 0)
            .u16(65535)
            .u16(65535)
            .bytes(&JUNK_ZLIB)
            .done(),
    );
    assert!(is_budget(&read1(
        &Header::default(),
        &[layer_chunk(1, 0, 0, 0, 255, "a"), cel]
    )));
}

// M2
#[test]
fn old_palette_past_256_entries_is_rejected() {
    let c = chunk(
        0x0004,
        &B::new()
            .u16(2)
            .u8(255)
            .u8(1)
            .bytes(&[1, 1, 1])
            .u8(255)
            .u8(1)
            .bytes(&[2, 2, 2])
            .done(),
    );
    let r = read1(&Header::default(), &[c]);
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "old palette",
            ..
        })
    ));
}

// M5
#[test]
fn unknown_chunks_survive_in_their_frame() {
    let h = Header {
        frames: 2,
        ..Header::default()
    };
    let b = file(
        &h,
        &[
            frame(
                100,
                &[layer_chunk(1, 0, 0, 0, 255, "a"), chunk(0x2099, &[1, 2, 3])],
            ),
            frame(100, &[chunk(0x2016, &[9; 12]), chunk(0x3000, &[])]),
        ],
    );
    let s = rp_io::read(&b).unwrap();
    assert_eq!(
        s.raw_chunks,
        vec![
            RawChunk {
                frame: 0,
                kind: 0x2099,
                data: vec![1, 2, 3]
            },
            RawChunk {
                frame: 1,
                kind: 0x2016,
                data: vec![9; 12]
            },
            RawChunk {
                frame: 1,
                kind: 0x3000,
                data: vec![]
            },
        ]
    );
    let out = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&out).unwrap(), s);
    assert_eq!(kinds(&out, 1), vec![0x2016, 0x3000]);
}

// Revised 2026-10-12 from black-box observation: Aseprite ignores unknown chunks when routing
// user data, and at the start of frame 0 user data belongs to the sprite.
#[test]
fn user_data_after_an_unknown_chunk_at_frame_start_goes_to_the_sprite() {
    let b = file(
        &Header::default(),
        &[frame(
            100,
            &[chunk(0x2099, &[1]), ud_chunk(Some("x"), None)],
        )],
    );
    let s = rp_io::read(&b).unwrap();
    assert_eq!(s.user_data.text.as_deref(), Some("x"));
    let k: Vec<u16> = s.raw_chunks.iter().map(|c| c.kind).collect();
    assert_eq!(k, vec![0x2099]);
    let out = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&out).unwrap(), s);
}

#[test]
fn writer_rejects_raw_chunk_past_last_frame() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.raw_chunks.push(RawChunk {
        frame: 5,
        kind: 0x2099,
        data: vec![],
    });
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "raw chunk",
            ..
        })
    ));
}

// M6
#[test]
fn palette_chunks_after_frame_zero_are_kept_raw() {
    let pal = |v: u8| {
        chunk(
            0x2019,
            &B::new()
                .u32(1)
                .u32(0)
                .u32(0)
                .zeros(8)
                .u16(0)
                .bytes(&[v, v, v, 255])
                .done(),
        )
    };
    let old = chunk(
        0x0004,
        &B::new().u16(1).u8(0).u8(1).bytes(&[50, 50, 50]).done(),
    );
    let h = Header {
        frames: 2,
        depth: 8,
        ..Header::default()
    };
    let s = rp_io::read(&file(
        &h,
        &[frame(100, &[pal(10)]), frame(100, &[pal(200), old])],
    ))
    .unwrap();
    assert_eq!(s.palette.entries.len(), 1);
    assert_eq!(s.palette.entries[0].rgba, [10, 10, 10, 255]);
    let raw: Vec<(u32, u16)> = s.raw_chunks.iter().map(|c| (c.frame, c.kind)).collect();
    assert_eq!(raw, vec![(1, 0x2019), (1, 0x0004)]);
    let out = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&out).unwrap(), s);
}

// M7
#[test]
fn many_layers_read_and_write_in_linear_time() {
    let n = 30_000;
    let chunks: Vec<Vec<u8>> = (0..n)
        .map(|i| layer_chunk(1, 0, 0, 0, 255, &format!("l{i}")))
        .collect();
    let b = file(&Header::default(), &[frame(100, &chunks)]);
    let t = std::time::Instant::now();
    let s = rp_io::read(&b).unwrap();
    let out = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&out).unwrap().layers.len(), n);
    assert!(
        t.elapsed() < std::time::Duration::from_secs(8),
        "took {:?}",
        t.elapsed()
    );
}

// L1 (a)
#[test]
fn writer_rejects_cel_image_in_the_wrong_mode() {
    let mut s = Sprite::new(2, 2, ColorMode::Indexed);
    s.layers = vec![image_layer(0, None, LayerKind::Image)];
    s.cels.insert(
        (LayerId(0), 0),
        cel_with(CelContent::Image(
            Image::new(1, 1, ColorMode::Rgba).unwrap(),
        )),
    );
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "cel image",
            ..
        })
    ));
}

// L1 (b)
#[test]
fn writer_rejects_zero_size_tiles_in_file() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.tilesets = vec![Tileset {
        id: 0,
        flags: 2,
        tile_count: 1,
        tile_width: 0,
        tile_height: 0,
        base_index: 1,
        name: "t".into(),
        external: None,
        tiles: vec![Image::new(0, 0, ColorMode::Rgba).unwrap()],
        user_data: UserData::default(),
        tile_user_data: vec![],
    }];
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "tileset",
            ..
        })
    ));
}

// L1 (c)
#[test]
fn reader_rejects_zero_frames() {
    let r = rp_io::read(&file(
        &Header {
            frames: 0,
            ..Header::default()
        },
        &[],
    ));
    assert!(matches!(r, Err(AseError::Invalid { what: "frames", .. })));
}

// L1 (d)
#[test]
fn writer_rejects_icc_bytes_without_icc_kind() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.color_profile = Some(ColorProfile {
        kind: 1,
        flags: 0,
        gamma: 0,
        icc: vec![1, 2],
    });
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "color profile",
            ..
        })
    ));
}

// L1 (e) and the contiguity rule
#[test]
fn writer_rejects_duplicate_ids_and_split_subtrees() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.layers = vec![
        image_layer(0, None, LayerKind::Image),
        image_layer(0, None, LayerKind::Image),
    ];
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "layer tree",
            ..
        })
    ));
    s.layers = vec![
        image_layer(0, None, LayerKind::Group),
        image_layer(1, Some(0), LayerKind::Image),
        image_layer(2, None, LayerKind::Image),
        image_layer(3, Some(0), LayerKind::Image),
    ];
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "layer tree",
            ..
        })
    ));
}

// L1 (f)
#[test]
fn tag_ranges_are_checked_both_ways() {
    let tags = |from: u16, to: u16| {
        chunk(
            0x2018,
            &B::new()
                .u16(1)
                .zeros(8)
                .u16(from)
                .u16(to)
                .u8(0)
                .u16(0)
                .zeros(6)
                .bytes(&[0; 3])
                .u8(0)
                .string("t")
                .done(),
        )
    };
    for (from, to) in [(0, 1), (1, 0)] {
        let r = read1(&Header::default(), &[tags(from, to)]);
        assert!(
            matches!(
                r,
                Err(AseError::Invalid {
                    what: "tag range",
                    ..
                })
            ),
            "{from}..{to}"
        );
    }
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.tags = vec![Tag {
        from: 0,
        to: 3,
        direction: AniDir::Forward,
        repeat: 0,
        legacy_rgb: [0; 3],
        name: "t".into(),
        user_data: UserData::default(),
    }];
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "tag range",
            ..
        })
    ));
}

// L3
#[test]
fn new_palette_fully_replaces_old_palette() {
    let old = chunk(
        0x0004,
        &B::new()
            .u16(1)
            .u8(0)
            .u8(2)
            .bytes(&[7, 7, 7, 8, 8, 8])
            .done(),
    );
    let newp = chunk(
        0x2019,
        &B::new()
            .u32(2)
            .u32(0)
            .u32(0)
            .zeros(8)
            .u16(0)
            .bytes(&[1, 1, 1, 255])
            .done(),
    );
    for order in [vec![old.clone(), newp.clone()], vec![newp, old]] {
        let s = read1(&Header::default(), &order).unwrap();
        let rgba: Vec<[u8; 4]> = s.palette.entries.iter().map(|e| e.rgba).collect();
        assert_eq!(rgba, vec![[1, 1, 1, 255], [0, 0, 0, 255]]);
    }
}

// L4
#[test]
fn second_user_data_for_one_owner_is_rejected() {
    let r = read1(
        &Header::default(),
        &[
            layer_chunk(1, 0, 0, 0, 255, "a"),
            ud_chunk(Some("1"), None),
            ud_chunk(Some("2"), None),
        ],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "duplicate user data",
            ..
        })
    ));
}

/// One user-properties map holding `name` with a value of `type_code`.
fn props_ud(name: &str, type_code: u16, value: &[u8]) -> Vec<u8> {
    let prop = B::new().string(name).u16(type_code).bytes(value).done();
    let maps = B::new().u32(0).u32(1).bytes(&prop).done();
    chunk(
        0x2020,
        &B::new()
            .u32(4)
            .u32(8 + maps.len() as u32)
            .u32(1)
            .bytes(&maps)
            .done(),
    )
}

fn nested_maps(total: usize) -> Vec<u8> {
    // `total` maps in all: the outer property value plus total-1 nested inside it.
    let mut inner = B::new().u32(0).done();
    for _ in 0..total - 1 {
        inner = B::new().u32(1).string("d").u16(0x12).bytes(&inner).done();
    }
    inner
}

// L11
#[test]
fn properties_depth_limit_is_exactly_128() {
    let pal = chunk(
        0x2019,
        &B::new()
            .u32(1)
            .u32(0)
            .u32(0)
            .zeros(8)
            .u16(0)
            .bytes(&[0, 0, 0, 255])
            .done(),
    );
    assert!(
        read1(
            &Header::default(),
            &[pal.clone(), props_ud("p", 0x12, &nested_maps(128))]
        )
        .is_ok()
    );
    let r = read1(
        &Header::default(),
        &[pal, props_ud("p", 0x12, &nested_maps(129))],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "properties depth",
            ..
        })
    ));
}

// L12: anything that reads must write without panicking, and anything written must read back equal.
#[test]
fn mutated_fixtures_round_trip_or_error_cleanly() {
    let dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for name in ["tilemap", "user_data", "animation", "layers_tree"] {
        let base = std::fs::read(format!("{dir}/{name}.aseprite")).unwrap();
        for _ in 0..1500 {
            let mut m = base.clone();
            for _ in 0..1 + (next() % 3) as usize {
                let i = (next() as usize) % m.len();
                m[i] = next() as u8;
            }
            let Ok(s) = rp_io::read(&m) else { continue };
            let Ok(out) = rp_io::write(&s) else { continue };
            assert_eq!(
                rp_io::read(&out).unwrap(),
                s,
                "{name}: written file did not read back equal"
            );
        }
    }
}
