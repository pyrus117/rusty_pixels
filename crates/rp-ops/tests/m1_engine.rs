#![allow(clippy::unwrap_used, clippy::expect_used)]
//! M1 engine semantics: per-origin transactions with conflict detection,
//! the replay log, strict command JSON, registry flags, palette size, limits.
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
fn agent2() -> Origin {
    Origin::Agent {
        session: "s2".into(),
    }
}
fn engine() -> Engine {
    Engine::new(Sprite::new(8, 8, ColorMode::Rgba))
}
fn add_as(e: &mut Engine, name: &str, origin: Origin) -> LayerId {
    e.apply(
        Command::AddLayer(AddLayer {
            name: name.into(),
            above: None,
            uuid: None,
        }),
        origin,
    )
    .unwrap()
    .created_layer
    .unwrap()
}
fn paint_as(
    e: &mut Engine,
    layer: LayerId,
    x: i32,
    y: i32,
    value: PixelValue,
    origin: Origin,
) -> Result<Outcome, OpError> {
    e.apply(
        Command::SetPixels(SetPixels {
            layer,
            frame: 0,
            pixels: vec![PixelWrite { x, y, value }],
        }),
        origin,
    )
}
fn pixel(e: &Engine, layer: LayerId, x: i32, y: i32) -> Option<Vec<u8>> {
    let (_, cel) = e.sprite().resolve_cel(layer, 0)?;
    match &cel.content {
        CelContent::Image(img) => img
            .get((x - cel.x) as u32, (y - cel.y) as u32)
            .map(|p| p.to_vec()),
        _ => None,
    }
}

// ---- per-origin transactions ----

#[test]
fn each_origin_has_its_own_transaction() {
    let mut e = engine();
    let h = add_as(&mut e, "human", Origin::Human);
    let a = add_as(&mut e, "agent", agent());
    e.begin("draw", agent()).unwrap();
    assert!(matches!(
        e.begin("again", agent()),
        Err(OpError::TransactionOpen)
    ));
    e.begin("mine", agent2()).unwrap();
    e.begin("stroke", Origin::Human).unwrap();
    e.commit(&Origin::Human).unwrap(); // empty: adds nothing
    e.rollback(&agent2()).unwrap(); // empty: nothing to revert

    paint_as(&mut e, a, 0, 0, RED, agent()).unwrap();
    // The human's edit is not captured by the agent's open transaction.
    paint_as(&mut e, h, 1, 1, BLUE, Origin::Human).unwrap();
    let labels: Vec<String> = e.history().into_iter().map(|h| h.label).collect();
    assert_eq!(labels, vec!["New Layer", "Agent: New Layer", "Set Pixels"]);

    e.commit(&agent()).unwrap();
    let last = e.history().pop().unwrap();
    assert_eq!(
        last,
        HistoryEntry {
            label: "Agent: draw".into(),
            origin: agent()
        }
    );
    assert!(matches!(e.commit(&agent()), Err(OpError::NoTransaction)));
    assert!(matches!(e.rollback(&agent2()), Err(OpError::NoTransaction)));
}

#[test]
fn rollback_reverts_only_its_own_origin() {
    let mut e = engine();
    let h = add_as(&mut e, "human", Origin::Human);
    let a = add_as(&mut e, "agent", agent());
    e.begin("try", agent()).unwrap();
    paint_as(&mut e, a, 0, 0, RED, agent()).unwrap();
    paint_as(&mut e, h, 1, 1, BLUE, Origin::Human).unwrap();
    let rev = e.revision();
    e.rollback(&agent()).unwrap();
    assert_eq!(e.revision(), rev + 1);
    assert!(e.sprite().cel(a, 0).is_none(), "agent's cel reverted");
    assert_eq!(
        pixel(&e, h, 1, 1),
        Some(vec![0, 0, 255, 255]),
        "human's edit kept"
    );
    assert_eq!(e.history().len(), 3, "two layers and the human's paint");
}

#[test]
fn rollback_conflicts_when_someone_else_touched_the_same_cel() {
    let mut e = engine();
    let a = add_as(&mut e, "a", Origin::Human);
    e.begin("try", agent()).unwrap();
    paint_as(&mut e, a, 0, 0, RED, agent()).unwrap();
    paint_as(&mut e, a, 1, 1, BLUE, Origin::Human).unwrap();
    let before = e.sprite().clone();
    let rev = e.revision();
    assert!(matches!(e.rollback(&agent()), Err(OpError::Conflict(_))));
    assert_eq!(e.sprite(), &before, "a failed rollback changes nothing");
    assert_eq!(e.revision(), rev);
    // The transaction is still open and can be committed instead.
    e.commit(&agent()).unwrap();
    assert_eq!(e.history().last().unwrap().label, "Agent: try");
}

