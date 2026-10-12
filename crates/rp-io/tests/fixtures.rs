#![allow(clippy::unwrap_used, clippy::expect_used)]
use rp_core::*;

fn load(name: &str) -> Sprite {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    rp_io::read(&std::fs::read(&path).unwrap()).unwrap()
}

const ALL: &[&str] = &[
    "rgba_basic",
    "grayscale",
    "indexed",
    "layers_tree",
    "animation",
    "slices",
    "user_data",
    "tilemap",
    "uuid_layers",
    "ratio_grid",
];

fn layer<'a>(s: &'a Sprite, name: &str) -> &'a Layer {
    s.layers
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("no layer {name}"))
}

fn tag<'a>(s: &'a Sprite, name: &str) -> &'a Tag {
    s.tags
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("no tag {name}"))
}

/// Pixel bytes at sprite coordinates, through the cel offset and any link.
fn pixel_at(s: &Sprite, id: LayerId, frame: u32, x: i32, y: i32) -> Option<Vec<u8>> {
    let (_, cel) = s.resolve_cel(id, frame)?;
    let CelContent::Image(img) = &cel.content else {
        return None;
    };
    let (lx, ly) = (x - cel.x, y - cel.y);
    if lx < 0 || ly < 0 {
        return None;
    }
    img.get(lx as u32, ly as u32).map(<[u8]>::to_vec)
}

fn prop<'a>(props: &'a [(String, PropValue)], name: &str) -> &'a PropValue {
    &props
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no property {name}"))
        .1
}

fn as_i64(v: &PropValue) -> Option<i64> {
    Some(match v {
        PropValue::I8(n) => i64::from(*n),
        PropValue::U8(n) => i64::from(*n),
        PropValue::I16(n) => i64::from(*n),
        PropValue::U16(n) => i64::from(*n),
        PropValue::I32(n) => i64::from(*n),
        PropValue::U32(n) => i64::from(*n),
        PropValue::I64(n) => *n,
        PropValue::U64(n) => i64::try_from(*n).ok()?,
        _ => return None,
    })
}

#[test]
fn every_fixture_round_trips() {
    for name in ALL {
        let s = load(&format!("{name}.aseprite"));
        let again = rp_io::read(&rp_io::write(&s).unwrap()).unwrap();
        assert_eq!(again, s, "{name}");
    }
}

#[test]
fn rgba_basic_contents() {
    let s = load("rgba_basic.aseprite");
    assert_eq!((s.width, s.height, s.color_mode), (8, 8, ColorMode::Rgba));
    let ink = layer(&s, "ink").id;
    assert_eq!(pixel_at(&s, ink, 0, 1, 1), Some(vec![255, 0, 0, 255]));
    assert_eq!(pixel_at(&s, ink, 0, 6, 6), Some(vec![0, 255, 0, 128]));
}

#[test]
fn grayscale_contents() {
    let s = load("grayscale.aseprite");
    assert_eq!(s.color_mode, ColorMode::Grayscale);
    assert_eq!(pixel_at(&s, s.layers[0].id, 0, 0, 0), Some(vec![200, 255]));
}

#[test]
fn indexed_contents() {
    let s = load("indexed.aseprite");
    assert_eq!(s.color_mode, ColorMode::Indexed);
    assert_eq!(s.transparent_index, 0);
    let rgba: Vec<[u8; 4]> = s.palette.entries.iter().take(4).map(|e| e.rgba).collect();
    assert_eq!(
        rgba,
        vec![
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            [255, 0, 0, 255],
            [0, 255, 0, 255]
        ]
    );
    assert_eq!(pixel_at(&s, s.layers[0].id, 0, 2, 2), Some(vec![2]));
}

#[test]
fn layers_tree_contents() {
    let s = load("layers_tree.aseprite");
    let names: Vec<&str> = s.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, vec!["bg", "body", "arm", "head", "eye"]);
    let (body, head) = (layer(&s, "body"), layer(&s, "head"));
    assert_eq!((body.kind, head.kind), (LayerKind::Group, LayerKind::Group));
    let arm = layer(&s, "arm");
    assert_eq!(arm.parent, Some(body.id));
    assert_eq!((arm.blend_mode, arm.opacity), (BlendMode::Multiply, 128));
    assert!(head.flags.contains(LayerFlags::COLLAPSED));
    let eye = layer(&s, "eye");
    assert_eq!(eye.parent, Some(head.id));
    assert!(!eye.flags.contains(LayerFlags::VISIBLE));
    assert_eq!(layer(&s, "bg").parent, None);
}

#[test]
fn animation_contents() {
    let s = load("animation.aseprite");
    let d: Vec<u16> = s.frames.iter().map(|f| f.duration_ms).collect();
    assert_eq!(d, vec![100, 150, 200, 250]);
    let a = layer(&s, "a").id;
    let first = s.cel(a, 0).unwrap();
    assert!(matches!(first.content, CelContent::Image(_)));
    assert_eq!((first.opacity, first.z_index), (64, 1));
    assert_eq!(
        s.cel(a, 1).unwrap().content,
        CelContent::Linked { frame: 0 }
    );
    assert_eq!(
        s.cel(a, 2).unwrap().content,
        CelContent::Linked { frame: 0 }
    );
    assert!(s.cel(a, 3).is_none());
    let expect = [
        ("fwd", 0, 1, AniDir::Forward, [255, 0, 0, 255]),
        ("rev", 0, 1, AniDir::Reverse, [0, 255, 0, 255]),
        ("pp", 0, 3, AniDir::PingPong, [0, 0, 255, 255]),
        ("ppr", 2, 3, AniDir::PingPongReverse, [255, 255, 0, 255]),
    ];
    for (name, from, to, dir, color) in expect {
        let t = tag(&s, name);
        assert_eq!(
            (t.from, t.to, t.direction, t.user_data.color),
            (from, to, dir, Some(color)),
            "{name}"
        );
    }
    assert_eq!(tag(&s, "rev").repeat, 3);
}

