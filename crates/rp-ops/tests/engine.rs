#![allow(clippy::unwrap_used, clippy::expect_used)]
use rp_core::*;
use rp_ops::*;

const RED: PixelValue = PixelValue::Rgba {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};
const BLUE: PixelValue = PixelValue::Rgba {
    r: 0,
    g: 0,
    b: 255,
    a: 255,
};
fn agent() -> Origin {
    Origin::Agent {
        session: "s1".into(),
    }
}
fn engine() -> Engine {
    Engine::new(Sprite::new(8, 8, ColorMode::Rgba))
}
fn add(e: &mut Engine, name: &str, above: Option<LayerId>) -> LayerId {
    e.apply(
        Command::AddLayer(AddLayer {
            name: name.into(),
            above,
        }),
        Origin::Human,
    )
    .unwrap()
    .created_layer
    .unwrap()
}
fn px(
    e: &mut Engine,
    layer: LayerId,
    frame: u32,
    pts: &[(i32, i32, PixelValue)],
) -> Result<Outcome, OpError> {
    e.apply(
        Command::SetPixels(SetPixels {
            layer,
            frame,
            pixels: pts
                .iter()
                .map(|&(x, y, value)| PixelWrite { x, y, value })
                .collect(),
        }),
        Origin::Human,
    )
}
fn image_of(e: &Engine, layer: LayerId, frame: u32) -> (i32, i32, Image) {
    let (_, cel) = e.sprite().resolve_cel(layer, frame).unwrap();
    match &cel.content {
        CelContent::Image(i) => (cel.x, cel.y, i.clone()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn add_layer_returns_id_and_bumps_revision() {
    let mut e = engine();
    let a = add(&mut e, "a", None);
    assert_eq!(e.revision(), 1);
    let l = e.sprite().layer(a).unwrap();
    assert_eq!(
        (l.name.as_str(), l.kind, l.opacity, l.parent),
        ("a", LayerKind::Image, 255, None)
    );
    assert!(l.flags.contains(LayerFlags::VISIBLE) && l.flags.contains(LayerFlags::EDITABLE));
    assert_eq!(l.uuid, None);
}

#[test]
fn set_pixels_creates_then_grows_cel() {
    let mut e = engine();
    let a = add(&mut e, "a", None);
    px(&mut e, a, 0, &[(2, 3, RED)]).unwrap();
    let (x, y, img) = image_of(&e, a, 0);
    assert_eq!((x, y, img.width(), img.height()), (2, 3, 1, 1));
    px(&mut e, a, 0, &[(5, 1, BLUE)]).unwrap();
    let (x, y, img) = image_of(&e, a, 0);
    assert_eq!((x, y, img.width(), img.height()), (2, 1, 4, 3));
    assert_eq!(
        img.get(0, 2),
        Some(&[255u8, 0, 0, 255][..]),
        "old pixel kept at new offset"
    );
    assert_eq!(img.get(3, 0), Some(&[0u8, 0, 255, 255][..]));
    assert_eq!(
        img.get(1, 1),
        Some(&[0u8, 0, 0, 0][..]),
        "new area is transparent"
    );
}

#[test]
fn failed_commands_change_nothing() {
    let mut e = engine();
    let a = add(&mut e, "a", None);
    px(&mut e, a, 0, &[(0, 0, RED)]).unwrap();
    let before = e.sprite().clone();
    let rev = e.revision();
    let cases: Vec<(Result<Outcome, OpError>, &str)> = vec![
        (
            px(&mut e, a, 0, &[(1, 1, RED), (8, 0, RED)]),
            "out of canvas",
        ),
        (
            px(&mut e, a, 0, &[(1, 1, PixelValue::Indexed { index: 1 })]),
            "mode",
        ),
        (px(&mut e, a, 5, &[(1, 1, RED)]), "frame"),
        (px(&mut e, LayerId(99), 0, &[(1, 1, RED)]), "layer"),
        (px(&mut e, a, 0, &[]), "empty"),
        (
            e.apply(
                Command::RenameLayer(RenameLayer {
                    layer: a,
                    name: String::new(),
                }),
                Origin::Human,
            ),
            "empty name",
        ),
        (
            e.apply(
                Command::SetFrameDuration(SetFrameDuration {
                    frame: 0,
                    duration_ms: 0,
                }),
                Origin::Human,
            ),
            "zero duration",
        ),
        (
            e.apply(
                Command::SetPaletteColor(SetPaletteColor {
                    index: 0,
                    rgba: [0; 4],
                }),
                Origin::Human,
            ),
            "palette",
        ),
    ];
    for (r, what) in cases {
        assert!(r.is_err(), "{what} should fail");
        assert_eq!(e.sprite(), &before, "{what} mutated the sprite");
        assert_eq!(e.revision(), rev, "{what} bumped the revision");
    }
    assert!(matches!(
        px(&mut e, a, 0, &[(9, 9, RED)]),
        Err(OpError::OutOfCanvas {
            x: 9,
            y: 9,
            width: 8,
            height: 8
        })
    ));
}

#[test]
fn set_pixels_rejects_group_layers() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.layers.push(Layer {
        id: LayerId(0),
        parent: None,
        name: "g".into(),
        kind: LayerKind::Group,
        flags: LayerFlags(3),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    });
    let mut e = Engine::new(s);
    assert_eq!(
        px(&mut e, LayerId(0), 0, &[(0, 0, RED)]),
        Err(OpError::NotAnImageLayer(LayerId(0)))
    );
}

#[test]
fn writing_through_a_linked_cel_edits_the_source() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.frames.push(Frame { duration_ms: 100 });
    s.layers.push(Layer {
        id: LayerId(0),
        parent: None,
        name: "a".into(),
        kind: LayerKind::Image,
        flags: LayerFlags(3),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    });
    let base = Cel {
        x: 0,
        y: 0,
        opacity: 255,
        z_index: 0,
        content: CelContent::Image(Image::new(4, 4, ColorMode::Rgba)),
        extra: None,
        user_data: UserData::default(),
    };
    s.cels.insert((LayerId(0), 0), base.clone());
    s.cels.insert(
        (LayerId(0), 1),
        Cel {
            content: CelContent::Linked { frame: 0 },
            ..base
        },
    );
    let mut e = Engine::new(s);
    px(&mut e, LayerId(0), 1, &[(1, 1, RED)]).unwrap();
    assert_eq!(
        e.sprite().cel(LayerId(0), 1).unwrap().content,
        CelContent::Linked { frame: 0 }
    );
    assert_eq!(
        image_of(&e, LayerId(0), 0).2.get(1, 1),
        Some(&[255u8, 0, 0, 255][..])
    );
}

#[test]
fn undo_redo_restore_exact_states() {
    let mut e = engine();
    let s0 = e.sprite().clone();
    let a = add(&mut e, "a", None);
    let s1 = e.sprite().clone();
    px(&mut e, a, 0, &[(1, 1, RED)]).unwrap();
    let s2 = e.sprite().clone();
    assert_eq!(e.undo().unwrap(), Some("Set Pixels".into()));
    assert_eq!(e.sprite(), &s1);
    assert_eq!(e.undo().unwrap(), Some("New Layer".into()));
    assert_eq!(e.sprite(), &s0);
    assert_eq!(e.undo().unwrap(), None);
    assert_eq!(e.redo().unwrap(), Some("New Layer".into()));
    assert_eq!(e.redo().unwrap(), Some("Set Pixels".into()));
    assert_eq!(e.sprite(), &s2);
    assert_eq!(e.redo().unwrap(), None);
    assert_eq!(e.revision(), 6);
}

#[test]
fn new_command_clears_redo() {
    let mut e = engine();
    add(&mut e, "a", None);
    e.undo().unwrap();
    add(&mut e, "b", None);
    assert_eq!(e.redo().unwrap(), None);
}

#[test]
fn transactions_group_and_label_with_agent_prefix() {
    let mut e = engine();
    e.begin("outline", agent()).unwrap();
    let a = e
        .apply(
            Command::AddLayer(AddLayer {
                name: "a".into(),
                above: None,
            }),
            agent(),
        )
        .unwrap()
        .created_layer
        .unwrap();
    e.apply(
        Command::SetPixels(SetPixels {
            layer: a,
            frame: 0,
            pixels: vec![PixelWrite {
                x: 0,
                y: 0,
                value: RED,
            }],
        }),
        agent(),
    )
    .unwrap();
    assert!(matches!(e.undo(), Err(OpError::TransactionOpen)));
    assert!(matches!(
        e.begin("x", Origin::Human),
        Err(OpError::TransactionOpen)
    ));
    e.commit().unwrap();
    assert_eq!(
        e.history(),
        vec![HistoryEntry {
            label: "Agent: outline".into(),
            origin: agent()
        }]
    );
    assert_eq!(e.undo().unwrap(), Some("Agent: outline".into()));
    assert!(e.sprite().layers.is_empty());
}

#[test]
fn single_agent_command_is_labelled() {
    let mut e = engine();
    e.apply(
        Command::AddLayer(AddLayer {
            name: "a".into(),
            above: None,
        }),
        agent(),
    )
    .unwrap();
    assert_eq!(e.history()[0].label, "Agent: New Layer");
}

#[test]
fn rollback_restores_and_leaves_no_history() {
    let mut e = engine();
    let a = add(&mut e, "a", None);
    let before = e.sprite().clone();
    let rev = e.revision();
    e.begin("try", Origin::Human).unwrap();
    px(&mut e, a, 0, &[(3, 3, RED)]).unwrap();
    e.rollback().unwrap();
    assert_eq!(e.sprite(), &before);
    assert_eq!(e.history().len(), 1);
    assert_eq!(e.revision(), rev + 2, "apply +1, rollback +1");
    assert!(matches!(e.commit(), Err(OpError::NoTransaction)));
    assert!(matches!(e.rollback(), Err(OpError::NoTransaction)));
    e.begin("empty", Origin::Human).unwrap();
    e.commit().unwrap();
    assert_eq!(e.history().len(), 1, "empty commit adds nothing");
}

#[test]
fn stale_revision_is_rejected() {
    let mut e = engine();
    let seen = e.revision();
    add(&mut e, "human", None);
    let r = e.apply_at(
        Command::AddLayer(AddLayer {
            name: "agent".into(),
            above: None,
        }),
        agent(),
        seen,
    );
    assert_eq!(
        r,
        Err(OpError::StaleRevision {
            expected: seen,
            actual: seen + 1
        })
    );
    assert_eq!(e.sprite().layers.len(), 1);
    assert!(
        e.apply_at(
            Command::AddLayer(AddLayer {
                name: "ok".into(),
                above: None
            }),
            agent(),
            seen + 1
        )
        .is_ok()
    );
}

#[test]
fn add_layer_above_goes_after_subtree_with_same_parent() {
    // g { child }, then add "x" above g -> after child, root level.
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    let mk = |id: u32, parent: Option<u32>, kind: LayerKind| Layer {
        id: LayerId(id),
        parent: parent.map(LayerId),
        name: format!("{id}"),
        kind,
        flags: LayerFlags(3),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    };
    s.layers = vec![
        mk(0, None, LayerKind::Group),
        mk(1, Some(0), LayerKind::Image),
        mk(2, None, LayerKind::Image),
    ];
    let mut e = Engine::new(s);
    let x = add(&mut e, "x", Some(LayerId(0)));
    assert_eq!(x, LayerId(3), "next id is max + 1");
    let order: Vec<u32> = e.sprite().layers.iter().map(|l| l.id.0).collect();
    assert_eq!(order, vec![0, 1, 3, 2]);
    assert_eq!(e.sprite().layer(x).unwrap().parent, None);
    let y = add(&mut e, "y", Some(LayerId(1)));
    assert_eq!(e.sprite().layer(y).unwrap().parent, Some(LayerId(0)));
    assert_eq!(e.sprite().layer_index(y), Some(2));
}

#[test]
fn removing_a_group_takes_descendants_and_cels_and_undo_restores() {
    let mut e = engine();
    let a = add(&mut e, "a", None);
    px(&mut e, a, 0, &[(0, 0, RED)]).unwrap();
    let mut s = e.sprite().clone();
    s.layers.insert(
        0,
        Layer {
            id: LayerId(50),
            parent: None,
            name: "g".into(),
            kind: LayerKind::Group,
            flags: LayerFlags(3),
            blend_mode: BlendMode::Normal,
            opacity: 255,
            uuid: None,
            user_data: UserData::default(),
        },
    );
    s.layers[1].parent = Some(LayerId(50));
    let mut e = Engine::new(s);
    let before = e.sprite().clone();
    e.apply(
        Command::RemoveLayer(RemoveLayer { layer: LayerId(50) }),
        Origin::Human,
    )
    .unwrap();
    assert!(e.sprite().layers.is_empty() && e.sprite().cels.is_empty());
    e.undo().unwrap();
    assert_eq!(e.sprite(), &before);
}

#[test]
fn layer_ids_are_never_reused() {
    let mut e = engine();
    let a = add(&mut e, "a", None);
    e.undo().unwrap();
    let b = add(&mut e, "b", None);
    assert_ne!(a, b);
}

#[test]
fn uuid_layers_get_uuids() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.flags = HeaderFlags(HeaderFlags::LAYER_OPACITY_VALID | HeaderFlags::LAYER_UUIDS);
    let mut e = Engine::new(s);
    let a = add(&mut e, "a", None);
    assert!(e.sprite().layer(a).unwrap().uuid.is_some());
}