#[test]
fn rollback_of_a_new_layer_depends_on_who_used_it() {
    // Human paints a different layer: the agent can still roll back its new layer.
    let mut e = engine();
    let h = add_as(&mut e, "h", Origin::Human);
    e.begin("layer", agent()).unwrap();
    let x = add_as(&mut e, "x", agent());
    paint_as(&mut e, x, 2, 2, RED, agent()).unwrap();
    paint_as(&mut e, h, 0, 0, BLUE, Origin::Human).unwrap();
    e.rollback(&agent()).unwrap();
    assert!(e.sprite().layer(x).is_none());
    assert!(e.sprite().cel(x, 0).is_none());
    assert_eq!(pixel(&e, h, 0, 0), Some(vec![0, 0, 255, 255]));

    // Human paints on the agent's new layer: rolling back would orphan that cel.
    let mut e = engine();
    e.begin("layer", agent()).unwrap();
    let x = add_as(&mut e, "x", agent());
    paint_as(&mut e, x, 0, 0, BLUE, Origin::Human).unwrap();
    assert!(matches!(e.rollback(&agent()), Err(OpError::Conflict(_))));
    assert!(e.sprite().layer(x).is_some());
}

#[test]
fn another_agents_layer_insert_blocks_rollback_of_mine() {
    // Two layer inserts both write the layer order; reverting the older one
    // would remove the wrong index.
    let mut e = engine();
    e.begin("mine", agent()).unwrap();
    add_as(&mut e, "a1", agent());
    add_as(&mut e, "a2", agent2());
    assert!(matches!(e.rollback(&agent()), Err(OpError::Conflict(_))));
}

#[test]
fn undo_works_while_a_transaction_is_open() {
    let mut e = engine();
    let h = add_as(&mut e, "h", Origin::Human);
    let a = add_as(&mut e, "a", Origin::Human);
    paint_as(&mut e, h, 0, 0, RED, Origin::Human).unwrap();
    e.begin("draw", agent()).unwrap();
    paint_as(&mut e, a, 3, 3, BLUE, agent()).unwrap();
    assert_eq!(e.undo().unwrap(), Some("Set Pixels".into()));
    assert!(e.sprite().cel(h, 0).is_none());
    assert_eq!(
        pixel(&e, a, 3, 3),
        Some(vec![0, 0, 255, 255]),
        "agent's work untouched"
    );
    assert_eq!(e.redo().unwrap(), Some("Set Pixels".into()));
    e.commit(&agent()).unwrap();
    assert_eq!(e.history().len(), 4);
}

#[test]
fn undo_conflicts_with_a_later_open_transaction_change() {
    let mut e = engine();
    let h = add_as(&mut e, "h", Origin::Human);
    paint_as(&mut e, h, 0, 0, RED, Origin::Human).unwrap();
    e.begin("draw", agent()).unwrap();
    paint_as(&mut e, h, 1, 1, BLUE, agent()).unwrap();
    let before = e.sprite().clone();
    let history = e.history();
    assert!(matches!(e.undo(), Err(OpError::Conflict(_))));
    assert_eq!(e.sprite(), &before);
    assert_eq!(e.history(), history, "the entry stays on the stack");
    // Once the agent commits, its entry is on top and undo proceeds normally.
    e.commit(&agent()).unwrap();
    assert_eq!(e.undo().unwrap(), Some("Agent: draw".into()));
    assert_eq!(e.undo().unwrap(), Some("Set Pixels".into()));
    assert!(e.sprite().cel(h, 0).is_none());
}

#[test]
fn redo_conflicts_with_a_change_made_after_the_undo() {
    let mut e = engine();
    let h = add_as(&mut e, "h", Origin::Human);
    paint_as(&mut e, h, 0, 0, RED, Origin::Human).unwrap();
    e.undo().unwrap();
    e.begin("draw", agent()).unwrap();
    paint_as(&mut e, h, 1, 1, BLUE, agent()).unwrap();
    let before = e.sprite().clone();
    assert!(matches!(e.redo(), Err(OpError::Conflict(_))));
    assert_eq!(e.sprite(), &before);
    e.rollback(&agent()).unwrap();
    assert_eq!(e.redo().unwrap(), Some("Set Pixels".into()));
    assert_eq!(pixel(&e, h, 0, 0), Some(vec![255, 0, 0, 255]));
}

// ---- replay log ----

