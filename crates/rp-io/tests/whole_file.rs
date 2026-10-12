#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use rp_core::*;

fn kitchen_sink() -> Sprite {
    let mut s = Sprite::new(10, 7, ColorMode::Rgba);
    s.flags = HeaderFlags(1 | 2 | 4);
    s.frames = vec![
        Frame { duration_ms: 40 },
        Frame { duration_ms: 60 },
        Frame { duration_ms: 80 },
    ];
    s.palette.entries = (0..20u8)
        .map(|i| PaletteEntry {
            rgba: [i, i, i, 255],
            name: None,
        })
        .collect();
    s.user_data = UserData {
        text: Some("sink".into()),
        ..UserData::default()
    };
    let mk = |id: u32, parent: Option<u32>, kind: LayerKind| Layer {
        id: LayerId(id),
        parent: parent.map(LayerId),
        name: format!("layer {id}"),
        kind,
        flags: LayerFlags(3),
        blend_mode: BlendMode::Overlay,
        opacity: 200,
        uuid: Some([id as u8; 16]),
        user_data: UserData::default(),
    };
    s.layers = vec![
        mk(0, None, LayerKind::Group),
        mk(1, Some(0), LayerKind::Image),
        mk(2, None, LayerKind::Image),
    ];
    for f in 0..3u32 {
        let mut img = Image::new(3, 2, ColorMode::Rgba);
        img.put(
            f,
            1,
            PixelValue::Rgba {
                r: 10 * f as u8,
                g: 1,
                b: 2,
                a: 255,
            },
        )
        .unwrap();
        s.cels.insert(
            (LayerId(1), f),
            Cel {
                x: f as i32,
                y: -1,
                opacity: 255,
                z_index: f as i16 - 1,
                content: CelContent::Image(img),
                extra: None,
                user_data: UserData::default(),
            },
        );
    }
    s.cels.insert(
        (LayerId(2), 2),
        Cel {
            x: 0,
            y: 0,
            opacity: 128,
            z_index: 0,
            content: CelContent::Linked { frame: 0 },
            extra: None,
            user_data: UserData::default(),
        },
    );
    s.cels.insert(
        (LayerId(2), 0),
        Cel {
            x: 0,
            y: 0,
            opacity: 128,
            z_index: 0,
            content: CelContent::Image(Image::new(10, 7, ColorMode::Rgba)),
            extra: None,
            user_data: UserData::default(),
        },
    );
    s.tags = vec![Tag {
        from: 0,
        to: 2,
        direction: AniDir::Reverse,
        repeat: 0,
        legacy_rgb: [0; 3],
        name: "all".into(),
        user_data: UserData {
            color: Some([255, 0, 0, 255]),
            ..UserData::default()
        },
    }];
    s
}

#[test]
fn kitchen_sink_round_trips() {
    let s = kitchen_sink();
    assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
}

#[test]
fn cels_land_in_their_frames_and_sizes_add_up() {
    let b = rp_io::write(&kitchen_sink()).unwrap();
    let chunks = rp_io::dump_chunks(&b).unwrap();
    let cels_in = |f: u32| {
        chunks
            .iter()
            .filter(|c| c.frame == f && c.kind == 0x2005)
            .count()
    };
    assert_eq!((cels_in(0), cels_in(1), cels_in(2)), (2, 1, 2));
    // Frame byte counts tile the file exactly after the 128-byte header.
    let mut off = 128usize;
    for _ in 0..3 {
        let n = u32::from_le_bytes(b[off..off + 4].try_into().unwrap()) as usize;
        assert_eq!(
            u16::from_le_bytes(b[off + 4..off + 6].try_into().unwrap()),
            0xF1FA
        );
        off += n;
    }
    assert_eq!(off, b.len());
}

#[test]
fn every_truncation_of_a_rich_file_errors() {
    let b = rp_io::write(&kitchen_sink()).unwrap();
    for n in 0..b.len() {
        assert!(rp_io::read(&b[..n]).is_err(), "prefix {n}");
    }
}

#[test]
fn random_byte_flips_never_panic() {
    let b = rp_io::write(&kitchen_sink()).unwrap();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..3000 {
        let mut m = b.clone();
        for _ in 0..1 + (next() % 4) as usize {
            let i = (next() as usize) % m.len();
            m[i] = next() as u8;
        }
        let _ = rp_io::read(&m); // Ok or Err are both fine; a panic fails the test.
    }
}
