#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Regression tests for the second M0 review pass. Every Aseprite behaviour asserted here was
//! observed black-box on 2026-10-12 by opening crafted files in the installed Aseprite.
mod common;
use common::*;
use rp_core::*;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}.aseprite",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn text(t: &str) -> UserData {
    UserData {
        text: Some(t.into()),
        ..UserData::default()
    }
}

// R1. Observed: with tiles in the file, Aseprite reads one user-data chunk per tile after the
// tileset's own, and warns "Unexpected chunk type" if the run is shorter.
#[test]
fn tiles_in_file_pad_tile_user_data_to_tile_count() {
    let mut s = rp_io::read(&fixture("tilemap")).unwrap();
    let ts = &mut s.tilesets[0];
    assert_eq!(ts.tile_count, 3);
    ts.user_data = UserData::default();
    ts.tile_user_data = vec![text("t0")];
    let b = rp_io::write(&s).unwrap();
    let k = kinds(&b, 0);
    let at = k.iter().position(|&c| c == 0x2023).unwrap();
    assert_eq!(&k[at..at + 5], &[0x2023, 0x2020, 0x2020, 0x2020, 0x2020]);
    assert_eq!(rp_io::read(&b).unwrap(), s);
}

// R2. Observed: Aseprite skips unknown chunks (here 0x2017) when routing user data, so the
// user data belongs to the object before them.
#[test]
fn user_data_after_an_unknown_chunk_attaches_to_the_previous_object() {
    let cel = chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 0, 0)
            .u16(1)
            .u16(1)
            .bytes(&[0; 4])
            .done(),
    );
    let b = file(
        &Header::default(),
        &[frame(
            100,
            &[
                layer_chunk(1, 0, 0, 0, 255, "A"),
                chunk(0x2017, &[]),
                ud_chunk(Some("for A"), None),
                cel,
                ud_chunk(Some("cel"), None),
            ],
        )],
    );
    let s = rp_io::read(&b).unwrap();
    assert_eq!(s.layers[0].user_data.text.as_deref(), Some("for A"));
    assert_eq!(
        s.cel(LayerId(0), 0).unwrap().user_data.text.as_deref(),
        Some("cel")
    );
    assert_eq!(
        s.raw_chunks,
        vec![RawChunk {
            frame: 0,
            kind: 0x2017,
            data: vec![]
        }]
    );
    assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
}

// R9. Observed: user data at the start of frame 0, with no palette, and also after a colour
// profile chunk, belongs to the sprite.
#[test]
fn user_data_at_the_start_of_frame_zero_belongs_to_the_sprite() {
    let profile = chunk(0x2007, &B::new().u16(1).u16(0).i32(0).zeros(8).done());
    for lead in [vec![], vec![profile]] {
        let mut chunks = lead.clone();
        chunks.push(ud_chunk(Some("note"), None));
        chunks.push(layer_chunk(1, 0, 0, 0, 255, "A"));
        let s = rp_io::read(&file(&Header::default(), &[frame(100, &chunks)])).unwrap();
        assert_eq!(s.user_data.text.as_deref(), Some("note"));
        assert!(s.layers[0].user_data.is_empty());
        assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
    }
}

// R13. Stronger invariant than review_fixes.rs: anything we can read, we can save, and the
// saved file reads back equal.
#[test]
fn anything_readable_is_writable() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for name in [
        "tilemap",
        "user_data",
        "animation",
        "layers_tree",
        "slices",
        "indexed",
    ] {
        let base = fixture(name);
        for _ in 0..1500 {
            let mut m = base.clone();
            for _ in 0..1 + (next() % 3) as usize {
                let i = (next() as usize) % m.len();
                m[i] = next() as u8;
            }
            let Ok(s) = rp_io::read(&m) else { continue };
            let out = rp_io::write(&s)
                .unwrap_or_else(|e| panic!("{name}: read OK but write failed: {e}"));
            assert_eq!(rp_io::read(&out).unwrap(), s, "{name}");
        }
    }
}

// R4, R4b. Writing must not scan every cel or raw chunk once per frame.
#[test]
fn many_frames_with_cels_and_raw_chunks_write_in_linear_time() {
    let frames = 20_000u32;
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.frames = vec![Frame { duration_ms: 10 }; frames as usize];
    s.layers = vec![Layer {
        id: LayerId(0),
        parent: None,
        name: "a".into(),
        kind: LayerKind::Image,
        flags: LayerFlags(3),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    }];
    let cel = |content| Cel {
        x: 0,
        y: 0,
        opacity: 255,
        z_index: 0,
        content,
        extra: None,
        user_data: UserData::default(),
    };
    s.cels.insert(
        (LayerId(0), 0),
        cel(CelContent::Image(Image::new(1, 1, ColorMode::Rgba))),
    );
    for f in 1..frames {
        s.cels
            .insert((LayerId(0), f), cel(CelContent::Linked { frame: 0 }));
        s.raw_chunks.push(RawChunk {
            frame: f,
            kind: 0x2099,
            data: vec![1],
        });
    }
    let t = std::time::Instant::now();
    let b = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&b).unwrap(), s);
    assert!(
        t.elapsed() < std::time::Duration::from_secs(8),
        "took {:?}",
        t.elapsed()
    );
}