#[test]
fn log_records_every_engine_event() {
    let mut e = engine();
    let h = add_as(&mut e, "h", Origin::Human);
    e.begin("draw", agent()).unwrap();
    paint_as(&mut e, h, 0, 0, RED, agent()).unwrap();
    e.commit(&agent()).unwrap();
    e.undo().unwrap();
    e.redo().unwrap();
    e.begin("oops", agent()).unwrap();
    paint_as(&mut e, h, 1, 0, BLUE, agent()).unwrap();
    e.rollback(&agent()).unwrap();
    let _ = e.undo(); // ok
    let _ = e.commit(&agent()); // fails: not logged

    let kinds: Vec<&str> = e
        .log()
        .iter()
        .map(|l| match l {
            LogEntry::Apply { .. } => "apply",
            LogEntry::Undo { .. } => "undo",
            LogEntry::Redo { .. } => "redo",
            LogEntry::Begin { .. } => "begin",
            LogEntry::Commit { .. } => "commit",
            LogEntry::Rollback { .. } => "rollback",
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "apply", "begin", "apply", "commit", "undo", "redo", "begin", "apply", "rollback",
            "undo"
        ]
    );
    assert!(matches!(
        &e.log()[1],
        LogEntry::Begin { origin, label } if *origin == agent() && label == "draw"
    ));
}

#[test]
fn log_serialises_and_replays_to_the_same_state() {
    let initial = Sprite::new(8, 8, ColorMode::Rgba);
    let mut e = Engine::new(initial.clone());
    let h = add_as(&mut e, "h", Origin::Human);
    let a = add_as(&mut e, "a", agent());
    paint_as(&mut e, h, 0, 0, RED, Origin::Human).unwrap();
    e.begin("draw", agent()).unwrap();
    paint_as(&mut e, a, 4, 4, BLUE, agent()).unwrap();
    paint_as(&mut e, h, 7, 7, BLUE, Origin::Human).unwrap();
    e.commit(&agent()).unwrap();
    e.undo().unwrap();
    e.begin("gone", agent2()).unwrap();
    paint_as(&mut e, a, 5, 5, RED, agent2()).unwrap();
    e.rollback(&agent2()).unwrap();

    let json = serde_json::to_string(e.log()).unwrap();
    let log: Vec<LogEntry> = serde_json::from_str(&json).unwrap();
    assert_eq!(log, e.log());
    let r = Engine::replay(initial, &log).unwrap();
    assert_eq!(r.sprite(), e.sprite());
    assert_eq!(r.revision(), e.revision());
    assert_eq!(r.history(), e.history());
    assert_eq!(r.log(), e.log());
}

#[test]
fn origin_json_shape() {
    assert_eq!(
        serde_json::to_value(Origin::Human).unwrap(),
        serde_json::json!({"kind": "human"})
    );
    assert_eq!(
        serde_json::to_value(agent()).unwrap(),
        serde_json::json!({"kind": "agent", "session": "s1"})
    );
}

// ---- deterministic UUIDs ----

#[test]
fn generated_uuids_are_logged_so_replay_is_deterministic() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.flags = HeaderFlags(HeaderFlags::LAYER_OPACITY_VALID | HeaderFlags::LAYER_UUIDS);
    let mut e = Engine::new(s.clone());
    let a = add_as(&mut e, "a", Origin::Human);
    let made = e.sprite().layer(a).unwrap().uuid.unwrap();
    match &e.log()[0] {
        LogEntry::Apply {
            command: Command::AddLayer(c),
            ..
        } => assert_eq!(c.uuid, Some(made)),
        other => panic!("{other:?}"),
    }
    let r = Engine::replay(s.clone(), e.log()).unwrap();
    assert_eq!(r.sprite().layer(a).unwrap().uuid, Some(made));

    let given = [7u8; 16];
    let mut e = Engine::new(s);
    let b = e
        .apply(
            Command::AddLayer(AddLayer {
                name: "b".into(),
                above: None,
                uuid: Some(given),
            }),
            agent(),
        )
        .unwrap()
        .created_layer
        .unwrap();
    assert_eq!(e.sprite().layer(b).unwrap().uuid, Some(given));

    let mut plain = engine();
    assert!(matches!(
        plain.apply(
            Command::AddLayer(AddLayer {
                name: "c".into(),
                above: None,
                uuid: Some(given),
            }),
            Origin::Human
        ),
        Err(OpError::InvalidArgument(_))
    ));
}

// ---- strict JSON and registry flags ----

