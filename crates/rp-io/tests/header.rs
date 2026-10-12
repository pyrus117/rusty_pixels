#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

#[test]
fn reads_header_fields() {
    let h = Header {
        width: 4,
        height: 3,
        flags: 5,
        transparent_index: 7,
        pixel_w: 2,
        pixel_h: 1,
        grid: (-1, 2, 8, 4),
        ..Header::default()
    };
    let s = rp_io::read(&file(&h, &[frame(120, &[])])).unwrap();
    assert_eq!((s.width, s.height, s.color_mode), (4, 3, ColorMode::Rgba));
    assert_eq!(s.flags, HeaderFlags(5));
    assert_eq!(s.transparent_index, 7);
    assert_eq!((s.pixel_width, s.pixel_height), (2, 1));
    assert_eq!(
        s.grid,
        Grid {
            x: -1,
            y: 2,
            width: 8,
            height: 4
        }
    );
    assert_eq!(s.frames, vec![Frame { duration_ms: 120 }]);
    assert!(s.layers.is_empty() && s.cels.is_empty() && s.palette.entries.is_empty());
    assert_eq!(s.color_profile, None);
}

#[test]
fn depth_selects_color_mode() {
    for (depth, mode) in [
        (32, ColorMode::Rgba),
        (16, ColorMode::Grayscale),
        (8, ColorMode::Indexed),
    ] {
        let s = rp_io::read(&file(
            &Header {
                depth,
                ..Header::default()
            },
            &[frame(100, &[])],
        ))
        .unwrap();
        assert_eq!(s.color_mode, mode);
    }
    let err = rp_io::read(&file(
        &Header {
            depth: 24,
            ..Header::default()
        },
        &[frame(100, &[])],
    ));
    assert!(matches!(err, Err(AseError::UnsupportedDepth(24))));
}

#[test]
fn bad_magics_are_rejected() {
    let mut b = file(&Header::default(), &[frame(100, &[])]);
    b[4] = 0;
    assert!(matches!(rp_io::read(&b), Err(AseError::BadMagic { .. })));
    let mut b = file(&Header::default(), &[frame(100, &[])]);
    b[128 + 4] = 0;
    assert!(matches!(
        rp_io::read(&b),
        Err(AseError::BadFrameMagic { frame: 0, .. })
    ));
}

#[test]
fn zero_frame_duration_uses_header_speed() {
    let h = Header {
        frames: 2,
        speed: 77,
        ..Header::default()
    };
    let s = rp_io::read(&file(&h, &[frame(0, &[]), frame(30, &[])])).unwrap();
    assert_eq!(
        s.frames,
        vec![Frame { duration_ms: 77 }, Frame { duration_ms: 30 }]
    );
    let h = Header {
        speed: 0,
        ..Header::default()
    };
    let s = rp_io::read(&file(&h, &[frame(0, &[])])).unwrap();
    assert_eq!(s.frames, vec![Frame { duration_ms: 100 }]);
}

#[test]
fn chunk_count_falls_back_to_old_field() {
    let c = chunk(0x7777, &[1, 2, 3]);
    let b = file(
        &Header::default(),
        &[frame_counts(100, 1, 0, std::slice::from_ref(&c))],
    );
    assert_eq!(
        rp_io::dump_chunks(&b).unwrap(),
        vec![rp_io::ChunkInfo {
            frame: 0,
            kind: 0x7777,
            size: 9
        }]
    );
    let b = file(
        &Header::default(),
        &[frame_counts(100, 0xFFFF, 2, &[c.clone(), c])],
    );
    assert_eq!(rp_io::dump_chunks(&b).unwrap().len(), 2);
}

#[test]
fn unknown_and_deprecated_chunks_are_skipped() {
    let b = file(
        &Header::default(),
        &[frame(
            100,
            &[
                chunk(0x7777, &[9; 10]),
                chunk(0x2017, &[]),
                chunk(0x2016, &[0; 4]),
            ],
        )],
    );
    assert!(rp_io::read(&b).is_ok());
}

#[test]
fn bad_chunk_sizes_are_errors() {
    let tiny = B::new().u32(5).u16(0x7777).done();
    let b = file(&Header::default(), &[frame(100, &[tiny])]);
    assert!(matches!(
        rp_io::read(&b),
        Err(AseError::Invalid {
            what: "chunk size",
            ..
        })
    ));
    let huge = B::new().u32(10_000).u16(0x7777).done();
    let b = file(&Header::default(), &[frame(100, &[huge])]);
    assert!(rp_io::read(&b).is_err());
}

#[test]
fn every_truncation_is_an_error_not_a_panic() {
    let b = file(
        &Header {
            frames: 2,
            ..Header::default()
        },
        &[frame(100, &[chunk(0x7777, &[1; 20])]), frame(50, &[])],
    );
    for n in 0..b.len() {
        assert!(rp_io::read(&b[..n]).is_err(), "prefix {n} should fail");
    }
}

#[test]
fn write_then_read_preserves_header_and_frames() {
    let mut s = Sprite::new(5, 6, ColorMode::Grayscale);
    s.frames = vec![Frame { duration_ms: 50 }, Frame { duration_ms: 70 }];
    s.grid = Grid {
        x: 3,
        y: -4,
        width: 10,
        height: 12,
    };
    s.pixel_width = 2;
    s.flags = HeaderFlags(3);
    s.color_profile = None;
    let b = rp_io::write(&s).unwrap();
    assert_eq!(
        u32::from_le_bytes(b[0..4].try_into().unwrap()) as usize,
        b.len()
    );
    assert_eq!(rp_io::read(&b).unwrap(), s);
}

#[test]
fn writer_rejects_zero_duration_and_no_frames() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.frames[0].duration_ms = 0;
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { .. })));
    s.frames.clear();
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { .. })));
}