#[test]
fn slices_contents() {
    let s = load("slices.aseprite");
    let sl = &s.slices[0];
    assert_eq!(sl.name, "btn");
    assert_eq!(
        sl.flags & (Slice::NINE_PATCH | Slice::PIVOT),
        Slice::NINE_PATCH | Slice::PIVOT
    );
    assert_eq!(
        sl.keys[0],
        SliceKey {
            frame: 0,
            x: 1,
            y: 1,
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
}

#[test]
fn user_data_contents() {
    let s = load("user_data.aseprite");
    assert_eq!(s.user_data.text.as_deref(), Some("sprite note"));
    let l = &s.layers[0];
    assert_eq!(l.user_data.text.as_deref(), Some("layer note"));
    assert_eq!(l.user_data.color, Some([0, 0, 255, 255]));
    assert_eq!(
        s.cel(l.id, 0).unwrap().user_data.text.as_deref(),
        Some("cel note")
    );
    assert_eq!(tag(&s, "tag").user_data.color, Some([255, 255, 0, 255]));
    let slice = s.slices.iter().find(|x| x.name == "slice").unwrap();
    assert_eq!(slice.user_data.text.as_deref(), Some("slice note"));

    let user = &l
        .user_data
        .properties
        .iter()
        .find(|m| m.key == 0)
        .unwrap()
        .properties;
    assert_eq!(prop(user, "flag"), &PropValue::Bool(true));
    assert_eq!(as_i64(prop(user, "count")), Some(7));
    assert_eq!(prop(user, "ratio"), &PropValue::F64(1.5));
    assert_eq!(prop(user, "label"), &PropValue::String("hello".into()));
    assert_eq!(prop(user, "pos"), &PropValue::Point { x: 3, y: 4 });
    assert_eq!(
        prop(user, "size"),
        &PropValue::Size {
            width: 5,
            height: 6
        }
    );
    assert_eq!(
        prop(user, "rect"),
        &PropValue::Rect {
            x: 1,
            y: 2,
            width: 3,
            height: 4
        }
    );
    match prop(user, "nums") {
        PropValue::Vector { items, .. } => assert_eq!(
            items.iter().map(as_i64).collect::<Vec<_>>(),
            vec![Some(1), Some(2), Some(3)]
        ),
        other => panic!("nums: {other:?}"),
    }
    match prop(user, "mixed") {
        PropValue::Vector {
            element_type: 0,
            items,
        } => {
            assert_eq!(as_i64(&items[0]), Some(1));
            assert_eq!(items[1], PropValue::String("two".into()));
            assert_eq!(items[2], PropValue::F64(3.5));
        }
        other => panic!("mixed: {other:?}"),
    }
    match prop(user, "nested") {
        PropValue::Map(m) => {
            assert_eq!(as_i64(prop(m, "x")), Some(1));
            assert_eq!(as_i64(prop(m, "y")), Some(2));
        }
        other => panic!("nested: {other:?}"),
    }
    let uuid: [u8; 16] = [
        0x74, 0x34, 0x1b, 0x56, 0x4f, 0x7f, 0x4a, 0xb1, 0x94, 0x95, 0x58, 0xcf, 0x5b, 0xce, 0x0e,
        0x1c,
    ];
    assert_eq!(prop(user, "id"), &PropValue::Uuid(uuid));

    let ext = s
        .external_files
        .iter()
        .find(|e| e.name == "rusty/fixtures")
        .unwrap();
    assert_eq!(ext.kind, 2);
    let ext_props = &l
        .user_data
        .properties
        .iter()
        .find(|m| m.key == ext.id)
        .unwrap()
        .properties;
    assert_eq!(prop(ext_props, "enabled"), &PropValue::Bool(true));
    assert_eq!(
        prop(ext_props, "name"),
        &PropValue::String("fixtures".into())
    );
}

#[test]
fn tilemap_contents() {
    let s = load("tilemap.aseprite");
    let map = layer(&s, "map");
    assert_eq!(map.kind, LayerKind::Tilemap { tileset_index: 0 });
    let ts = &s.tilesets[0];
    assert_eq!(
        (ts.tile_width, ts.tile_height, ts.tile_count, ts.tiles.len()),
        (8, 8, 3, 3)
    );
    assert_eq!(
        ts.tile_user_data.get(1).and_then(|u| u.text.as_deref()),
        Some("grass")
    );
    match &s.cel(map.id, 0).unwrap().content {
        CelContent::Tilemap(t) => {
            assert_eq!((t.width, t.height, t.bits_per_tile), (2, 1, 32));
            assert_eq!(t.x_flip_mask, 0x8000_0000);
            assert_eq!(t.tiles, vec![0x8000_0002, 0]);
            assert_eq!(t.tiles[0] & t.tile_id_mask, 2);
        }
        other => panic!("expected tilemap, got {other:?}"),
    }
}

#[test]
fn uuid_layers_contents() {
    let s = load("uuid_layers.aseprite");
    assert!(s.flags.contains(HeaderFlags::LAYER_UUIDS));
    assert_eq!(s.layers.len(), 2);
    let (a, b) = (s.layers[0].uuid.unwrap(), s.layers[1].uuid.unwrap());
    assert_ne!(a, b);
}

#[test]
fn ratio_grid_contents() {
    let s = load("ratio_grid.aseprite");
    assert_eq!((s.pixel_width, s.pixel_height), (2, 1));
    assert_eq!(
        s.grid,
        Grid {
            x: 1,
            y: 2,
            width: 4,
            height: 4
        }
    );
}
