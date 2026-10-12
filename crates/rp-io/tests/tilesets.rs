#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

fn read1(h: &Header, chunks: &[Vec<u8>]) -> Result<Sprite, AseError> {
    rp_io::read(&file(h, &[frame(100, chunks)]))
}

fn tileset_chunk(count: u32, tw: u16, th: u16, pixels: &[u8]) -> Vec<u8> {
    let z = zlib(pixels);
    chunk(
        0x2023,
        &B::new()
            .u32(0)
            .u32(2 | 4)
            .u32(count)
            .u16(tw)
            .u16(th)
            .i16(1)
            .zeros(14)
            .string("ts")
            .u32(z.len() as u32)
            .bytes(&z)
            .done(),
    )
}

#[test]
fn embedded_tileset_splits_into_tiles() {
    // Indexed, 2x1 tiles, 3 tiles: rows [0,0] [1,2] [3,4].
    let c = tileset_chunk(3, 2, 1, &[0, 0, 1, 2, 3, 4]);
    let s = read1(
        &Header {
            depth: 8,
            ..Header::default()
        },
        &[c],
    )
    .unwrap();
    let ts = &s.tilesets[0];
    assert_eq!(
        (ts.tile_count, ts.tile_width, ts.tile_height, ts.base_index),
        (3, 2, 1, 1)
    );
    assert_eq!(ts.tiles.len(), 3);
    assert_eq!(ts.tiles[1].data(), &[1, 2]);
    assert_eq!(ts.tiles[2].data(), &[3, 4]);
    assert_eq!(ts.external, None);
}

#[test]
fn external_tileset_reference() {
    let c = chunk(
        0x2023,
        &B::new()
            .u32(4)
            .u32(1)
            .u32(10)
            .u16(8)
            .u16(8)
            .i16(0)
            .zeros(14)
            .string("ext")
            .u32(6)
            .u32(2)
            .done(),
    );
    let s = read1(&Header::default(), &[c]).unwrap();
    let ts = &s.tilesets[0];
    assert_eq!(
        ts.external,
        Some(ExternalTilesetRef {
            file_id: 6,
            tileset_id: 2
        })
    );
    assert!(ts.tiles.is_empty());
    assert_eq!(ts.tile_count, 10);
}

#[test]
fn wrong_tileset_image_size_is_an_error() {
    let c = tileset_chunk(3, 2, 1, &[0, 0, 1]);
    let r = read1(
        &Header {
            depth: 8,
            ..Header::default()
        },
        &[c],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "tileset image",
            ..
        })
    ));
}

#[test]
fn tileset_and_tile_user_data() {
    let c = tileset_chunk(3, 2, 1, &[0; 6]);
    let s = read1(
        &Header {
            depth: 8,
            ..Header::default()
        },
        &[
            c.clone(),
            ud_chunk(Some("set"), None),
            ud_chunk(None, None),
            ud_chunk(Some("tile1"), None),
            ud_chunk(None, None),
        ],
    )
    .unwrap();
    let ts = &s.tilesets[0];
    assert_eq!(ts.user_data.text.as_deref(), Some("set"));
    assert_eq!(
        ts.tile_user_data.len(),
        2,
        "trailing empty entry is trimmed"
    );
    assert_eq!(ts.tile_user_data[1].text.as_deref(), Some("tile1"));
    let too_many: Vec<Vec<u8>> = std::iter::once(c)
        .chain((0..5).map(|_| ud_chunk(None, None)))
        .collect();
    let r = read1(
        &Header {
            depth: 8,
            ..Header::default()
        },
        &too_many,
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "orphan user data",
            ..
        })
    ));
}

fn tilemap_cel(bits: u16, tiles_raw: &[u8], w: u16, h: u16) -> Vec<u8> {
    let z = zlib(tiles_raw);
    chunk(
        0x2005,
        &cel_head(0, 0, 0, 255, 3, 0)
            .u16(w)
            .u16(h)
            .u16(bits)
            .u32(0x1fff_ffff)
            .u32(0x2000_0000)
            .u32(0x4000_0000)
            .u32(0x8000_0000)
            .zeros(10)
            .bytes(&z)
            .done(),
    )
}

fn tilemap_layer() -> Vec<u8> {
    chunk(
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
    )
}

