//! Aseprite render comparison for the corpus round-trip tool.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::aseprite::{self, AsepriteRun, Diagnostics};
use crate::compare::{RgbaImage, decode_png, diff_pixels};

/// Which render pass runs: the layers as saved, or every layer forced visible.
#[derive(Clone, Copy)]
pub enum Mode {
    Visible,
    AllLayers,
}

/// Why the two renders of a file disagree.
pub enum RenderDiff {
    /// Aseprite refused the original input. The detail is its output.
    Orig(String),
    /// Aseprite refused our rewrite. The detail is its output.
    Ours(String),
    /// The renders disagree with no single frame to name.
    Detail(String),
    /// Frame `key` differs. `pixels` counts the mismatched positions.
    Frame { key: String, pixels: usize },
}

/// Renders `input` and `ours` with Aseprite in one mode and compares frames
/// with matching names. Aseprite writes them into `<out>/render` as
/// `<n>-{orig|ours}-{frame}.png`, or `<n>-{orig|ours}-all-{frame}.png` when
/// `mode` is `AllLayers`. Returns the diff plus the diagnostic lines the two
/// runs printed; a matching pair gives `(None, diags)`.
pub fn compare(
    aseprite: &Path,
    out: &Path,
    n: usize,
    input: &Path,
    ours: &Path,
    mode: Mode,
) -> Result<(Option<RenderDiff>, Diagnostics)> {
    let render = out.join("render");
    let mut diags = Diagnostics::default();
    for (tag, file) in [("orig", input), ("ours", ours)] {
        match run(aseprite, file, &render, n, tag, mode)? {
            AsepriteRun::Ok(lines) => {
                if tag == "orig" {
                    diags.orig.extend(lines);
                } else {
                    diags.ours.extend(lines);
                }
            }
            AsepriteRun::Fail(detail) => {
                let diff = if tag == "orig" {
                    RenderDiff::Orig(detail)
                } else {
                    RenderDiff::Ours(detail)
                };
                return Ok((Some(diff), diags));
            }
        }
    }
    let orig = collect(&render, n, "orig", mode)?;
    let ours = collect(&render, n, "ours", mode)?;
    let mut keys: Vec<String> = orig.keys().chain(ours.keys()).cloned().collect();
    // Frame names are decimal numbers, so order them as numbers.
    keys.sort_by(|a, b| match (a.parse::<u32>(), b.parse::<u32>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => a.cmp(b),
    });
    keys.dedup();
    if keys.is_empty() {
        let diff = RenderDiff::Detail(String::from("aseprite wrote no frame files"));
        return Ok((Some(diff), diags));
    }
    for key in keys {
        let pixels = match (orig.get(&key), ours.get(&key)) {
            (Some(a), Some(b)) => {
                let count = diff_pixels(&load(a)?, &load(b)?);
                if count == 0 {
                    continue;
                }
                count
            }
            // The key came from one of the two maps, so one side always holds it.
            (a, b) => match a.or(b) {
                Some(only) => load(only)?.pixels.len() / 4,
                None => continue,
            },
        };
        return Ok((Some(RenderDiff::Frame { key, pixels }), diags));
    }
    Ok((None, diags))
}

/// Renders one file in one mode. A spawn failure is an Err, which the caller
/// records as this file's ERROR.
fn run(
    aseprite: &Path,
    file: &Path,
    render: &Path,
    n: usize,
    tag: &str,
    mode: Mode,
) -> Result<AsepriteRun> {
    let name = match mode {
        Mode::Visible => format!("{n}-{tag}-{{frame}}.png"),
        Mode::AllLayers => format!("{n}-{tag}-all-{{frame}}.png"),
    };
    let template = render.join(name);
    let mut command = Command::new(aseprite);
    command.arg("-b");
    if let Mode::AllLayers = mode {
        command.arg("--all-layers");
    }
    let output = command
        .arg(file)
        .arg("--save-as")
        .arg(&template)
        .output()
        .with_context(|| format!("running {}", aseprite.display()))?;
    Ok(aseprite::classify(&output))
}

/// Maps Aseprite's `{frame}` substitution to the file it wrote for this run.
fn collect(render: &Path, n: usize, tag: &str, mode: Mode) -> Result<BTreeMap<String, PathBuf>> {
    let prefix = match mode {
        Mode::Visible => format!("{n}-{tag}-"),
        Mode::AllLayers => format!("{n}-{tag}-all-"),
    };
    let mut out = BTreeMap::new();
    let entries =
        std::fs::read_dir(render).with_context(|| format!("listing {}", render.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("listing {}", render.display()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(key) = name
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(".png"))
        {
            // The all-layers pass writes `<n>-{tag}-all-{frame}.png`, which the
            // visible prefix also matches as key `all-{frame}`. `{frame}` is
            // decimal, so those names never belong to the visible pass.
            if matches!(mode, Mode::Visible) && key.starts_with("all-") {
                continue;
            }
            out.insert(key.to_string(), entry.path());
        }
    }
    Ok(out)
}

fn load(path: &Path) -> Result<RgbaImage> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    decode_png(&bytes).with_context(|| format!("decoding {}", path.display()))
}
