//! Aseprite metadata comparison for the corpus round-trip tool.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::aseprite::{self, AsepriteRun};

/// The filename-independent metadata that must survive the round trip.
/// Aseprite's sheet JSON also carries path-dependent fields — `meta.image`
/// and `frames[].filename` come from the paths we pass — so only these four
/// values are taken from it.
pub struct Meta {
    layers: Value,
    frame_tags: Value,
    slices: Value,
    durations: Vec<u64>,
}

/// Asks Aseprite for one file's metadata, written to `<out>/meta/<n>-<tag>.json`
/// beside the sheet PNG that Aseprite needs as its companion. `--all-layers`
/// is not optional here: without it Aseprite omits hidden layers from the
/// layer list, so their entries would go un-compared.
pub fn export(
    aseprite: &Path,
    out: &Path,
    n: usize,
    tag: &str,
    file: &Path,
) -> Result<AsepriteRun> {
    let dir = out.join("meta");
    let sheet = dir.join(format!("{n}-{tag}.png"));
    let data = dir.join(format!("{n}-{tag}.json"));
    let output = Command::new(aseprite)
        .arg("-b")
        .arg("--all-layers")
        .arg("--list-layers")
        .arg("--list-tags")
        .arg("--list-slices")
        .arg(file)
        .arg("--sheet")
        .arg(&sheet)
        .arg("--data")
        .arg(&data)
        .arg("--format")
        .arg("json-array")
        .output()
        .with_context(|| format!("running {}", aseprite.display()))?;
    Ok(aseprite::classify(&output))
}

/// Reads and parses one exported metadata document.
pub fn load(path: &Path) -> Result<Meta> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse_meta(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Parses one Aseprite `json-array` sheet document into the compared fields.
pub fn parse_meta(text: &str) -> Result<Meta> {
    let doc: Value = serde_json::from_str(text).context("parsing metadata JSON")?;
    let meta = doc
        .get("meta")
        .context("metadata JSON has no meta object")?;
    let layers = meta
        .get("layers")
        .context("metadata JSON has no meta.layers")?
        .clone();
    let frame_tags = meta
        .get("frameTags")
        .context("metadata JSON has no meta.frameTags")?
        .clone();
    let slices = meta
        .get("slices")
        .context("metadata JSON has no meta.slices")?
        .clone();
    let frames = doc
        .get("frames")
        .and_then(Value::as_array)
        .context("metadata JSON has no frames array")?;
    let mut durations = Vec::with_capacity(frames.len());
    for frame in frames {
        let duration = frame
            .get("duration")
            .and_then(Value::as_u64)
            .context("metadata JSON frame has no duration")?;
        durations.push(duration);
    }
    Ok(Meta {
        layers,
        frame_tags,
        slices,
        durations,
    })
}

/// Names the first compared key where the two documents disagree. The order
/// is fixed, so one report names one key.
pub fn first_meta_diff(a: &Meta, b: &Meta) -> Option<&'static str> {
    let diffs = [
        ("meta.layers", a.layers != b.layers),
        ("meta.frameTags", a.frame_tags != b.frame_tags),
        ("meta.slices", a.slices != b.slices),
        ("durations", a.durations != b.durations),
    ];
    diffs
        .into_iter()
        .find(|(_, differ)| *differ)
        .map(|(name, _)| name)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// One sheet document. The filename arguments stand in for the
    /// path-dependent fields; `opacity` for a compared one.
    fn doc(image: &str, frame_name: &str, opacity: u32) -> String {
        format!(
            r#"{{ "frames": [{{ "filename": "{frame_name}", "duration": 100 }}],
                "meta": {{ "image": "{image}",
                    "layers": [{{ "name": "a", "opacity": {opacity}, "blendMode": "normal" }}],
                    "frameTags": [{{ "name": "t", "from": 0, "to": 0, "direction": "forward" }}],
                    "slices": [{{ "name": "s", "keys": [{{ "frame": 0 }}] }}] }} }}"#
        )
    }

    #[test]
    fn filenames_do_not_make_a_diff() {
        let a = parse_meta(&doc("1-orig.png", "walk 0.aseprite", 255)).unwrap();
        let b = parse_meta(&doc("1-ours.png", "1 0.aseprite", 255)).unwrap();
        assert_eq!(first_meta_diff(&a, &b), None);
    }

    #[test]
    fn layer_opacity_names_meta_layers() {
        let a = parse_meta(&doc("1-orig.png", "walk 0.aseprite", 255)).unwrap();
        let b = parse_meta(&doc("1-orig.png", "walk 0.aseprite", 128)).unwrap();
        assert_eq!(first_meta_diff(&a, &b), Some("meta.layers"));
    }

    #[test]
    fn each_key_is_named_in_order() {
        let base = parse_meta(&doc("1-orig.png", "walk 0.aseprite", 255)).unwrap();

        let tags =
            doc("1-orig.png", "walk 0.aseprite", 255).replace("\"name\": \"t\"", "\"name\": \"u\"");
        assert_eq!(
            first_meta_diff(&base, &parse_meta(&tags).unwrap()),
            Some("meta.frameTags")
        );

        let slices =
            doc("1-orig.png", "walk 0.aseprite", 255).replace("\"name\": \"s\"", "\"name\": \"z\"");
        assert_eq!(
            first_meta_diff(&base, &parse_meta(&slices).unwrap()),
            Some("meta.slices")
        );

        let durations = doc("1-orig.png", "walk 0.aseprite", 255)
            .replace("\"duration\": 100", "\"duration\": 150");
        assert_eq!(
            first_meta_diff(&base, &parse_meta(&durations).unwrap()),
            Some("durations")
        );
    }

    #[test]
    fn parse_meta_rejects_a_shallow_document() {
        assert!(parse_meta("not json").is_err());
        assert!(parse_meta("{}").is_err());
        let no_duration =
            r#"{ "frames": [{}], "meta": { "layers": [], "frameTags": [], "slices": [] } }"#;
        assert!(parse_meta(no_duration).is_err());
    }
}