#[test]
fn tilemap_cels_32_and_16_bit() {
    let raw32 = B::new().u32(1).u32(0x2000_0002).done();
    let s = read1(
        &Header::default(),
        &[tilemap_layer(), tilemap_cel(32, &raw32, 2, 1)],
    )
    .unwrap();
    match &s.cel(LayerId(0), 0).unwrap().content {
        CelContent::Tilemap(t) => {
            assert_eq!((t.width, t.height, t.bits_per_tile), (2, 1, 32));
            assert_eq!(t.tiles, vec![1, 0x2000_0002]);
            assert_eq!(
                (
                    t.tile_id_mask,
                    t.x_flip_mask,
                    t.y_flip_mask,
                    t.diagonal_flip_mask
                ),
                (0x1fff_ffff, 0x2000_0000, 0x4000_0000, 0x8000_0000)
            );
        }
        other => panic!("expected tilemap, got {other:?}"),
    }
    let raw16 = B::new().u16(7).done();
    let s = read1(
        &Header::default(),
        &[tilemap_layer(), tilemap_cel(16, &raw16, 1, 1)],
    )
    .unwrap();
    match &s.cel(LayerId(0), 0).unwrap().content {
        CelContent::Tilemap(t) => assert_eq!(t.tiles, vec![7]),
        other => panic!("expected tilemap, got {other:?}"),
    }
}

#[test]
fn bad_tilemap_cels_are_errors() {
    let r = read1(
        &Header::default(),
        &[tilemap_layer(), tilemap_cel(12, &[0; 3], 1, 1)],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "tile bits",
            ..
        })
    ));
    let r = read1(
        &Header::default(),
        &[
            layer_chunk(1, 0, 0, 0, 255, "img"),
            tilemap_cel(32, &[0; 4], 1, 1),
        ],
    );
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "cel type",
            ..
        })
    ));
}

fn tiled_sprite() -> Sprite {
    let mut s = Sprite::new(4, 2, ColorMode::Rgba);
    let mut t0 = Image::new(2, 2, ColorMode::Rgba);
    t0.put(
        0,
        0,
        PixelValue::Rgba {
            r: 1,
            g: 1,
            b: 1,
            a: 255,
        },
    )
    .unwrap();
    s.tilesets = vec![Tileset {
        id: 0,
        flags: 2 | 4 | 8,
        tile_count: 2,
        tile_width: 2,
        tile_height: 2,
        base_index: 1,
        name: "set".into(),
        external: None,
        tiles: vec![Image::new(2, 2, ColorMode::Rgba), t0],
        user_data: UserData::default(),
        tile_user_data: vec![
            UserData::default(),
            UserData {
                text: Some("t1".into()),
                ..UserData::default()
            },
        ],
    }];
    s.layers = vec![Layer {
        id: LayerId(0),
        parent: None,
        name: "map".into(),
        kind: LayerKind::Tilemap { tileset_index: 0 },
        flags: LayerFlags(3),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    }];
    s.cels.insert(
        (LayerId(0), 0),
        Cel {
            x: 0,
            y: 0,
            opacity: 255,
            z_index: 0,
            extra: None,
            user_data: UserData::default(),
            content: CelContent::Tilemap(Tilemap {
                width: 2,
                height: 1,
                bits_per_tile: 32,
                tile_id_mask: 0x1fff_ffff,
                x_flip_mask: 0x2000_0000,
                y_flip_mask: 0x4000_0000,
                diagonal_flip_mask: 0x8000_0000,
                tiles: vec![1, 0x2000_0001],
            }),
        },
    );
    s
}

#[test]
fn tilesets_round_trip_with_order() {
    let s = tiled_sprite();
    let b = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&b).unwrap(), s);
    // profile, tileset, tileset UD, 2 tile UDs, layer, cel
    assert_eq!(
        kinds(&b, 0),
        vec![0x2007, 0x2023, 0x2020, 0x2020, 0x2020, 0x2004, 0x2005]
    );
}

#[test]
fn writer_validates_tilesets_and_tilemaps() {
    let mut s = tiled_sprite();
    s.tilesets[0].tiles.pop();
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "tileset",
            ..
        })
    ));
    let mut s = tiled_sprite();
    if let Some(Cel {
        content: CelContent::Tilemap(t),
        ..
    }) = s.cels.get_mut(&(LayerId(0), 0))
    {
        t.tiles.push(0);
    }
    assert!(matches!(
        rp_io::write(&s),
        Err(AseError::Invalid {
            what: "tilemap",
            ..
        })
    ));
}
