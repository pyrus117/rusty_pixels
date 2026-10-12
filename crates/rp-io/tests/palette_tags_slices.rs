#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

fn read1(chunks: &[Vec<u8>]) -> Result<Sprite, AseError> {
    rp_io::read(&file(&Header::default(), &[frame(100, chunks)]))
}

fn new_palette(size: u32, first: u32, entries: &[([u8; 4], Option<&str>)]) -> Vec<u8> {
    let mut b = B::new()
        .u32(size)
        .u32(first)
        .u32(first + entries.len() as u32 - 1)
        .zeros(8);
    for (rgba, name) in entries {
        b = b.u16(if name.is_some() { 1 } else { 0 }).bytes(rgba);
        if let Some(n) = name {
            b = b.string(n);
        }
    }
    chunk(0x2019, &b.done())
}

#[test]
fn new_palette_with_names_and_resize() {
    let s = read1(&[new_palette(
        4,
        1,
        &[([1, 2, 3, 4], Some("skin")), ([5, 6, 7, 8], None)],
    )])
    .unwrap();
    assert_eq!(s.palette.entries.len(), 4);
    assert_eq!(
        s.palette.entries[0],
        PaletteEntry {
            rgba: [0, 0, 0, 255],
            name: None
        }
    );
    assert_eq!(
        s.palette.entries[1],
        PaletteEntry {
            rgba: [1, 2, 3, 4],
            name: Some("skin".into())
        }
    );
    assert_eq!(s.palette.entries[2].rgba, [5, 6, 7, 8]);
    let bad = chunk(0x2019, &B::new().u32(2).u32(1).u32(5).zeros(8).done());
    assert!(matches!(
        read1(&[bad]),
        Err(AseError::Invalid {
            what: "palette range",
            ..
        })
    ));
}

#[test]
fn old_palettes() {
    // Packet 1: skip 0, 2 colours. Packet 2: skip 1, 1 colour -> index 3.
    let old = chunk(
        0x0004,
        &B::new()
            .u16(2)
            .u8(0)
            .u8(2)
            .bytes(&[10, 20, 30, 40, 50, 60])
            .u8(1)
            .u8(1)
            .bytes(&[7, 8, 9])
            .done(),
    );
    let s = read1(std::slice::from_ref(&old)).unwrap();
    let rgba: Vec<[u8; 4]> = s.palette.entries.iter().map(|e| e.rgba).collect();
    assert_eq!(
        rgba,
        vec![
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            [0, 0, 0, 255],
            [7, 8, 9, 255]
        ]
    );

    let six = chunk(
        0x0011,
        &B::new().u16(1).u8(0).u8(1).bytes(&[63, 0, 21]).done(),
    );
    assert_eq!(
        read1(&[six]).unwrap().palette.entries[0].rgba,
        [255, 0, 85, 255]
    );

    // New palette wins whichever comes first.
    let newp = new_palette(1, 0, &[([1, 1, 1, 1], None)]);
    assert_eq!(
        read1(&[old.clone(), newp.clone()])
            .unwrap()
            .palette
            .entries
            .len(),
        1
    );
    assert_eq!(read1(&[newp, old]).unwrap().palette.entries.len(), 1);
}

#[test]
fn color_profile_and_external_files() {
    let icc = chunk(
        0x2007,
        &B::new()
            .u16(2)
            .u16(1)
            .i32(0x10000)
            .zeros(8)
            .u32(3)
            .bytes(&[1, 2, 3])
            .done(),
    );
    let ext = chunk(
        0x2008,
        &B::new()
            .u32(2)
            .zeros(8)
            .u32(5)
            .u8(2)
            .zeros(7)
            .string("me/Ext")
            .u32(6)
            .u8(1)
            .zeros(7)
            .string("tiles.aseprite")
            .done(),
    );
    let s = read1(&[icc, ext]).unwrap();
    assert_eq!(
        s.color_profile,
        Some(ColorProfile {
            kind: 2,
            flags: 1,
            gamma: 0x10000,
            icc: vec![1, 2, 3]
        })
    );
    assert_eq!(
        s.external_files,
        vec![
            ExternalFile {
                id: 5,
                kind: 2,
                name: "me/Ext".into()
            },
            ExternalFile {
                id: 6,
                kind: 1,
                name: "tiles.aseprite".into()
            },
        ]
    );
}

#[allow(clippy::type_complexity)]
fn tags_chunk(tags: &[(u16, u16, u8, u16, [u8; 3], &str)]) -> Vec<u8> {
    let mut b = B::new().u16(tags.len() as u16).zeros(8);
    for (from, to, dir, rep, rgb, name) in tags {
        b = b
            .u16(*from)
            .u16(*to)
            .u8(*dir)
            .u16(*rep)
            .zeros(6)
            .bytes(rgb)
            .u8(0)
            .string(name);
    }
    chunk(0x2018, &b.done())
}