#[test]
fn frames_palette_and_rename() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.palette.entries = vec![PaletteEntry {
        rgba: [0, 0, 0, 255],
        name: None,
    }];
    let mut e = Engine::new(s);
    e.apply(
        Command::AddFrame(AddFrame { duration_ms: 50 }),
        Origin::Human,
    )
    .unwrap();
    assert_eq!(
        e.sprite().frames,
        vec![Frame { duration_ms: 100 }, Frame { duration_ms: 50 }]
    );
    e.apply(
        Command::SetFrameDuration(SetFrameDuration {
            frame: 1,
            duration_ms: 70,
        }),
        Origin::Human,
    )
    .unwrap();
    assert_eq!(e.sprite().frames[1].duration_ms, 70);
    assert!(matches!(
        e.apply(
            Command::SetFrameDuration(SetFrameDuration {
                frame: 9,
                duration_ms: 70
            }),
            Origin::Human
        ),
        Err(OpError::UnknownFrame(9))
    ));
    e.apply(
        Command::SetPaletteColor(SetPaletteColor {
            index: 0,
            rgba: [1, 2, 3, 4],
        }),
        Origin::Human,
    )
    .unwrap();
    assert_eq!(e.sprite().palette.entries[0].rgba, [1, 2, 3, 4]);
    let a = add(&mut e, "a", None);
    e.apply(
        Command::RenameLayer(RenameLayer {
            layer: a,
            name: "ink".into(),
        }),
        Origin::Human,
    )
    .unwrap();
    assert_eq!(e.sprite().layer(a).unwrap().name, "ink");
    for _ in 0..5 {
        e.undo().unwrap();
    }
    assert_eq!(e.sprite().frames.len(), 1);
    assert_eq!(e.sprite().palette.entries[0].rgba, [0, 0, 0, 255]);
}

#[test]
fn log_records_every_applied_command() {
    let mut e = engine();
    add(&mut e, "a", None);
    let _ = px(&mut e, LayerId(77), 0, &[(0, 0, RED)]); // fails: not logged
    e.apply(Command::AddFrame(AddFrame { duration_ms: 10 }), agent())
        .unwrap();
    let log = e.log();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].origin, Origin::Human);
    assert_eq!(log[1].origin, agent());
    assert_eq!(log[1].revision, 2);
    assert!(matches!(log[1].command, Command::AddFrame(_)));
}