#[test]
fn command_json_rejects_unknown_fields() {
    let ok: Command = serde_json::from_str(r#"{"op":"add_layer","name":"a"}"#).unwrap();
    assert!(matches!(ok, Command::AddLayer(AddLayer { uuid: None, .. })));
    assert!(serde_json::from_str::<Command>(r#"{"op":"add_layer","name":"a","bogus":1}"#).is_err());
    assert!(
        serde_json::from_str::<Command>(r#"{"op":"add_layer","name":"a","abvoe":3}"#).is_err(),
        "a misspelt optional field is an error, not a silent default"
    );
    assert!(
        serde_json::from_str::<Command>(r#"{"op":"remove_layer","layer":1,"recursive":true}"#)
            .is_err()
    );
    let r: Command = serde_json::from_str(r#"{"op":"resize_palette","len":4}"#).unwrap();
    assert_eq!(r, Command::ResizePalette(ResizePalette { len: 4 }));
}

#[test]
fn permitted_checks_agent_exposure() {
    let hidden = OpSpec {
        id: "secret",
        title: "Secret",
        doc: "",
        menu_path: None,
        default_shortcut: None,
        undoable: true,
        agent_exposed: false,
    };
    assert_eq!(
        permitted(&hidden, &agent()),
        Err(OpError::NotAgentExposed("secret"))
    );
    assert_eq!(permitted(&hidden, &Origin::Human), Ok(()));
    for s in registry() {
        assert_eq!(permitted(s, &agent()), Ok(()), "{} is agent-exposed", s.id);
    }
}

// ---- palette size and indexed checks ----

#[test]
fn resize_palette_grows_shrinks_and_undoes() {
    let mut e = engine();
    let spec = spec(CommandKind::ResizePalette);
    assert_eq!(
        (spec.id, spec.title, spec.undoable, spec.agent_exposed),
        ("resize_palette", "Palette Size", true, true)
    );
    e.apply(Command::ResizePalette(ResizePalette { len: 3 }), agent())
        .unwrap();
    assert_eq!(
        e.sprite().palette.entries,
        vec![
            PaletteEntry {
                rgba: [0, 0, 0, 255],
                name: None
            };
            3
        ]
    );
    e.apply(
        Command::SetPaletteColor(SetPaletteColor {
            index: 2,
            rgba: [1, 2, 3, 4],
        }),
        agent(),
    )
    .unwrap();
    e.apply(Command::ResizePalette(ResizePalette { len: 1 }), agent())
        .unwrap();
    assert_eq!(e.sprite().palette.entries.len(), 1);
    e.undo().unwrap();
    assert_eq!(
        e.sprite().palette.entries[2].rgba,
        [1, 2, 3, 4],
        "undo restores dropped entries"
    );
    for bad in [0, 257] {
        assert!(matches!(
            e.apply(Command::ResizePalette(ResizePalette { len: bad }), agent()),
            Err(OpError::InvalidArgument(_))
        ));
    }
}

#[test]
fn indexed_pixels_must_be_in_the_palette() {
    let mut s = Sprite::new(4, 4, ColorMode::Indexed);
    s.palette.entries = vec![
        PaletteEntry {
            rgba: [0, 0, 0, 255],
            name: None
        };
        2
    ];
    s.transparent_index = 9;
    let mut e = Engine::new(s);
    let l = add_as(&mut e, "l", Origin::Human);
    let idx = |i| PixelValue::Indexed { index: i };
    paint_as(&mut e, l, 0, 0, idx(1), Origin::Human).unwrap();
    assert!(matches!(
        paint_as(&mut e, l, 0, 0, idx(2), Origin::Human),
        Err(OpError::PaletteIndex { index: 2, len: 2 })
    ));
    paint_as(&mut e, l, 1, 0, idx(9), Origin::Human).expect("transparent index is always allowed");
}

// ---- format limits and fallible Image::new ----

#[test]
fn format_limits_are_enforced() {
    let mut e = engine();
    let long = "x".repeat(65536);
    assert!(matches!(
        e.apply(
            Command::AddLayer(AddLayer {
                name: long.clone(),
                above: None,
                uuid: None
            }),
            Origin::Human
        ),
        Err(OpError::InvalidArgument(_))
    ));
    let a = add_as(&mut e, "a", Origin::Human);
    assert!(matches!(
        e.apply(
            Command::RenameLayer(RenameLayer {
                layer: a,
                name: long
            }),
            Origin::Human
        ),
        Err(OpError::InvalidArgument(_))
    ));
    add_as(&mut e, &"y".repeat(65535), Origin::Human);

    let mut s = Sprite::new(1, 1, ColorMode::Rgba);
    s.frames = vec![Frame { duration_ms: 100 }; 65535];
    let mut e = Engine::new(s);
    assert!(matches!(
        e.apply(
            Command::AddFrame(AddFrame { duration_ms: 10 }),
            Origin::Human
        ),
        Err(OpError::InvalidArgument(_))
    ));
}

#[test]
fn image_new_is_fallible() {
    assert!(matches!(
        Image::new(u32::MAX, u32::MAX, ColorMode::Rgba),
        Err(CoreError::TooLarge { .. })
    ));
    assert!(matches!(
        Image::new(65536, 65536, ColorMode::Rgba),
        Err(CoreError::TooLarge { .. })
    ));
    let img = Image::new(2, 2, ColorMode::Grayscale).unwrap();
    assert_eq!(img.data().len(), 8);
}