#[test]
fn tags_all_directions() {
    let c = tags_chunk(&[
        (0, 1, 0, 0, [1, 2, 3], "idle"),
        (1, 2, 1, 3, [0; 3], "back"),
        (0, 2, 2, 1, [0; 3], "pp"),
        (2, 2, 3, 0, [0; 3], "ppr"),
    ]);
    let h = Header {
        frames: 3,
        ..Header::default()
    };
    let s = rp_io::read(&file(
        &h,
        &[frame(100, &[c]), frame(100, &[]), frame(100, &[])],
    ))
    .unwrap();
    let dirs: Vec<AniDir> = s.tags.iter().map(|t| t.direction).collect();
    assert_eq!(
        dirs,
        vec![
            AniDir::Forward,
            AniDir::Reverse,
            AniDir::PingPong,
            AniDir::PingPongReverse
        ]
    );
    assert_eq!((s.tags[1].from, s.tags[1].to, s.tags[1].repeat), (1, 2, 3));
    assert_eq!(s.tags[0].legacy_rgb, [1, 2, 3]);
    assert_eq!(s.tags[0].name, "idle");
    let bad = tags_chunk(&[(0, 0, 7, 0, [0; 3], "x")]);
    assert!(matches!(
        read1(&[bad]),
        Err(AseError::Invalid {
            what: "tag direction",
            ..
        })
    ));
}

#[test]
fn slices_with_center_and_pivot() {
    let c = chunk(
        0x2022,
        &B::new()
            .u32(2)
            .u32(3)
            .u32(0)
            .string("button")
            .u32(0)
            .i32(1)
            .i32(2)
            .u32(10)
            .u32(8)
            .i32(2)
            .i32(2)
            .u32(6)
            .u32(4)
            .i32(5)
            .i32(4)
            .u32(2)
            .i32(0)
            .i32(0)
            .u32(0)
            .u32(0)
            .i32(0)
            .i32(0)
            .u32(0)
            .u32(0)
            .i32(0)
            .i32(0)
            .done(),
    );
    let s = read1(&[c]).unwrap();
    let sl = &s.slices[0];
    assert_eq!((sl.name.as_str(), sl.flags), ("button", 3));
    assert_eq!(
        sl.keys[0],
        SliceKey {
            frame: 0,
            x: 1,
            y: 2,
            width: 10,
            height: 8,
            center: Some(SliceCenter {
                x: 2,
                y: 2,
                width: 6,
                height: 4
            }),
            pivot: Some(Pivot { x: 5, y: 4 })
        }
    );
    assert_eq!(sl.keys[1].frame, 2);
    assert_eq!(sl.keys[1].width, 0);
}

fn sprite_with_everything() -> Sprite {
    let mut s = Sprite::new(8, 8, ColorMode::Indexed);
    s.frames = vec![Frame { duration_ms: 100 }; 3];
    s.palette.entries = vec![
        PaletteEntry {
            rgba: [0, 0, 0, 0],
            name: None,
        },
        PaletteEntry {
            rgba: [255, 0, 0, 128],
            name: Some("red".into()),
        },
    ];
    s.color_profile = Some(ColorProfile {
        kind: 2,
        flags: 0,
        gamma: 0,
        icc: vec![9, 9],
    });
    s.external_files = vec![ExternalFile {
        id: 1,
        kind: 2,
        name: "a/b".into(),
    }];
    s.tags = vec![Tag {
        from: 0,
        to: 2,
        direction: AniDir::PingPong,
        repeat: 2,
        legacy_rgb: [0, 0, 0],
        name: "walk".into(),
        user_data: UserData::default(),
    }];
    s.slices = vec![Slice {
        name: "s".into(),
        flags: Slice::PIVOT,
        user_data: UserData::default(),
        keys: vec![SliceKey {
            frame: 1,
            x: -1,
            y: 0,
            width: 3,
            height: 3,
            center: None,
            pivot: Some(Pivot { x: 1, y: 1 }),
        }],
    }];
    s
}

#[test]
fn everything_round_trips_and_order_matches_aseprite() {
    let s = sprite_with_everything();
    let b = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&b).unwrap(), s);
    assert_eq!(
        kinds(&b, 0),
        vec![0x2008, 0x2007, 0x2019, 0x2018, 0x2020, 0x2022]
    );
}

#[test]
fn writer_rejects_inconsistent_slice_keys() {
    let mut s = sprite_with_everything();
    s.slices[0].keys[0].pivot = None;
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "slice key",
            ..
        })
    ));
}
