#![allow(clippy::unwrap_used, clippy::expect_used)]
use rp_ops::*;
use std::collections::HashSet;
use strum::IntoEnumIterator;

#[test]
fn every_command_kind_has_exactly_one_spec() {
    for kind in CommandKind::iter() {
        let name: &'static str = kind.into();
        let matches: Vec<_> = registry().iter().filter(|s| s.id == name).collect();
        assert_eq!(matches.len(), 1, "{name}");
        assert_eq!(spec(kind).id, name);
    }
    assert_eq!(
        registry().len(),
        CommandKind::iter().count(),
        "no orphan specs"
    );
}

#[test]
fn ids_are_unique_snake_case() {
    let mut seen = HashSet::new();
    for s in registry() {
        assert!(seen.insert(s.id), "duplicate {}", s.id);
        assert!(
            s.id.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{}",
            s.id
        );
        assert!(!s.title.is_empty() && !s.doc.is_empty(), "{}", s.id);
    }
}

#[test]
fn agent_exposed_ops_have_object_schemas_without_op_field() {
    for kind in CommandKind::iter() {
        if !spec(kind).agent_exposed {
            continue;
        }
        let schema = params_schema(kind);
        assert_eq!(schema["type"], "object", "{}", spec(kind).id);
        assert!(
            schema["properties"].get("op").is_none(),
            "{}",
            spec(kind).id
        );
    }
    let sp = params_schema(CommandKind::SetPixels);
    for field in ["layer", "frame", "pixels"] {
        assert!(sp["properties"].get(field).is_some(), "set_pixels.{field}");
    }
}

#[test]
fn commands_round_trip_through_json_with_op_tag() {
    let v = serde_json::json!({
        "op": "set_pixels", "layer": 0, "frame": 0,
        "pixels": [{"x": 1, "y": 2, "value": {"mode": "rgba", "r": 255, "g": 0, "b": 0, "a": 255}}]
    });
    let cmd: Command = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(cmd.kind(), CommandKind::SetPixels);
    assert_eq!(serde_json::to_value(&cmd).unwrap(), v);
    let add: Command =
        serde_json::from_value(serde_json::json!({"op": "add_layer", "name": "ink"})).unwrap();
    assert_eq!(
        add,
        Command::AddLayer(AddLayer {
            name: "ink".into(),
            above: None,
            uuid: None
        })
    );
    assert!(serde_json::from_value::<Command>(serde_json::json!({"op": "nope"})).is_err());
}

#[test]
fn shortcuts_are_unique() {
    let mut seen = HashSet::new();
    for s in registry() {
        if let Some(k) = s.default_shortcut {
            assert!(seen.insert(k), "{k} bound twice");
        }
    }
}
