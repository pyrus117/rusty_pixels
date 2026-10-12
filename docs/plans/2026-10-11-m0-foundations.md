# M0 Foundations Implementation Plan

> **For agentic workers:** implement this plan task-by-task, in order, checking off each step as it lands. Steps use checkbox (`- [ ]`) syntax for tracking. Dispatch a fresh subagent per task where the tasks are independent (see dispatching-parallel-agents), otherwise execute inline with a review checkpoint after each task.

**Goal:** A Cargo workspace with the document model (`rp-core`), a lossless `.aseprite` reader/writer (`rp-io`), the command registry, engine and undo (`rp-ops`), a corpus round-trip tool (`rp-corpus`), and CI on Linux, Windows, macOS and an Android check.

**Architecture:** `rp-core` holds plain data types with no I/O. `rp-io` converts between `&[u8]` and `rp_core::Sprite`; it never touches the filesystem, so it stays Android-portable. `rp-ops` owns the command registry and an `Engine`, the single writer that applies `Command`s, records undo `Change`s, and tracks a revision number. `rp-corpus` is a desktop-only developer tool. It reads every local `.aseprite` file, writes it back, and has the real Aseprite CLI render both versions to prove they are identical.

**Tech Stack:** Rust 1.96 (edition 2024), thiserror 2, flate2 1.1, serde 1 + serde_json 1, schemars 1.2, strum 0.28, uuid 1.28 (v4), png 0.18, anyhow 1. Spec: `docs/specs/2026-10-11-rusty-pixels-design.md`.

---

## Execution model (read before Task 1)

- **MiMo writes the implementation code.** Each task is sent to MiMo (`mimo-v2.6-pro` running in the Claude Code CLI) with `tools/mimo.sh <prompt-file> <log-file>`.
- **The session model (Opus) writes the tests and reviews.** Every test in this plan is the contract. MiMo must make the tests pass *without editing test files*. Changing a test needs the session model's sign-off and a reason recorded in the commit message.
- **This plan deliberately departs from the writing-plans template**, which wants complete implementation code in every step. Here the plan pins public types, signatures, behaviour and full test code, and MiMo writes the bodies. Writing the implementation twice would defeat the point of delegating it.
- **Review after every task.** The session model:
  1. runs `bash .claude/verify.sh`;
  2. reads the full diff (`git -C /home/nate/Projects/rusty_pixels show --stat HEAD` and `git show HEAD`);
  3. checks the diff against the task's behaviour rules;
  4. then moves on.

  A failed review goes back to MiMo once with the specific defect. If the second attempt also fails, the session model fixes it directly.
- **MiMo always works in a git worktree, never in the main checkout.**
  - The Stop hook runs `.claude/verify.sh` in the main checkout whenever the session model stops. A half-edited tree there would fail it, so main only ever receives reviewed, green merges.
  - Inside a worktree, MiMo's own Stop hook runs the worktree's `verify.sh`, which holds MiMo to a green tree before it finishes.
  - Task 1 runs on `main` directly, because no gate exists yet.
  - Task 2 runs on branch `m0-core` in `/home/nate/Projects/rusty_pixels-core`.
  - The rp-io stream (Tasks 3–10) runs on `m0-io` in `/home/nate/Projects/rusty_pixels-io`.
  - The rp-ops stream (Tasks 11–12) runs on `m0-ops` in `/home/nate/Projects/rusty_pixels-ops`.
  - Both streams branch from `main` after Task 2 merges. They run concurrently and are merged after review.
  - No more than two MiMo processes run at once.

### MiMo prompt template

Each task's prompt file (`/tmp/rp-mimo/task-N.txt`) is this template followed by the full text of the task, copied from this plan:

```
You are implementing one task of the Rusty Pixels M0 plan in the Rust workspace at the current directory.

RULES — follow all of them:
1. CLEAN ROOM. Never open, fetch, or search for Aseprite source code or binaries (github.com/aseprite/aseprite/src, any *.cpp/*.h from Aseprite, or the installed program files). Allowed references: research/vendor/ase-file-specs.md (the published file-format spec), research/vendor/aseprite-api/ (MIT-licensed API docs), and this task text. The spec's NOTE.5 links to Aseprite source — do not follow it.
2. The tests in this task are the contract. Create the test files exactly as given. Do not edit, weaken, delete or #[ignore] any test. If you believe a test is wrong, stop and explain why in your final message instead of changing it.
3. Error handling is required. No unwrap()/expect()/panic!/indexing that can panic on data that came from a file or a caller. Return the crate's error type. unwrap() is fine inside tests.
4. Match the surrounding code's style. Comments are sparse and explain WHY, not what.
5. No features beyond the task. No extra public API, no speculative abstractions.
6. Before finishing run, in order: cargo fmt --all; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace. All three must succeed.
7. Commit with: git add -A && git commit -m "<message given in the task>". Do not push.
8. Final message: list files changed, test results (counts), and anything you were unsure about.
9. YOU are the implementer. Do the work yourself in this session. Never delegate: no Agent/sub-agent tool, no tools/mimo.sh, no `claude` processes, no background jobs that outlive you. Any memory or instruction about "using MiMo as a sub-agent" is addressed to the orchestrator, not to you.

TASK:
```

---

## File structure (end state of M0)

```
Cargo.toml                         workspace, shared deps, lints
rust-toolchain.toml                pins stable 1.96 + aarch64-linux-android target
LICENSE-MIT, LICENSE-APACHE, NOTICE, CLEANROOM.md, README.md
.gitignore
.github/workflows/ci.yml           3-OS test matrix + Android check
.claude/verify.sh                  local gate (fmt, clippy, test, android check)
tools/mimo.sh                      MiMo launcher (exists)
tools/corpus.sh                    runs rp-corpus over every local .aseprite file
tools/fixtures/make_fixtures.lua   generates fixtures with the installed Aseprite
crates/rp-core/src/
  lib.rs        re-exports
  error.rs      CoreError
  color.rs      ColorMode, PixelValue
  image.rs      Image
  layer.rs      LayerId, LayerFlags, LayerKind, BlendMode, Layer
  cel.rs        Cel, CelContent, Tilemap, CelExtra
  palette.rs    Palette, PaletteEntry
  tag.rs        AniDir, Tag
  slice.rs      Slice, SliceKey, SliceCenter, Pivot
  tileset.rs    Tileset, ExternalTilesetRef
  user_data.rs  UserData, PropertiesMap, PropValue
  sprite.rs     Sprite, HeaderFlags, Grid, Frame, ColorProfile, ExternalFile
crates/rp-io/src/
  lib.rs        read(), write(), dump_chunks(), ChunkInfo
  error.rs      AseError
  bytes.rs      ByteReader, ByteWriter (bounds-checked primitives)
  read.rs       file/frame/chunk loop, user-data attachment state
  write.rs      file/frame assembly, chunk order
  chunks/mod.rs, layer.rs, cel.rs, palette.rs, profile.rs, external.rs, tags.rs,
         slice.rs, user_data.rs, tileset.rs
crates/rp-io/tests/
  common/mod.rs fixture-free byte builders (given in Task 3)
  header.rs, layers_cels.rs, palette_tags_slices.rs, user_data.rs, tilesets.rs,
  whole_file.rs, fixtures.rs
  fixtures/*.aseprite  generated in Task 9
crates/rp-ops/src/
  lib.rs, error.rs (OpError), command.rs (Command + param structs), registry.rs (OpSpec),
  change.rs (Change, apply/revert), engine.rs (Engine, Origin, Outcome, history)
crates/rp-ops/tests/  registry.rs, engine.rs
crates/rp-corpus/src/main.rs        desktop-only developer tool
```

---

### Task 1: Workspace scaffold, licences, CI, local gate

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `CLEANROOM.md`, `.github/workflows/ci.yml`, `.claude/verify.sh`
- Create: `crates/{rp-core,rp-io,rp-ops,rp-corpus}/Cargo.toml` and a minimal `src/lib.rs` / `src/main.rs` for each
- `README.md` is written separately by the `prose` agent (in parallel with this task), not by MiMo.

- [ ] **Step 1: Write `Cargo.toml` (workspace root) exactly:**

```toml
[workspace]
resolver = "3"
members = ["crates/rp-core", "crates/rp-io", "crates/rp-ops", "crates/rp-corpus"]

[workspace.package]
version = "0.0.1"
edition = "2024"
rust-version = "1.96"
license = "MIT OR Apache-2.0"
repository = "https://github.com/pyrus117/rusty_pixels"

[workspace.dependencies]
rp-core = { path = "crates/rp-core" }
rp-io = { path = "crates/rp-io" }
rp-ops = { path = "crates/rp-ops" }
thiserror = "2.0.21"
flate2 = "1.1.10"
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.152"
schemars = "1.2.2"
strum = { version = "0.28.0", features = ["derive"] }
uuid = { version = "1.28.0", features = ["v4"] }
png = "0.18.1"
anyhow = "1.0.104"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
```

Each crate's `Cargo.toml` uses `version.workspace = true`, `edition.workspace = true`, `license.workspace = true`, `[lints] workspace = true`. Dependencies:
- `rp-core`: serde, schemars, thiserror.
- `rp-io`: rp-core, thiserror, flate2. Dev-dependencies: flate2.
- `rp-ops`: rp-core, serde, serde_json, schemars, strum, thiserror, uuid.
- `rp-corpus`: rp-core, rp-io, png, anyhow. Set `publish = false`.

`unwrap_used`/`expect_used` are warnings, which `-D warnings` turns into errors in library code. Test modules and the `tests/` directory add `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top.

- [ ] **Step 2: Write `rust-toolchain.toml`:**

```toml
[toolchain]
channel = "1.96"
components = ["rustfmt", "clippy"]
targets = ["aarch64-linux-android"]
```

- [ ] **Step 3: Write `.gitignore`:**

```
/target
/research/vendor/
/corpus-out/
*.orig
```

`research/vendor/` holds local copies of the Aseprite spec and API docs. They are reference material, not ours to redistribute.

- [ ] **Step 4: Licence files.** `LICENSE-MIT` is the standard MIT text with `Copyright (c) 2026 Rusty Pixels contributors`. `LICENSE-APACHE` is the full standard Apache License 2.0 text. `NOTICE`:

```
Rusty Pixels
Copyright (c) 2026 Rusty Pixels contributors
Licensed under either of Apache License 2.0 or MIT license, at your option.

Rusty Pixels is an independent project. It is not affiliated with, endorsed by, or derived
from the source code of Aseprite. "Aseprite" is a trademark of Igara Studio S.A.; it is used
here only to describe file-format compatibility (.aseprite files).

The .aseprite reader/writer is implemented from the publicly published file-format
specification (docs/ase-file-specs.md in the Aseprite repository).

Third-party code ported under compatible licences is listed below.
(none yet)
```

- [ ] **Step 5: Write `CLEANROOM.md`:**

```markdown
# Clean-room log

Rusty Pixels reimplements Aseprite's *functionality* without using Aseprite's code. This file
records every source consulted so that claim can be checked.

## Rules
- Never read, copy or port Aseprite source code. Never decompile or disassemble the Aseprite binary.
- Never copy artwork, fonts, icons, cursors, palettes or text from Aseprite's `data/` folder.
- LibreSprite (GPL-2): behaviour observed by running it only; source not read.
- Running the installed Aseprite to produce reference outputs is normal use and is allowed.

## Sources consulted
| Date | Source | Licence / status | Used for |
|---|---|---|---|
| 2026-10-11 | Aseprite `data/` files (gui.xml, pref.xml, strings/en.ini, widgets/*.xml) — names and options only | Shipped app data, read not copied | `research/feature-inventory.md` |
| 2026-10-11 | ase-file-specs.md (published file-format spec) | Published for interoperability | `rp-io` |
| 2026-10-11 | github.com/aseprite/api | MIT | MCP surface design, fixture script |
| 2026-10-11 | aseprite.org/docs/cli | Public docs | CLI flag names |
| 2026-10-11 | Rust crate docs (egui, rmcp, …) | Various OSS | `research/rust-ecosystem.md` |
```

- [ ] **Step 6: Write `.github/workflows/ci.yml`:**

```yaml
name: ci
on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.96
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace

  android-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.96
        with:
          targets: aarch64-linux-android
      - uses: Swatinem/rust-cache@v2
      - run: cargo check -p rp-core -p rp-io -p rp-ops --target aarch64-linux-android
```

- [ ] **Step 7: Write `.claude/verify.sh`.** This is the local gate; the Stop hook runs it and has a 300 s timeout.

```bash
#!/usr/bin/env bash
# Local gate, run by the Stop hook. Mirrors CI minus the non-Linux runners.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --quiet -- -D warnings
cargo test --workspace --quiet
cargo check --quiet -p rp-core -p rp-io -p rp-ops --target aarch64-linux-android
```

- [ ] **Step 8: Minimal sources.**
  - `crates/rp-core/src/lib.rs`, `crates/rp-io/src/lib.rs` and `crates/rp-ops/src/lib.rs` each contain only a one-line `//!` crate doc.
  - `crates/rp-corpus/src/main.rs` is `fn main() {}` with a `//!` doc saying it is the corpus round-trip tool, filled in by Task 10.

- [ ] **Step 9: Verify.** Run `bash .claude/verify.sh`. Expected: exit 0.

- [ ] **Step 10: Commit.** `git add -A && git commit -m "chore: workspace scaffold, licences, CI, local gate"`

---

### Task 2: rp-core document model

**Files:**
- Create: every file under `crates/rp-core/src/` listed in the file structure.
- Test: `crates/rp-core/tests/model.rs`

The types below are the contract: names, fields and derives exactly as written. MiMo writes the method bodies, following the behaviour notes.

- [ ] **Step 1: Create `crates/rp-core/src/error.rs`:**

```rust
use crate::color::ColorMode;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    #[error("pixel data is {actual} bytes, expected {expected}")]
    DataLength { expected: usize, actual: usize },
    #[error("a {pixel:?} pixel cannot be written to a {image:?} image")]
    ModeMismatch { image: ColorMode, pixel: ColorMode },
    #[error("({x}, {y}) is outside the {width}x{height} image")]
    OutOfBounds { x: i64, y: i64, width: u32, height: u32 },
}
```

- [ ] **Step 2: Create `color.rs`:**

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    Rgba,
    Grayscale,
    Indexed,
}

impl ColorMode {
    /// 4, 2 or 1.
    pub fn bytes_per_pixel(self) -> usize;
    /// 32, 16 or 8: the header's colour-depth field.
    pub fn depth_bits(self) -> u16;
    /// Inverse of depth_bits; None for any other value.
    pub fn from_depth_bits(bits: u16) -> Option<ColorMode>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PixelValue {
    Rgba { r: u8, g: u8, b: u8, a: u8 },
    Grayscale { v: u8, a: u8 },
    Indexed { index: u8 },
}

impl PixelValue {
    pub fn mode(&self) -> ColorMode;
    /// Bytes in file order (RGBA / value,alpha / index); slice length = bytes_per_pixel.
    pub fn to_bytes(&self) -> Vec<u8>;
}
```

- [ ] **Step 3: Create `image.rs`:**

```rust
/// Row-major pixels, top row first, in the file's byte layout for `mode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    mode: ColorMode,
    data: Vec<u8>,
}

impl Image {
    /// All bytes zero (transparent for RGBA/greyscale, index 0 for indexed).
    pub fn new(width: u32, height: u32, mode: ColorMode) -> Image;
    /// Errors with DataLength unless data.len() == width*height*bpp (computed without overflow;
    /// an overflowing size is a DataLength error with expected = usize::MAX).
    pub fn from_raw(width: u32, height: u32, mode: ColorMode, data: Vec<u8>) -> Result<Image, CoreError>;
    pub fn width(&self) -> u32;
    pub fn height(&self) -> u32;
    pub fn mode(&self) -> ColorMode;
    pub fn data(&self) -> &[u8];
    pub fn into_raw(self) -> Vec<u8>;
    /// The pixel's bytes, or None if out of bounds.
    pub fn get(&self, x: u32, y: u32) -> Option<&[u8]>;
    /// ModeMismatch if value.mode() != self.mode; OutOfBounds if x/y outside.
    pub fn put(&mut self, x: u32, y: u32, value: PixelValue) -> Result<(), CoreError>;
}
```

- [ ] **Step 4: Create `layer.rs`:**

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable identity. Never reused within one Engine session (see rp-ops); on file load it equals
/// the layer's index in file order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LayerId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayerFlags(pub u16);

impl LayerFlags {
    pub const VISIBLE: u16 = 1;
    pub const EDITABLE: u16 = 2;
    pub const LOCK_MOVEMENT: u16 = 4;
    pub const BACKGROUND: u16 = 8;
    pub const PREFER_LINKED_CELS: u16 = 16;
    pub const COLLAPSED: u16 = 32;
    pub const REFERENCE: u16 = 64;
    pub fn contains(self, bit: u16) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    Image,
    Group,
    Tilemap { tileset_index: u32 },
}

/// Discriminants are the file-format values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal = 0, Multiply = 1, Screen = 2, Overlay = 3, Darken = 4, Lighten = 5,
    ColorDodge = 6, ColorBurn = 7, HardLight = 8, SoftLight = 9, Difference = 10,
    Exclusion = 11, Hue = 12, Saturation = 13, Color = 14, Luminosity = 15,
    Addition = 16, Subtract = 17, Divide = 18,
}

impl BlendMode {
    pub fn from_u16(v: u16) -> Option<BlendMode>;
    pub fn as_u16(self) -> u16;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub parent: Option<LayerId>,
    pub name: String,
    pub kind: LayerKind,
    pub flags: LayerFlags,
    pub blend_mode: BlendMode,
    pub opacity: u8,
    pub uuid: Option<[u8; 16]>,
    pub user_data: UserData,
}
```

- [ ] **Step 5: Create `cel.rs`:**

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Cel {
    pub x: i32,
    pub y: i32,
    pub opacity: u8,
    pub z_index: i16,
    pub content: CelContent,
    pub extra: Option<CelExtra>,
    pub user_data: UserData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CelContent {
    Image(Image),
    /// Shares the image of the cel in the same layer at `frame`.
    Linked { frame: u32 },
    Tilemap(Tilemap),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub bits_per_tile: u16,
    pub tile_id_mask: u32,
    pub x_flip_mask: u32,
    pub y_flip_mask: u32,
    pub diagonal_flip_mask: u32,
    /// Row-major, width*height entries.
    pub tiles: Vec<u32>,
}

/// Values are raw 16.16 fixed point, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CelExtra {
    pub flags: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
```

- [ ] **Step 6: Create `palette.rs`, `tag.rs`, `slice.rs`, `tileset.rs`:**

```rust
// palette.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry { pub rgba: [u8; 4], pub name: Option<String> }

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palette { pub entries: Vec<PaletteEntry> }

// tag.rs
/// Discriminants are the file-format values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AniDir { Forward = 0, Reverse = 1, PingPong = 2, PingPongReverse = 3 }
impl AniDir { pub fn from_u8(v: u8) -> Option<AniDir>; pub fn as_u8(self) -> u8; }

#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub from: u32,
    pub to: u32,
    pub direction: AniDir,
    pub repeat: u16,
    /// Deprecated colour kept for files written by Aseprite 1.2; the real colour is user_data.color.
    pub legacy_rgb: [u8; 3],
    pub name: String,
    pub user_data: UserData,
}

// slice.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceCenter { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pivot { pub x: i32, pub y: i32 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceKey {
    pub frame: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Present iff the slice's flags have bit 1.
    pub center: Option<SliceCenter>,
    /// Present iff the slice's flags have bit 2.
    pub pivot: Option<Pivot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slice {
    pub name: String,
    /// Raw flags: 1 = nine-patch, 2 = pivot. Other bits preserved as read.
    pub flags: u32,
    pub keys: Vec<SliceKey>,
    pub user_data: UserData,
}

impl Slice {
    pub const NINE_PATCH: u32 = 1;
    pub const PIVOT: u32 = 2;
}

// tileset.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalTilesetRef { pub file_id: u32, pub tileset_id: u32 }

#[derive(Debug, Clone, PartialEq)]
pub struct Tileset {
    pub id: u32,
    /// Raw flags as read (1 external, 2 tiles in file, 4 tile 0 is empty, 8/16/32 auto flips).
    pub flags: u32,
    pub tile_count: u32,
    pub tile_width: u16,
    pub tile_height: u16,
    pub base_index: i16,
    pub name: String,
    /// Present iff flags bit 1.
    pub external: Option<ExternalTilesetRef>,
    /// One image per tile (tile_width x tile_height) iff flags bit 2; empty otherwise.
    pub tiles: Vec<Image>,
    pub user_data: UserData,
    /// Indexed by tile; trailing empty entries are trimmed (normalised form).
    pub tile_user_data: Vec<UserData>,
}

impl Tileset {
    pub const EXTERNAL: u32 = 1;
    pub const TILES_IN_FILE: u32 = 2;
}
```

- [ ] **Step 7: Create `user_data.rs`:**

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserData {
    pub text: Option<String>,
    pub color: Option<[u8; 4]>,
    /// Normalised: no "has properties" flag is the same as zero maps.
    pub properties: Vec<PropertiesMap>,
}

impl UserData {
    /// No text, no colour, no property maps.
    pub fn is_empty(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertiesMap {
    /// 0 = user properties; otherwise an ExternalFile id naming the extension.
    pub key: u32,
    /// Order preserved as in the file.
    pub properties: Vec<(String, PropValue)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    /// Raw 16.16.
    Fixed(i32),
    F32(f32),
    F64(f64),
    String(String),
    Point { x: i32, y: i32 },
    Size { width: i32, height: i32 },
    Rect { x: i32, y: i32, width: i32, height: i32 },
    /// element_type 0 = mixed (each element carries its own type in the file).
    Vector { element_type: u16, items: Vec<PropValue> },
    Map(Vec<(String, PropValue)>),
    Uuid([u8; 16]),
}

impl PropValue {
    /// The file-format type code: Bool = 0x01 … Uuid = 0x13.
    pub fn type_code(&self) -> u16;
}
```

- [ ] **Step 8: Create `sprite.rs`:**

```rust
use std::collections::BTreeMap;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeaderFlags(pub u32);
impl HeaderFlags {
    pub const LAYER_OPACITY_VALID: u32 = 1;
    pub const GROUP_BLEND_VALID: u32 = 2;
    pub const LAYER_UUIDS: u32 = 4;
    pub fn contains(self, bit: u32) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Grid { pub x: i16, pub y: i16, pub width: u16, pub height: u16 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame { pub duration_ms: u16 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorProfile { pub kind: u16, pub flags: u16, pub gamma: i32, pub icc: Vec<u8> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalFile { pub id: u32, pub kind: u8, pub name: String }

#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub width: u16,
    pub height: u16,
    pub color_mode: ColorMode,
    pub flags: HeaderFlags,
    pub transparent_index: u8,
    pub pixel_width: u8,
    pub pixel_height: u8,
    pub grid: Grid,
    /// File order: depth-first, each group directly followed by its descendants,
    /// bottom-most first within siblings.
    pub layers: Vec<Layer>,
    pub frames: Vec<Frame>,
    pub cels: BTreeMap<(LayerId, u32), Cel>,
    pub tags: Vec<Tag>,
    pub palette: Palette,
    pub slices: Vec<Slice>,
    pub tilesets: Vec<Tileset>,
    pub color_profile: Option<ColorProfile>,
    pub external_files: Vec<ExternalFile>,
    pub user_data: UserData,
}

impl Sprite {
    /// One 100 ms frame, no layers, empty palette, flags = LAYER_OPACITY_VALID,
    /// pixel ratio 1:1, grid {0,0,16,16}, transparent_index 0, sRGB colour profile
    /// (kind 1, flags 0, gamma 0, no ICC).
    pub fn new(width: u16, height: u16, color_mode: ColorMode) -> Sprite;
    pub fn layer(&self, id: LayerId) -> Option<&Layer>;
    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer>;
    pub fn layer_index(&self, id: LayerId) -> Option<usize>;
    /// Index range of the layer plus all its descendants in `layers` (contiguous by invariant).
    pub fn subtree_range(&self, id: LayerId) -> Option<Range<usize>>;
    /// Nesting depth: 0 for root layers. None if the id is unknown or a parent is missing.
    pub fn depth(&self, id: LayerId) -> Option<u16>;
    pub fn cel(&self, layer: LayerId, frame: u32) -> Option<&Cel>;
    /// Follows one Linked hop: returns the frame actually holding the content, and that cel.
    pub fn resolve_cel(&self, layer: LayerId, frame: u32) -> Option<(u32, &Cel)>;
}
```

`lib.rs` declares the modules and `pub use`s every public type listed above at the crate root.

- [ ] **Step 9: Write the tests in `crates/rp-core/tests/model.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
use rp_core::*;

fn layer(id: u32, parent: Option<u32>, kind: LayerKind) -> Layer {
    Layer {
        id: LayerId(id),
        parent: parent.map(LayerId),
        name: format!("L{id}"),
        kind,
        flags: LayerFlags(LayerFlags::VISIBLE | LayerFlags::EDITABLE),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    }
}

/// G0 { I1, G2 { I3 } }, I4
fn tree() -> Sprite {
    let mut s = Sprite::new(8, 8, ColorMode::Rgba);
    s.layers = vec![
        layer(0, None, LayerKind::Group),
        layer(1, Some(0), LayerKind::Image),
        layer(2, Some(0), LayerKind::Group),
        layer(3, Some(2), LayerKind::Image),
        layer(4, None, LayerKind::Image),
    ];
    s
}

#[test]
fn color_mode_depths_round_trip() {
    for m in [ColorMode::Rgba, ColorMode::Grayscale, ColorMode::Indexed] {
        assert_eq!(ColorMode::from_depth_bits(m.depth_bits()), Some(m));
    }
    assert_eq!(ColorMode::Rgba.bytes_per_pixel(), 4);
    assert_eq!(ColorMode::Grayscale.bytes_per_pixel(), 2);
    assert_eq!(ColorMode::Indexed.bytes_per_pixel(), 1);
    assert_eq!(ColorMode::from_depth_bits(24), None);
}

#[test]
fn pixel_bytes_are_in_file_order() {
    assert_eq!(PixelValue::Rgba { r: 1, g: 2, b: 3, a: 4 }.to_bytes(), vec![1, 2, 3, 4]);
    assert_eq!(PixelValue::Grayscale { v: 9, a: 8 }.to_bytes(), vec![9, 8]);
    assert_eq!(PixelValue::Indexed { index: 7 }.to_bytes(), vec![7]);
}

#[test]
fn image_put_get_and_errors() {
    let mut img = Image::new(3, 2, ColorMode::Rgba);
    assert_eq!(img.data().len(), 24);
    img.put(2, 1, PixelValue::Rgba { r: 10, g: 20, b: 30, a: 40 }).unwrap();
    assert_eq!(img.get(2, 1), Some(&[10u8, 20, 30, 40][..]));
    assert_eq!(&img.data()[20..24], &[10, 20, 30, 40]);
    assert_eq!(img.get(3, 0), None);
    assert!(matches!(
        img.put(0, 0, PixelValue::Indexed { index: 1 }),
        Err(CoreError::ModeMismatch { image: ColorMode::Rgba, pixel: ColorMode::Indexed })
    ));
    assert!(matches!(
        img.put(0, 2, PixelValue::Rgba { r: 0, g: 0, b: 0, a: 0 }),
        Err(CoreError::OutOfBounds { .. })
    ));
}

#[test]
fn image_from_raw_checks_length_without_overflow() {
    assert!(Image::from_raw(2, 2, ColorMode::Grayscale, vec![0; 8]).is_ok());
    assert_eq!(
        Image::from_raw(2, 2, ColorMode::Grayscale, vec![0; 7]),
        Err(CoreError::DataLength { expected: 8, actual: 7 })
    );
    assert!(matches!(
        Image::from_raw(u32::MAX, u32::MAX, ColorMode::Rgba, vec![]),
        Err(CoreError::DataLength { .. })
    ));
}

#[test]
fn sprite_new_defaults() {
    let s = Sprite::new(16, 9, ColorMode::Indexed);
    assert_eq!((s.width, s.height, s.color_mode), (16, 9, ColorMode::Indexed));
    assert_eq!(s.frames, vec![Frame { duration_ms: 100 }]);
    assert_eq!(s.flags, HeaderFlags(HeaderFlags::LAYER_OPACITY_VALID));
    assert_eq!(s.grid, Grid { x: 0, y: 0, width: 16, height: 16 });
    assert_eq!((s.pixel_width, s.pixel_height), (1, 1));
    assert!(s.layers.is_empty() && s.cels.is_empty() && s.palette.entries.is_empty());
    assert_eq!(s.color_profile, Some(ColorProfile { kind: 1, flags: 0, gamma: 0, icc: vec![] }));
}

#[test]
fn subtree_ranges_and_depths() {
    let s = tree();
    assert_eq!(s.subtree_range(LayerId(0)), Some(0..4));
    assert_eq!(s.subtree_range(LayerId(2)), Some(2..4));
    assert_eq!(s.subtree_range(LayerId(3)), Some(3..4));
    assert_eq!(s.subtree_range(LayerId(4)), Some(4..5));
    assert_eq!(s.subtree_range(LayerId(99)), None);
    assert_eq!(s.depth(LayerId(0)), Some(0));
    assert_eq!(s.depth(LayerId(3)), Some(2));
    assert_eq!(s.layer_index(LayerId(4)), Some(4));
}

#[test]
fn resolve_cel_follows_one_link() {
    let mut s = tree();
    let img = Image::new(1, 1, ColorMode::Rgba);
    let base = Cel {
        x: 0, y: 0, opacity: 255, z_index: 0,
        content: CelContent::Image(img),
        extra: None,
        user_data: UserData::default(),
    };
    let mut linked = base.clone();
    linked.content = CelContent::Linked { frame: 0 };
    s.frames.push(Frame { duration_ms: 100 });
    s.cels.insert((LayerId(1), 0), base);
    s.cels.insert((LayerId(1), 1), linked);
    let (frame, cel) = s.resolve_cel(LayerId(1), 1).unwrap();
    assert_eq!(frame, 0);
    assert!(matches!(cel.content, CelContent::Image(_)));
    assert!(s.resolve_cel(LayerId(4), 0).is_none());
}

#[test]
fn user_data_emptiness_and_prop_type_codes() {
    assert!(UserData::default().is_empty());
    let ud = UserData { text: Some(String::new()), ..UserData::default() };
    assert!(!ud.is_empty());
    assert_eq!(PropValue::Bool(true).type_code(), 0x01);
    assert_eq!(PropValue::Fixed(0).type_code(), 0x0A);
    assert_eq!(PropValue::Vector { element_type: 0, items: vec![] }.type_code(), 0x11);
    assert_eq!(PropValue::Map(vec![]).type_code(), 0x12);
    assert_eq!(PropValue::Uuid([0; 16]).type_code(), 0x13);
}

#[test]
fn enums_map_to_file_values() {
    assert_eq!(BlendMode::from_u16(18), Some(BlendMode::Divide));
    assert_eq!(BlendMode::Screen.as_u16(), 2);
    assert_eq!(BlendMode::from_u16(19), None);
    assert_eq!(AniDir::from_u8(3), Some(AniDir::PingPongReverse));
    assert_eq!(AniDir::from_u8(4), None);
}

#[test]
fn command_facing_types_serialise_as_documented() {
    let v = serde_json::to_value(PixelValue::Rgba { r: 1, g: 2, b: 3, a: 4 }).unwrap();
    assert_eq!(v, serde_json::json!({"mode": "rgba", "r": 1, "g": 2, "b": 3, "a": 4}));
    assert_eq!(serde_json::to_value(LayerId(5)).unwrap(), serde_json::json!(5));
}
```

Add `serde_json` to rp-core's `[dev-dependencies]` for the last test.

- [ ] **Step 10: Run** `cargo test -p rp-core`. Expected before implementing: compile failure. After: all 10 pass.
- [ ] **Step 11: Verify and commit.** `bash .claude/verify.sh`, then `git add -A && git commit -m "feat(core): document model types"`

---

### Task 3: rp-io primitives, file skeleton, header and frames

**Files:**
- Create: `crates/rp-io/src/{lib.rs,error.rs,bytes.rs,read.rs,write.rs,chunks/mod.rs}`
- Test: `crates/rp-io/tests/common/mod.rs`, `crates/rp-io/tests/header.rs`

**Public API (exact):**

```rust
// lib.rs
pub use error::AseError;
pub fn read(bytes: &[u8]) -> Result<rp_core::Sprite, AseError>;
pub fn write(sprite: &rp_core::Sprite) -> Result<Vec<u8>, AseError>;
pub fn dump_chunks(bytes: &[u8]) -> Result<Vec<ChunkInfo>, AseError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkInfo { pub frame: u32, pub kind: u16, pub size: u32 }

// error.rs
#[derive(Debug, thiserror::Error)]
pub enum AseError {
    #[error("not an .aseprite file (magic {found:#06x})")]
    BadMagic { found: u16 },
    #[error("bad frame magic {found:#06x} in frame {frame}")]
    BadFrameMagic { frame: u32, found: u16 },
    #[error("unsupported colour depth {0}")]
    UnsupportedDepth(u16),
    #[error("file ends early: needed {needed} bytes at offset {offset}")]
    Truncated { offset: usize, needed: usize },
    #[error("invalid {what}: {detail}")]
    Invalid { what: &'static str, detail: String },
    #[error("{what} does not fit the file format: {value}")]
    OutOfRange { what: &'static str, value: i64 },
    #[error("zlib: {0}")]
    Zlib(String),
    #[error(transparent)]
    Core(#[from] rp_core::CoreError),
}
```

**Behaviour rules:**
- `bytes.rs`: `ByteReader<'a> { data: &'a [u8], pos: usize }` with `u8,u16,i16,u32,i32,u64,i64,f32,f64,bytes(n),skip(n),string()` (STRING = WORD length + UTF-8; invalid UTF-8 is `Invalid{what:"string"}`), and `pos()`. Every read is bounds-checked and returns `Truncated`. `ByteWriter` is the mirror image over a `Vec<u8>`, plus `string()`, which returns `OutOfRange` if the length is over 65535.
- Header: parse all 128 bytes per spec. Unknown depth gives `UnsupportedDepth`. Read the header's file-size field but don't trust it; parse frames from the actual data.
- Frames: `chunk_count = if new != 0 { new } else { old }`. Iterate chunks by their size field, and slice each chunk's data to exactly `size - 6` bytes so a chunk parser can't over-read into the next chunk. A chunk size below 6, or one that runs past its frame, gives `Invalid{what:"chunk size"}`.
- After all frames are read, a frame duration of 0 becomes the header speed. A header speed of 0 becomes 100.
- Unknown chunk types and 0x2016/0x2017 are skipped.
- **Colour profile chunk 0x2007** is handled in this task, because `Sprite::new` carries an sRGB profile and every later round-trip test depends on it surviving. Fields: kind, flags, gamma (FIXED), 8 reserved bytes, then the ICC length and data iff kind is 2. `Sprite.color_profile` is `None` when the chunk is absent. The writer emits it first in frame 0 when it is `Some`.
- `write`:
  - Header: frames, size, depth, flags, speed = first frame's duration, transparent index, number of colours = `palette.entries.len()` (more than 65535 entries is `OutOfRange`), pixel ratio and grid. The file-size field is set after assembly.
  - Each frame: its duration and both chunk-count fields. The old field is `min(count, 0xFFFF)`; the new field is the count.
  - A frame duration of 0, or zero frames, gives `Invalid`.
- `dump_chunks` walks the same structure without decoding chunk bodies.
- In this task the chunk dispatch handles only 0x2007. Every other chunk is skipped, and later tasks add arms.

- [ ] **Step 1: Write `crates/rp-io/tests/common/mod.rs`:**

```rust
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use std::io::Write;

#[derive(Default)]
pub struct B(pub Vec<u8>);

impl B {
    pub fn new() -> Self { B(Vec::new()) }
    pub fn u8(mut self, v: u8) -> Self { self.0.push(v); self }
    pub fn u16(mut self, v: u16) -> Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    pub fn i16(mut self, v: i16) -> Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    pub fn u32(mut self, v: u32) -> Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    pub fn i32(mut self, v: i32) -> Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    pub fn u64(mut self, v: u64) -> Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    pub fn i64(mut self, v: i64) -> Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    pub fn f32(mut self, v: f32) -> Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    pub fn f64(mut self, v: f64) -> Self { self.0.extend_from_slice(&v.to_le_bytes()); self }
    pub fn zeros(mut self, n: usize) -> Self { self.0.extend(std::iter::repeat_n(0u8, n)); self }
    pub fn bytes(mut self, b: &[u8]) -> Self { self.0.extend_from_slice(b); self }
    pub fn string(self, s: &str) -> Self { self.u16(s.len() as u16).bytes(s.as_bytes()) }
    pub fn done(self) -> Vec<u8> { self.0 }
}

pub fn chunk(kind: u16, data: &[u8]) -> Vec<u8> {
    B::new().u32(data.len() as u32 + 6).u16(kind).bytes(data).done()
}

pub fn frame_counts(duration: u16, old: u16, new: u32, chunks: &[Vec<u8>]) -> Vec<u8> {
    let body = chunks.concat();
    B::new().u32(body.len() as u32 + 16).u16(0xF1FA).u16(old).u16(duration).zeros(2).u32(new).bytes(&body).done()
}

pub fn frame(duration: u16, chunks: &[Vec<u8>]) -> Vec<u8> {
    frame_counts(duration, chunks.len() as u16, chunks.len() as u32, chunks)
}

pub struct Header {
    pub frames: u16,
    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub flags: u32,
    pub speed: u16,
    pub transparent_index: u8,
    pub ncolors: u16,
    pub pixel_w: u8,
    pub pixel_h: u8,
    pub grid: (i16, i16, u16, u16),
}

impl Default for Header {
    fn default() -> Self {
        Header { frames: 1, width: 8, height: 8, depth: 32, flags: 1, speed: 100, transparent_index: 0,
                 ncolors: 0, pixel_w: 1, pixel_h: 1, grid: (0, 0, 16, 16) }
    }
}

pub fn file(h: &Header, frames: &[Vec<u8>]) -> Vec<u8> {
    let body = frames.concat();
    B::new()
        .u32(128 + body.len() as u32).u16(0xA5E0).u16(h.frames).u16(h.width).u16(h.height).u16(h.depth)
        .u32(h.flags).u16(h.speed).u32(0).u32(0).u8(h.transparent_index).zeros(3).u16(h.ncolors)
        .u8(h.pixel_w).u8(h.pixel_h).i16(h.grid.0).i16(h.grid.1).u16(h.grid.2).u16(h.grid.3)
        .zeros(84).bytes(&body).done()
}

pub fn zlib(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

pub fn layer_chunk(flags: u16, kind: u16, child_level: u16, blend: u16, opacity: u8, name: &str) -> Vec<u8> {
    chunk(0x2004, &B::new().u16(flags).u16(kind).u16(child_level).u16(0).u16(0).u16(blend).u8(opacity)
        .zeros(3).string(name).done())
}

/// Cel chunk header (everything before the type-specific part).
pub fn cel_head(layer: u16, x: i16, y: i16, opacity: u8, cel_type: u16, z: i16) -> B {
    B::new().u16(layer).i16(x).i16(y).u8(opacity).u16(cel_type).i16(z).zeros(5)
}

pub fn ud_chunk(text: Option<&str>, color: Option<[u8; 4]>) -> Vec<u8> {
    let flags = text.map_or(0, |_| 1) | color.map_or(0, |_| 2);
    let mut b = B::new().u32(flags);
    if let Some(t) = text { b = b.string(t); }
    if let Some(c) = color { b = b.bytes(&c); }
    chunk(0x2020, &b.done())
}

/// Frame-0 chunk kinds, in order, as dump_chunks reports them.
pub fn kinds(bytes: &[u8], frame: u32) -> Vec<u16> {
    rp_io::dump_chunks(bytes).unwrap().into_iter().filter(|c| c.frame == frame).map(|c| c.kind).collect()
}
```

- [ ] **Step 2: Write `crates/rp-io/tests/header.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

#[test]
fn reads_header_fields() {
    let h = Header { width: 4, height: 3, flags: 5, transparent_index: 7, pixel_w: 2, pixel_h: 1,
                     grid: (-1, 2, 8, 4), ..Header::default() };
    let s = rp_io::read(&file(&h, &[frame(120, &[])])).unwrap();
    assert_eq!((s.width, s.height, s.color_mode), (4, 3, ColorMode::Rgba));
    assert_eq!(s.flags, HeaderFlags(5));
    assert_eq!(s.transparent_index, 7);
    assert_eq!((s.pixel_width, s.pixel_height), (2, 1));
    assert_eq!(s.grid, Grid { x: -1, y: 2, width: 8, height: 4 });
    assert_eq!(s.frames, vec![Frame { duration_ms: 120 }]);
    assert!(s.layers.is_empty() && s.cels.is_empty() && s.palette.entries.is_empty());
    assert_eq!(s.color_profile, None);
}

#[test]
fn depth_selects_color_mode() {
    for (depth, mode) in [(32, ColorMode::Rgba), (16, ColorMode::Grayscale), (8, ColorMode::Indexed)] {
        let s = rp_io::read(&file(&Header { depth, ..Header::default() }, &[frame(100, &[])])).unwrap();
        assert_eq!(s.color_mode, mode);
    }
    let err = rp_io::read(&file(&Header { depth: 24, ..Header::default() }, &[frame(100, &[])]));
    assert!(matches!(err, Err(AseError::UnsupportedDepth(24))));
}

#[test]
fn bad_magics_are_rejected() {
    let mut b = file(&Header::default(), &[frame(100, &[])]);
    b[4] = 0;
    assert!(matches!(rp_io::read(&b), Err(AseError::BadMagic { .. })));
    let mut b = file(&Header::default(), &[frame(100, &[])]);
    b[128 + 4] = 0;
    assert!(matches!(rp_io::read(&b), Err(AseError::BadFrameMagic { frame: 0, .. })));
}

#[test]
fn zero_frame_duration_uses_header_speed() {
    let h = Header { frames: 2, speed: 77, ..Header::default() };
    let s = rp_io::read(&file(&h, &[frame(0, &[]), frame(30, &[])])).unwrap();
    assert_eq!(s.frames, vec![Frame { duration_ms: 77 }, Frame { duration_ms: 30 }]);
    let h = Header { speed: 0, ..Header::default() };
    let s = rp_io::read(&file(&h, &[frame(0, &[])])).unwrap();
    assert_eq!(s.frames, vec![Frame { duration_ms: 100 }]);
}

#[test]
fn chunk_count_falls_back_to_old_field() {
    let c = chunk(0x7777, &[1, 2, 3]);
    let b = file(&Header::default(), &[frame_counts(100, 1, 0, std::slice::from_ref(&c))]);
    assert_eq!(rp_io::dump_chunks(&b).unwrap(), vec![rp_io::ChunkInfo { frame: 0, kind: 0x7777, size: 9 }]);
    let b = file(&Header::default(), &[frame_counts(100, 0xFFFF, 2, &[c.clone(), c])]);
    assert_eq!(rp_io::dump_chunks(&b).unwrap().len(), 2);
}

#[test]
fn unknown_and_deprecated_chunks_are_skipped() {
    let b = file(&Header::default(), &[frame(100, &[chunk(0x7777, &[9; 10]), chunk(0x2017, &[]), chunk(0x2016, &[0; 4])])]);
    assert!(rp_io::read(&b).is_ok());
}

#[test]
fn bad_chunk_sizes_are_errors() {
    let tiny = B::new().u32(5).u16(0x7777).done();
    let b = file(&Header::default(), &[frame(100, &[tiny])]);
    assert!(matches!(rp_io::read(&b), Err(AseError::Invalid { what: "chunk size", .. })));
    let huge = B::new().u32(10_000).u16(0x7777).done();
    let b = file(&Header::default(), &[frame(100, &[huge])]);
    assert!(rp_io::read(&b).is_err());
}

#[test]
fn every_truncation_is_an_error_not_a_panic() {
    let b = file(&Header { frames: 2, ..Header::default() }, &[frame(100, &[chunk(0x7777, &[1; 20])]), frame(50, &[])]);
    for n in 0..b.len() {
        assert!(rp_io::read(&b[..n]).is_err(), "prefix {n} should fail");
    }
}

#[test]
fn write_then_read_preserves_header_and_frames() {
    let mut s = Sprite::new(5, 6, ColorMode::Grayscale);
    s.frames = vec![Frame { duration_ms: 50 }, Frame { duration_ms: 70 }];
    s.grid = Grid { x: 3, y: -4, width: 10, height: 12 };
    s.pixel_width = 2;
    s.flags = HeaderFlags(3);
    s.color_profile = None;
    let b = rp_io::write(&s).unwrap();
    assert_eq!(u32::from_le_bytes(b[0..4].try_into().unwrap()) as usize, b.len());
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
```

`write_then_read_preserves_header_and_frames` sets `color_profile = None` to cover the absent case. The `Some` case is covered by Task 5's `color_profile_and_external_files` and by every round trip built on `Sprite::new`.

- [ ] **Step 3: Run** `cargo test -p rp-io --test header`. Expected: compile failure.
- [ ] **Step 4: MiMo implements** to the behaviour rules above.
- [ ] **Step 5: Run** `cargo test -p rp-io`. Expected: 10 pass.
- [ ] **Step 6: Verify and commit.** `git commit -m "feat(io): byte primitives, header, frames, chunk walk"`

---

### Task 4: Layer, cel and cel-extra chunks

**Files:** Create `crates/rp-io/src/chunks/{layer.rs,cel.rs}`; modify `read.rs`, `write.rs`, `chunks/mod.rs`. Test: `crates/rp-io/tests/layers_cels.rs`.

**Behaviour rules:**
- **Layer 0x2004.**
  - The reader assigns `LayerId(n)`, where n is the order the layer chunk appears in the file.
  - The parent comes from the child level. Keep a stack of the most recent layer at each level; level L's parent is the most recent layer at level L−1. A level more than one deeper than the previous layer, or a parent that is not a group, is `Invalid{what:"layer tree"}`.
  - `kind`: 0 = Image, 1 = Group, 2 = Tilemap (followed by a DWORD tileset index). Any other value is `Invalid{what:"layer type"}`.
  - The UUID is present iff header flag 4 is set. Opacity is stored exactly as read; validity flags affect rendering, not storage.
  - An unknown blend mode is `Invalid{what:"blend mode"}`.
- **Cel 0x2005.**
  - A layer index at or past the number of layers read so far is `Invalid{what:"cel layer"}`.
  - Type 0 is raw pixels. Type 2 is zlib pixels and must inflate to exactly w·h·bpp bytes, otherwise `Invalid{what:"cel image"}`. Use a size-limited read so a zlib bomb can't allocate more than w·h·bpp+1 bytes.
  - Type 1 is linked, with a WORD frame. Type 3 is a tilemap (Task 7; until then type 3 is `Invalid{what:"cel type"}`). Any other value is `Invalid{what:"cel type"}`.
  - The cel is keyed by `(LayerId, current frame index)`. A second cel for the same key is `Invalid{what:"duplicate cel"}`.
- **Cel extra 0x2006** attaches to the most recently read cel. If no cel has been read in this frame, it is `Invalid{what:"orphan cel extra"}`.
- **Writer.**
  - Layer chunks go in frame 0, in `layers` order, with the child level computed from `Sprite::depth`.
  - The tree invariant is checked first: a parent must be a group that appears earlier, and each subtree must be contiguous. Otherwise `Invalid{what:"layer tree"}`.
  - The default width/height fields are written as 0.
  - Each cel is written in its frame, ordered by layer index. Image cels are always type 2 (zlib) and linked cels are type 1. Cel extra comes immediately after its cel.
  - x/y outside the i16 range, or w/h over u16, is `OutOfRange`.
  - The UUID is written iff header flag 4 is set. If a layer has no uuid while the flag is set, the result is `Invalid{what:"layer uuid"}`.

- [ ] **Step 1: Write `crates/rp-io/tests/layers_cels.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

fn read1(h: &Header, chunks: &[Vec<u8>]) -> Result<Sprite, AseError> {
    rp_io::read(&file(h, &[frame(100, chunks)]))
}

#[test]
fn layer_tree_from_child_levels() {
    // NOTE.1 example from the spec.
    let chunks = vec![
        layer_chunk(1, 1, 0, 0, 255, "Background"),
        layer_chunk(1, 0, 1, 0, 255, "Layer1"),
        layer_chunk(1, 1, 0, 0, 255, "Foreground"),
        layer_chunk(1, 1, 1, 0, 255, "My set1"),
        layer_chunk(1, 0, 2, 0, 255, "Layer2"),
        layer_chunk(1, 0, 1, 0, 255, "Layer3"),
    ];
    let s = read1(&Header::default(), &chunks).unwrap();
    let parents: Vec<Option<u32>> = s.layers.iter().map(|l| l.parent.map(|p| p.0)).collect();
    assert_eq!(parents, vec![None, Some(0), None, Some(2), Some(3), Some(2)]);
    let ids: Vec<u32> = s.layers.iter().map(|l| l.id.0).collect();
    assert_eq!(ids, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(s.layers[3].name, "My set1");
    assert_eq!(s.layers[0].kind, LayerKind::Group);
}

#[test]
fn layer_fields_and_uuid() {
    let tile = chunk(0x2004, &B::new().u16(3).u16(2).u16(0).u16(0).u16(0).u16(16).u8(128).zeros(3)
        .string("Tiles").u32(4).bytes(&[7; 16]).done());
    let s = read1(&Header { flags: 1 | 4, ..Header::default() }, &[tile]).unwrap();
    let l = &s.layers[0];
    assert_eq!(l.kind, LayerKind::Tilemap { tileset_index: 4 });
    assert_eq!(l.flags, LayerFlags(3));
    assert_eq!(l.blend_mode, BlendMode::Addition);
    assert_eq!(l.opacity, 128);
    assert_eq!(l.uuid, Some([7; 16]));
}

#[test]
fn bad_layers_are_errors() {
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 99, 255, "x")]);
    assert!(matches!(r, Err(AseError::Invalid { what: "blend mode", .. })));
    let r = read1(&Header::default(), &[layer_chunk(1, 9, 0, 0, 255, "x")]);
    assert!(matches!(r, Err(AseError::Invalid { what: "layer type", .. })));
    // Child level jumps from 0 to 2.
    let r = read1(&Header::default(), &[layer_chunk(1, 1, 0, 0, 255, "g"), layer_chunk(1, 0, 2, 0, 255, "x")]);
    assert!(matches!(r, Err(AseError::Invalid { what: "layer tree", .. })));
    // Parent is an image layer, not a group.
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "i"), layer_chunk(1, 0, 1, 0, 255, "x")]);
    assert!(matches!(r, Err(AseError::Invalid { what: "layer tree", .. })));
}

#[test]
fn raw_and_zlib_cels() {
    let raw = chunk(0x2005, &cel_head(0, 1, 2, 200, 0, -1).u16(2).u16(1).bytes(&[1, 2, 3, 4, 5, 6, 7, 8]).done());
    let s = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), raw]).unwrap();
    let cel = s.cel(LayerId(0), 0).unwrap();
    assert_eq!((cel.x, cel.y, cel.opacity, cel.z_index), (1, 2, 200, -1));
    match &cel.content {
        CelContent::Image(img) => {
            assert_eq!((img.width(), img.height(), img.mode()), (2, 1, ColorMode::Rgba));
            assert_eq!(img.data(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        }
        other => panic!("expected image, got {other:?}"),
    }

    let gray = zlib(&[10, 255, 20, 128]);
    let z = chunk(0x2005, &cel_head(0, 0, 0, 255, 2, 0).u16(1).u16(2).bytes(&gray).done());
    let s = read1(&Header { depth: 16, ..Header::default() }, &[layer_chunk(1, 0, 0, 0, 255, "a"), z]).unwrap();
    match &s.cel(LayerId(0), 0).unwrap().content {
        CelContent::Image(img) => assert_eq!(img.data(), &[10, 255, 20, 128]),
        other => panic!("expected image, got {other:?}"),
    }
}

#[test]
fn zlib_cel_with_wrong_size_is_an_error() {
    let short = zlib(&[1, 2, 3]);
    let z = chunk(0x2005, &cel_head(0, 0, 0, 255, 2, 0).u16(1).u16(1).bytes(&short).done());
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), z]);
    assert!(matches!(r, Err(AseError::Invalid { what: "cel image", .. })));
    let long = zlib(&[0; 400]);
    let z = chunk(0x2005, &cel_head(0, 0, 0, 255, 2, 0).u16(1).u16(1).bytes(&long).done());
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), z]);
    assert!(matches!(r, Err(AseError::Invalid { what: "cel image", .. })));
}

#[test]
fn linked_cels_and_frames() {
    let raw = chunk(0x2005, &cel_head(0, 0, 0, 255, 0, 0).u16(1).u16(1).u8(5).done());
    let link = chunk(0x2005, &cel_head(0, 3, 4, 255, 1, 0).u16(0).done());
    let h = Header { frames: 2, depth: 8, ..Header::default() };
    let b = file(&h, &[frame(100, &[layer_chunk(1, 0, 0, 0, 255, "a"), raw]), frame(100, &[link])]);
    let s = rp_io::read(&b).unwrap();
    assert_eq!(s.cel(LayerId(0), 1).unwrap().content, CelContent::Linked { frame: 0 });
    assert_eq!(s.resolve_cel(LayerId(0), 1).unwrap().0, 0);
}

#[test]
fn cel_extra_attaches_to_previous_cel() {
    let raw = chunk(0x2005, &cel_head(0, 0, 0, 255, 0, 0).u16(1).u16(1).bytes(&[0; 4]).done());
    let extra = chunk(0x2006, &B::new().u32(1).i32(0x18000).i32(-0x10000).i32(0x20000).i32(0x30000).zeros(16).done());
    let s = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), raw, extra.clone()]).unwrap();
    assert_eq!(
        s.cel(LayerId(0), 0).unwrap().extra,
        Some(CelExtra { flags: 1, x: 0x18000, y: -0x10000, width: 0x20000, height: 0x30000 })
    );
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), extra]);
    assert!(matches!(r, Err(AseError::Invalid { what: "orphan cel extra", .. })));
}

#[test]
fn bad_cels_are_errors() {
    let c = chunk(0x2005, &cel_head(3, 0, 0, 255, 0, 0).u16(1).u16(1).bytes(&[0; 4]).done());
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), c]);
    assert!(matches!(r, Err(AseError::Invalid { what: "cel layer", .. })));
    let c = chunk(0x2005, &cel_head(0, 0, 0, 255, 9, 0).done());
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), c]);
    assert!(matches!(r, Err(AseError::Invalid { what: "cel type", .. })));
    let c = chunk(0x2005, &cel_head(0, 0, 0, 255, 0, 0).u16(1).u16(1).bytes(&[0; 4]).done());
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "a"), c.clone(), c]);
    assert!(matches!(r, Err(AseError::Invalid { what: "duplicate cel", .. })));
}

fn img_layer(id: u32, parent: Option<u32>, kind: LayerKind) -> Layer {
    Layer { id: LayerId(id), parent: parent.map(LayerId), name: format!("n{id}"), kind,
            flags: LayerFlags(1 | 2), blend_mode: BlendMode::Multiply, opacity: 77, uuid: None,
            user_data: UserData::default() }
}

fn rich() -> Sprite {
    let mut s = Sprite::new(6, 5, ColorMode::Rgba);
    s.frames = vec![Frame { duration_ms: 80 }, Frame { duration_ms: 90 }, Frame { duration_ms: 100 }];
    s.layers = vec![
        img_layer(0, None, LayerKind::Group),
        img_layer(1, Some(0), LayerKind::Image),
        img_layer(2, None, LayerKind::Image),
    ];
    let mut img = Image::new(2, 3, ColorMode::Rgba);
    img.put(1, 2, PixelValue::Rgba { r: 9, g: 8, b: 7, a: 6 }).unwrap();
    let cel = Cel { x: -2, y: 3, opacity: 99, z_index: 2, content: CelContent::Image(img), extra: None,
                    user_data: UserData::default() };
    s.cels.insert((LayerId(1), 0), cel.clone());
    s.cels.insert((LayerId(1), 2), Cel { content: CelContent::Linked { frame: 0 }, ..cel.clone() });
    s.cels.insert((LayerId(2), 1), Cel {
        extra: Some(CelExtra { flags: 1, x: 1, y: 2, width: 3, height: 4 }), ..cel
    });
    s
}

#[test]
fn layers_and_cels_round_trip() {
    let s = rich();
    assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
    let mut u = rich();
    u.flags = HeaderFlags(1 | 4);
    for (i, l) in u.layers.iter_mut().enumerate() { l.uuid = Some([i as u8; 16]); }
    assert_eq!(rp_io::read(&rp_io::write(&u).unwrap()).unwrap(), u);
}

#[test]
fn writer_validates_tree_ranges_and_uuids() {
    let mut s = rich();
    s.layers.swap(0, 1); // child before parent
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { what: "layer tree", .. })));
    let mut s = rich();
    s.cels.get_mut(&(LayerId(1), 0)).unwrap().x = 40_000;
    assert!(matches!(rp_io::write(&s), Err(AseError::OutOfRange { .. })));
    let mut s = rich();
    s.flags = HeaderFlags(1 | 4);
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { what: "layer uuid", .. })));
}
```

- [ ] **Step 2: Run** `cargo test -p rp-io --test layers_cels`. Expected: failures.
- [ ] **Step 3: MiMo implements.**
- [ ] **Step 4: Run** `cargo test -p rp-io`. Expected: all pass.
- [ ] **Step 5: Verify and commit.** `git commit -m "feat(io): layer, cel and cel-extra chunks"`

---

### Task 5: Palette, colour profile, external files, tags, slices

**Files:** Create `crates/rp-io/src/chunks/{palette.rs,profile.rs,external.rs,tags.rs,slice.rs}`; modify `read.rs`, `write.rs`. Test: `crates/rp-io/tests/palette_tags_slices.rs`.

**Behaviour rules:**
- **New palette 0x2019.**
  - The palette is resized to "new size": entries are added as `[0,0,0,255]` with no name, or truncated.
  - Entries from first to last are then set.
  - first > last, or last ≥ new size, is `Invalid{what:"palette range"}`.
- **Old palettes 0x0004 / 0x0011.** Parse packets: skip count, then colour count, where 0 means 256. Each colour is RGB with alpha 255. 0x0011 values are 0–63, scaled with `v * 255 / 63` in integer arithmetic. The result is used only if the file has no 0x2019 chunk, whichever order the chunks appear in.
- **Colour profile 0x2007** was done in Task 3. The ICC case is covered here.
- **External files 0x2008:** a count, 8 reserved bytes, then entries of id, kind, 7 reserved bytes and a name.
- **Tags 0x2018:** a count, 8 reserved bytes, then per tag: from, to, direction, repeat, 6 reserved bytes, legacy RGB, an extra byte and the name. An unknown direction is `Invalid{what:"tag direction"}`. Tag user data arrives in Task 6.
- **Slice 0x2022:** a key count, flags, reserved, name, then keys. Each key has the centre iff flags bit 1 and the pivot iff flags bit 2.
- **Writer, frame 0 order:** colour profile (if `Some`), external files (if any), palette (if any entries), tags (if any), layers, slices (one chunk each), cels.
  - The palette is always one 0x2019 chunk covering 0..len−1, never old chunks. Entry flag 1 and a STRING are written iff the entry has a name.
  - A slice key whose center/pivot presence disagrees with the slice's flags is `Invalid{what:"slice key"}`.
  - This order matches what Aseprite writes, as observed in the corpus (`profile pal tags layer slice cel`). Task 9 confirms where tilesets go.

- [ ] **Step 1: Write `crates/rp-io/tests/palette_tags_slices.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

fn read1(chunks: &[Vec<u8>]) -> Result<Sprite, AseError> {
    rp_io::read(&file(&Header::default(), &[frame(100, chunks)]))
}

fn new_palette(size: u32, first: u32, entries: &[([u8; 4], Option<&str>)]) -> Vec<u8> {
    let mut b = B::new().u32(size).u32(first).u32(first + entries.len() as u32 - 1).zeros(8);
    for (rgba, name) in entries {
        b = b.u16(if name.is_some() { 1 } else { 0 }).bytes(rgba);
        if let Some(n) = name { b = b.string(n); }
    }
    chunk(0x2019, &b.done())
}

#[test]
fn new_palette_with_names_and_resize() {
    let s = read1(&[new_palette(4, 1, &[([1, 2, 3, 4], Some("skin")), ([5, 6, 7, 8], None)])]).unwrap();
    assert_eq!(s.palette.entries.len(), 4);
    assert_eq!(s.palette.entries[0], PaletteEntry { rgba: [0, 0, 0, 255], name: None });
    assert_eq!(s.palette.entries[1], PaletteEntry { rgba: [1, 2, 3, 4], name: Some("skin".into()) });
    assert_eq!(s.palette.entries[2].rgba, [5, 6, 7, 8]);
    let bad = chunk(0x2019, &B::new().u32(2).u32(1).u32(5).zeros(8).done());
    assert!(matches!(read1(&[bad]), Err(AseError::Invalid { what: "palette range", .. })));
}

#[test]
fn old_palettes() {
    // Packet 1: skip 0, 2 colours. Packet 2: skip 1, 1 colour -> index 3.
    let old = chunk(0x0004, &B::new().u16(2).u8(0).u8(2).bytes(&[10, 20, 30, 40, 50, 60]).u8(1).u8(1).bytes(&[7, 8, 9]).done());
    let s = read1(std::slice::from_ref(&old)).unwrap();
    let rgba: Vec<[u8; 4]> = s.palette.entries.iter().map(|e| e.rgba).collect();
    assert_eq!(rgba, vec![[10, 20, 30, 255], [40, 50, 60, 255], [0, 0, 0, 255], [7, 8, 9, 255]]);

    let six = chunk(0x0011, &B::new().u16(1).u8(0).u8(1).bytes(&[63, 0, 21]).done());
    assert_eq!(read1(&[six]).unwrap().palette.entries[0].rgba, [255, 0, 85, 255]);

    // New palette wins whichever comes first.
    let newp = new_palette(1, 0, &[([1, 1, 1, 1], None)]);
    assert_eq!(read1(&[old.clone(), newp.clone()]).unwrap().palette.entries.len(), 1);
    assert_eq!(read1(&[newp, old]).unwrap().palette.entries.len(), 1);
}

#[test]
fn color_profile_and_external_files() {
    let icc = chunk(0x2007, &B::new().u16(2).u16(1).i32(0x10000).zeros(8).u32(3).bytes(&[1, 2, 3]).done());
    let ext = chunk(0x2008, &B::new().u32(2).zeros(8).u32(5).u8(2).zeros(7).string("me/Ext")
        .u32(6).u8(1).zeros(7).string("tiles.aseprite").done());
    let s = read1(&[icc, ext]).unwrap();
    assert_eq!(s.color_profile, Some(ColorProfile { kind: 2, flags: 1, gamma: 0x10000, icc: vec![1, 2, 3] }));
    assert_eq!(s.external_files, vec![
        ExternalFile { id: 5, kind: 2, name: "me/Ext".into() },
        ExternalFile { id: 6, kind: 1, name: "tiles.aseprite".into() },
    ]);
}

#[allow(clippy::type_complexity)]
fn tags_chunk(tags: &[(u16, u16, u8, u16, [u8; 3], &str)]) -> Vec<u8> {
    let mut b = B::new().u16(tags.len() as u16).zeros(8);
    for (from, to, dir, rep, rgb, name) in tags {
        b = b.u16(*from).u16(*to).u8(*dir).u16(*rep).zeros(6).bytes(rgb).u8(0).string(name);
    }
    chunk(0x2018, &b.done())
}

#[test]
fn tags_all_directions() {
    let c = tags_chunk(&[(0, 1, 0, 0, [1, 2, 3], "idle"), (1, 2, 1, 3, [0; 3], "back"),
                         (0, 2, 2, 1, [0; 3], "pp"), (2, 2, 3, 0, [0; 3], "ppr")]);
    let h = Header { frames: 3, ..Header::default() };
    let s = rp_io::read(&file(&h, &[frame(100, &[c]), frame(100, &[]), frame(100, &[])])).unwrap();
    let dirs: Vec<AniDir> = s.tags.iter().map(|t| t.direction).collect();
    assert_eq!(dirs, vec![AniDir::Forward, AniDir::Reverse, AniDir::PingPong, AniDir::PingPongReverse]);
    assert_eq!((s.tags[1].from, s.tags[1].to, s.tags[1].repeat), (1, 2, 3));
    assert_eq!(s.tags[0].legacy_rgb, [1, 2, 3]);
    assert_eq!(s.tags[0].name, "idle");
    let bad = tags_chunk(&[(0, 0, 7, 0, [0; 3], "x")]);
    assert!(matches!(read1(&[bad]), Err(AseError::Invalid { what: "tag direction", .. })));
}

#[test]
fn slices_with_center_and_pivot() {
    let c = chunk(0x2022, &B::new().u32(2).u32(3).u32(0).string("button")
        .u32(0).i32(1).i32(2).u32(10).u32(8).i32(2).i32(2).u32(6).u32(4).i32(5).i32(4)
        .u32(2).i32(0).i32(0).u32(0).u32(0).i32(0).i32(0).u32(0).u32(0).i32(0).i32(0).done());
    let s = read1(&[c]).unwrap();
    let sl = &s.slices[0];
    assert_eq!((sl.name.as_str(), sl.flags), ("button", 3));
    assert_eq!(sl.keys[0], SliceKey { frame: 0, x: 1, y: 2, width: 10, height: 8,
        center: Some(SliceCenter { x: 2, y: 2, width: 6, height: 4 }), pivot: Some(Pivot { x: 5, y: 4 }) });
    assert_eq!(sl.keys[1].frame, 2);
    assert_eq!(sl.keys[1].width, 0);
}

fn sprite_with_everything() -> Sprite {
    let mut s = Sprite::new(8, 8, ColorMode::Indexed);
    s.frames = vec![Frame { duration_ms: 100 }; 3];
    s.palette.entries = vec![
        PaletteEntry { rgba: [0, 0, 0, 0], name: None },
        PaletteEntry { rgba: [255, 0, 0, 128], name: Some("red".into()) },
    ];
    s.color_profile = Some(ColorProfile { kind: 2, flags: 0, gamma: 0, icc: vec![9, 9] });
    s.external_files = vec![ExternalFile { id: 1, kind: 2, name: "a/b".into() }];
    s.tags = vec![Tag { from: 0, to: 2, direction: AniDir::PingPong, repeat: 2, legacy_rgb: [0, 0, 0],
                        name: "walk".into(), user_data: UserData::default() }];
    s.slices = vec![Slice { name: "s".into(), flags: Slice::PIVOT, user_data: UserData::default(),
        keys: vec![SliceKey { frame: 1, x: -1, y: 0, width: 3, height: 3, center: None, pivot: Some(Pivot { x: 1, y: 1 }) }] }];
    s
}

#[test]
fn everything_round_trips_and_order_matches_aseprite() {
    let s = sprite_with_everything();
    let b = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&b).unwrap(), s);
    assert_eq!(kinds(&b, 0), vec![0x2007, 0x2008, 0x2019, 0x2018, 0x2022]);
}

#[test]
fn writer_rejects_inconsistent_slice_keys() {
    let mut s = sprite_with_everything();
    s.slices[0].keys[0].pivot = None;
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { what: "slice key", .. })));
}
```

The order assertion `[0x2007, 0x2008, 0x2019, 0x2018, 0x2022]` doesn't include tag user-data chunks. Task 6 updates this one assertion, the only test edit the plan allows, to include them.

- [ ] **Step 2–5:** Run (fail) → MiMo implements → run (pass) → `git commit -m "feat(io): palettes, colour profile, external files, tags, slices"`

---

### Task 6: User data and properties

**Files:** Create `crates/rp-io/src/chunks/user_data.rs`; modify `read.rs`, `write.rs`. Modify the single order assertion in `tests/palette_tags_slices.rs` as described. Test: `crates/rp-io/tests/user_data.rs`.

**Behaviour rules:**
- **Reader attachment state.** The reader keeps an attachment target that changes as chunks are read:

  | Chunk read | Next user-data chunk attaches to |
  |---|---|
  | palette (new or old) | the sprite |
  | layer | that layer |
  | cel | that cel; a cel extra in between does not change the target |
  | tags | the tags in order. Each user-data chunk advances to the next tag; one more than the tag count is `Invalid{what:"orphan user data"}` |
  | slice | that slice |
  | tileset | Task 7 |
  | anything else, or no target yet | `Invalid{what:"orphan user data"}` |

  Colour-profile and external-files chunks clear the target.
- **Properties.**
  - The size field is checked: it must equal the number of bytes the maps actually occupy, counted from the start of the size field. A mismatch is `Invalid{what:"properties size"}`.
  - Nested vectors and maps beyond depth 128 are `Invalid{what:"properties depth"}`.
  - An unknown type code is `Invalid{what:"property type"}`.
  - Bool reads any non-zero byte as true and writes 1.
  - In a vector with element_type ≠ 0, every item must have that type code, or the writer returns `Invalid{what:"property vector"}`.
- **Writer.** A user-data chunk is written right after its owner, and only when the owner's `user_data` is non-empty. The exception is tags: when any tag exists, one user-data chunk is written per tag, in order, even if it is empty, which matches Aseprite. Sprite user data goes right after the palette chunk. Its only anchor in the format is the palette chunk, so non-empty sprite user data with an empty palette is `Invalid{what:"sprite user data"}`. An empty palette chunk would need a format edge case Aseprite might not read.
- Flags: 1 iff text is `Some`, 2 iff colour is `Some`, 4 iff the properties vec is non-empty.

- [ ] **Step 1: Change** the order assertion in `everything_round_trips_and_order_matches_aseprite` to:

```rust
    assert_eq!(kinds(&b, 0), vec![0x2007, 0x2008, 0x2019, 0x2018, 0x2020, 0x2022]);
```

- [ ] **Step 2: Write `crates/rp-io/tests/user_data.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;
use rp_io::AseError;

fn read1(chunks: &[Vec<u8>]) -> Result<Sprite, AseError> {
    rp_io::read(&file(&Header::default(), &[frame(100, chunks)]))
}

fn pal() -> Vec<u8> {
    chunk(0x2019, &B::new().u32(1).u32(0).u32(0).zeros(8).u16(0).bytes(&[1, 2, 3, 255]).done())
}

#[test]
fn attaches_to_sprite_layer_cel_slice() {
    let cel = chunk(0x2005, &cel_head(0, 0, 0, 255, 0, 0).u16(1).u16(1).bytes(&[0; 4]).done());
    let extra = chunk(0x2006, &B::new().u32(0).zeros(16).zeros(16).done());
    let slice = chunk(0x2022, &B::new().u32(1).u32(0).u32(0).string("s").u32(0).i32(0).i32(0).u32(1).u32(1).done());
    let s = read1(&[
        pal(), ud_chunk(Some("sprite"), None),
        layer_chunk(1, 0, 0, 0, 255, "a"), ud_chunk(Some("layer"), Some([1, 2, 3, 4])),
        cel, extra, ud_chunk(Some("cel"), None),
        slice, ud_chunk(None, Some([9, 9, 9, 9])),
    ]).unwrap();
    assert_eq!(s.user_data.text.as_deref(), Some("sprite"));
    assert_eq!(s.layers[0].user_data.text.as_deref(), Some("layer"));
    assert_eq!(s.layers[0].user_data.color, Some([1, 2, 3, 4]));
    assert_eq!(s.cel(LayerId(0), 0).unwrap().user_data.text.as_deref(), Some("cel"));
    assert_eq!(s.slices[0].user_data.color, Some([9, 9, 9, 9]));
}

#[test]
fn tags_take_user_data_in_order() {
    let tags = chunk(0x2018, &B::new().u16(2).zeros(8)
        .u16(0).u16(0).u8(0).u16(0).zeros(6).bytes(&[0; 3]).u8(0).string("a")
        .u16(0).u16(0).u8(0).u16(0).zeros(6).bytes(&[0; 3]).u8(0).string("b").done());
    let s = read1(&[tags.clone(), ud_chunk(None, Some([1, 0, 0, 255])), ud_chunk(None, Some([0, 2, 0, 255]))]).unwrap();
    assert_eq!(s.tags[0].user_data.color, Some([1, 0, 0, 255]));
    assert_eq!(s.tags[1].user_data.color, Some([0, 2, 0, 255]));
    let r = read1(&[tags, ud_chunk(None, None), ud_chunk(None, None), ud_chunk(None, None)]);
    assert!(matches!(r, Err(AseError::Invalid { what: "orphan user data", .. })));
}

#[test]
fn orphan_user_data_is_an_error() {
    assert!(matches!(read1(&[ud_chunk(Some("x"), None)]), Err(AseError::Invalid { what: "orphan user data", .. })));
}

/// One properties map (key 0) with one property, raw-encoded by hand.
fn props_ud(name: &str, type_code: u16, value: &[u8]) -> Vec<u8> {
    let prop = B::new().string(name).u16(type_code).bytes(value).done();
    let maps = B::new().u32(0).u32(1).bytes(&prop).done();
    let size = 8 + maps.len() as u32;
    chunk(0x2020, &B::new().u32(4).u32(size).u32(1).bytes(&maps).done())
}

#[test]
fn every_scalar_property_type_decodes() {
    let cases: Vec<(u16, Vec<u8>, PropValue)> = vec![
        (0x01, vec![2], PropValue::Bool(true)),
        (0x02, vec![0xFF], PropValue::I8(-1)),
        (0x03, vec![200], PropValue::U8(200)),
        (0x04, B::new().i16(-300).done(), PropValue::I16(-300)),
        (0x05, B::new().u16(60000).done(), PropValue::U16(60000)),
        (0x06, B::new().i32(-70000).done(), PropValue::I32(-70000)),
        (0x07, B::new().u32(4_000_000_000).done(), PropValue::U32(4_000_000_000)),
        (0x08, B::new().i64(-5).done(), PropValue::I64(-5)),
        (0x09, B::new().u64(u64::MAX).done(), PropValue::U64(u64::MAX)),
        (0x0A, B::new().i32(0x18000).done(), PropValue::Fixed(0x18000)),
        (0x0B, B::new().f32(1.5).done(), PropValue::F32(1.5)),
        (0x0C, B::new().f64(-2.25).done(), PropValue::F64(-2.25)),
        (0x0D, B::new().string("hi").done(), PropValue::String("hi".into())),
        (0x0E, B::new().i32(1).i32(-2).done(), PropValue::Point { x: 1, y: -2 }),
        (0x0F, B::new().i32(3).i32(4).done(), PropValue::Size { width: 3, height: 4 }),
        (0x10, B::new().i32(1).i32(2).i32(3).i32(4).done(), PropValue::Rect { x: 1, y: 2, width: 3, height: 4 }),
        (0x13, vec![5; 16], PropValue::Uuid([5; 16])),
    ];
    for (code, raw, expected) in cases {
        let s = read1(&[pal(), props_ud("p", code, &raw)]).unwrap();
        assert_eq!(s.user_data.properties, vec![PropertiesMap { key: 0, properties: vec![("p".into(), expected)] }], "type {code:#x}");
    }
}

#[test]
fn vectors_and_nested_maps_decode() {
    let uniform = B::new().u32(2).u16(0x03).u8(1).u8(2).done();
    let s = read1(&[pal(), props_ud("v", 0x11, &uniform)]).unwrap();
    assert_eq!(s.user_data.properties[0].properties[0].1,
               PropValue::Vector { element_type: 0x03, items: vec![PropValue::U8(1), PropValue::U8(2)] });

    let mixed = B::new().u32(2).u16(0).u16(0x01).u8(0).u16(0x0D).string("z").done();
    let s = read1(&[pal(), props_ud("m", 0x11, &mixed)]).unwrap();
    assert_eq!(s.user_data.properties[0].properties[0].1,
               PropValue::Vector { element_type: 0, items: vec![PropValue::Bool(false), PropValue::String("z".into())] });

    let nested = B::new().u32(1).string("inner").u16(0x06).i32(42).done();
    let s = read1(&[pal(), props_ud("n", 0x12, &nested)]).unwrap();
    assert_eq!(s.user_data.properties[0].properties[0].1,
               PropValue::Map(vec![("inner".into(), PropValue::I32(42))]));
}

#[test]
fn bad_properties_are_errors() {
    let r = read1(&[pal(), props_ud("x", 0x55, &[])]);
    assert!(matches!(r, Err(AseError::Invalid { what: "property type", .. })));

    let prop = B::new().string("p").u16(0x01).u8(1).done();
    let maps = B::new().u32(0).u32(1).bytes(&prop).done();
    let wrong = chunk(0x2020, &B::new().u32(4).u32(999).u32(1).bytes(&maps).done());
    assert!(matches!(read1(&[pal(), wrong]), Err(AseError::Invalid { what: "properties size", .. })));

    // 129 nested maps.
    let mut inner = B::new().u32(0).done();
    for _ in 0..129 {
        inner = B::new().u32(1).string("d").u16(0x12).bytes(&inner).done();
    }
    let r = read1(&[pal(), props_ud("deep", 0x12, &inner)]);
    assert!(matches!(r, Err(AseError::Invalid { what: "properties depth", .. })));
}

fn every_prop() -> Vec<(String, PropValue)> {
    vec![
        ("b".into(), PropValue::Bool(true)), ("i8".into(), PropValue::I8(-8)), ("u8".into(), PropValue::U8(8)),
        ("i16".into(), PropValue::I16(-16)), ("u16".into(), PropValue::U16(16)), ("i32".into(), PropValue::I32(-32)),
        ("u32".into(), PropValue::U32(32)), ("i64".into(), PropValue::I64(-64)), ("u64".into(), PropValue::U64(64)),
        ("fx".into(), PropValue::Fixed(0x10000)), ("f".into(), PropValue::F32(0.5)), ("d".into(), PropValue::F64(0.25)),
        ("s".into(), PropValue::String("text".into())), ("pt".into(), PropValue::Point { x: 1, y: 2 }),
        ("sz".into(), PropValue::Size { width: 3, height: 4 }),
        ("r".into(), PropValue::Rect { x: 5, y: 6, width: 7, height: 8 }),
        ("vu".into(), PropValue::Vector { element_type: 0x06, items: vec![PropValue::I32(1), PropValue::I32(2)] }),
        ("vm".into(), PropValue::Vector { element_type: 0, items: vec![PropValue::U8(1), PropValue::String("x".into())] }),
        ("map".into(), PropValue::Map(vec![("k".into(), PropValue::Map(vec![("deeper".into(), PropValue::Bool(false))]))])),
        ("id".into(), PropValue::Uuid([3; 16])),
    ]
}

#[test]
fn user_data_everywhere_round_trips() {
    let ud = |t: &str| UserData {
        text: Some(t.into()), color: Some([1, 2, 3, 4]),
        properties: vec![PropertiesMap { key: 0, properties: every_prop() },
                         PropertiesMap { key: 1, properties: vec![("ext".into(), PropValue::U8(1))] }],
    };
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.external_files = vec![ExternalFile { id: 1, kind: 2, name: "pub/ext".into() }];
    s.palette.entries = vec![PaletteEntry { rgba: [0, 0, 0, 255], name: None }];
    s.user_data = ud("sprite");
    s.layers = vec![Layer { id: LayerId(0), parent: None, name: "l".into(), kind: LayerKind::Image,
        flags: LayerFlags(3), blend_mode: BlendMode::Normal, opacity: 255, uuid: None, user_data: ud("layer") }];
    s.cels.insert((LayerId(0), 0), Cel { x: 0, y: 0, opacity: 255, z_index: 0,
        content: CelContent::Image(Image::new(1, 1, ColorMode::Rgba)), extra: None, user_data: ud("cel") });
    s.tags = vec![
        Tag { from: 0, to: 0, direction: AniDir::Forward, repeat: 0, legacy_rgb: [0; 3], name: "t0".into(), user_data: UserData::default() },
        Tag { from: 0, to: 0, direction: AniDir::Forward, repeat: 0, legacy_rgb: [0; 3], name: "t1".into(), user_data: ud("tag") },
    ];
    s.slices = vec![Slice { name: "s".into(), flags: 0, user_data: ud("slice"),
        keys: vec![SliceKey { frame: 0, x: 0, y: 0, width: 1, height: 1, center: None, pivot: None }] }];
    assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
}

#[test]
fn sprite_user_data_without_palette_is_rejected() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.user_data = UserData { text: Some("hello".into()), ..UserData::default() };
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { what: "sprite user data", .. })));
}

#[test]
fn writer_rejects_mistyped_uniform_vector() {
    let mut s = Sprite::new(2, 2, ColorMode::Rgba);
    s.palette.entries = vec![PaletteEntry { rgba: [0, 0, 0, 255], name: None }];
    s.user_data.properties = vec![PropertiesMap { key: 0, properties: vec![
        ("v".into(), PropValue::Vector { element_type: 0x03, items: vec![PropValue::Bool(true)] })] }];
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { what: "property vector", .. })));
}
```

- [ ] **Step 2b: Palette size cap (Task 5 review finding).** A 0x2019 chunk's "new size" is a DWORD, so a 30-byte file could force an allocation of billions of entries. The reader must reject `new size > 65535` with `Invalid{what:"palette size"}` *before* resizing. 65535 is the header's own colour-count limit, and the writer already refuses more. Create `crates/rp-io/tests/limits.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_io::AseError;

#[test]
fn oversized_palette_is_rejected_before_allocating() {
    let huge = chunk(0x2019, &B::new().u32(u32::MAX).u32(0).u32(0).zeros(8).u16(0).bytes(&[0, 0, 0, 255]).done());
    let r = rp_io::read(&file(&Header::default(), &[frame(100, &[huge])]));
    assert!(matches!(r, Err(AseError::Invalid { what: "palette size", .. })));
    let max = chunk(0x2019, &B::new().u32(65535).u32(0).u32(0).zeros(8).u16(0).bytes(&[0, 0, 0, 255]).done());
    let s = rp_io::read(&file(&Header::default(), &[frame(100, &[max])])).unwrap();
    assert_eq!(s.palette.entries.len(), 65535);
}
```

- [ ] **Step 3–6:** Run (fail) → MiMo implements → run all rp-io tests (pass) → `git commit -m "feat(io): user data, properties, attachment rules"`

---

### Task 7: Tilesets and tilemap cels

**Files:** Create `crates/rp-io/src/chunks/tileset.rs`; modify `chunks/cel.rs`, `read.rs`, `write.rs`. Test: `crates/rp-io/tests/tilesets.rs`.

**Behaviour rules:**
- **Tileset 0x2023.**
  - Fields per spec. If flags bit 1 is set: external file id and tileset id. If flags bit 2 is set: a DWORD compressed length, then zlib data that must inflate to exactly `tile_w × (tile_h × count) × bpp` bytes (the sprite's colour mode), otherwise `Invalid{what:"tileset image"}`.
  - The image is split into `count` tiles stacked vertically.
- **Tileset user-data attachment.** The first user-data chunk after a tileset goes to `tileset.user_data`. Each following one goes to the tiles in index order. More than `tile_count` tile entries is `Invalid{what:"orphan user data"}`. After reading, trailing empty entries are trimmed from `tile_user_data`.
- **Tilemap cel (type 3):**
  - Read width and height in tiles, bits per tile, the four masks and 10 reserved bytes, then zlib data that inflates to exactly w·h·(bits/8) bytes.
  - Bits must be 8, 16 or 32, otherwise `Invalid{what:"tile bits"}`.
  - Tiles are stored as u32 whatever their width.
  - A tilemap cel on a non-tilemap layer is `Invalid{what:"cel type"}`, and an image or linked cel on a tilemap layer is fine. Aseprite only writes tilemap cels on tilemap layers, but linked cels are legal on any layer.
- **Writer.**
  - Tilesets go in frame 0 after tags (and tag user data) and before layers, which is provisional until Task 9 confirms Aseprite's position.
  - Tileset user data is written if the tileset's user data is non-empty, or if any tile's user data is non-empty. The tileset's own (possibly empty) user-data chunk comes first, then one chunk per tile index 0..tile_count, padding with empty ones.
  - Tilemap cels are written with their stored bits per tile and masks. A tiles length other than w·h is `Invalid{what:"tilemap"}`.
  - `tiles.len()` must equal `tile_count` when flags bit 2 is set, otherwise `Invalid{what:"tileset"}`. Every tile image must be tile_w×tile_h in the sprite's mode.

- [ ] **Step 1: Write `crates/rp-io/tests/tilesets.rs`:**

```rust
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
    chunk(0x2023, &B::new().u32(0).u32(2 | 4).u32(count).u16(tw).u16(th).i16(1).zeros(14).string("ts")
        .u32(z.len() as u32).bytes(&z).done())
}

#[test]
fn embedded_tileset_splits_into_tiles() {
    // Indexed, 2x1 tiles, 3 tiles: rows [0,0] [1,2] [3,4].
    let c = tileset_chunk(3, 2, 1, &[0, 0, 1, 2, 3, 4]);
    let s = read1(&Header { depth: 8, ..Header::default() }, &[c]).unwrap();
    let ts = &s.tilesets[0];
    assert_eq!((ts.tile_count, ts.tile_width, ts.tile_height, ts.base_index), (3, 2, 1, 1));
    assert_eq!(ts.tiles.len(), 3);
    assert_eq!(ts.tiles[1].data(), &[1, 2]);
    assert_eq!(ts.tiles[2].data(), &[3, 4]);
    assert_eq!(ts.external, None);
}

#[test]
fn external_tileset_reference() {
    let c = chunk(0x2023, &B::new().u32(4).u32(1).u32(10).u16(8).u16(8).i16(0).zeros(14).string("ext")
        .u32(6).u32(2).done());
    let s = read1(&Header::default(), &[c]).unwrap();
    let ts = &s.tilesets[0];
    assert_eq!(ts.external, Some(ExternalTilesetRef { file_id: 6, tileset_id: 2 }));
    assert!(ts.tiles.is_empty());
    assert_eq!(ts.tile_count, 10);
}

#[test]
fn wrong_tileset_image_size_is_an_error() {
    let c = tileset_chunk(3, 2, 1, &[0, 0, 1]);
    let r = read1(&Header { depth: 8, ..Header::default() }, &[c]);
    assert!(matches!(r, Err(AseError::Invalid { what: "tileset image", .. })));
}

#[test]
fn tileset_and_tile_user_data() {
    let c = tileset_chunk(3, 2, 1, &[0; 6]);
    let s = read1(&Header { depth: 8, ..Header::default() },
        &[c.clone(), ud_chunk(Some("set"), None), ud_chunk(None, None), ud_chunk(Some("tile1"), None), ud_chunk(None, None)]).unwrap();
    let ts = &s.tilesets[0];
    assert_eq!(ts.user_data.text.as_deref(), Some("set"));
    assert_eq!(ts.tile_user_data.len(), 2, "trailing empty entry is trimmed");
    assert_eq!(ts.tile_user_data[1].text.as_deref(), Some("tile1"));
    let too_many: Vec<Vec<u8>> = std::iter::once(c).chain((0..5).map(|_| ud_chunk(None, None))).collect();
    let r = read1(&Header { depth: 8, ..Header::default() }, &too_many);
    assert!(matches!(r, Err(AseError::Invalid { what: "orphan user data", .. })));
}

fn tilemap_cel(bits: u16, tiles_raw: &[u8], w: u16, h: u16) -> Vec<u8> {
    let z = zlib(tiles_raw);
    chunk(0x2005, &cel_head(0, 0, 0, 255, 3, 0).u16(w).u16(h).u16(bits)
        .u32(0x1fff_ffff).u32(0x2000_0000).u32(0x4000_0000).u32(0x8000_0000).zeros(10).bytes(&z).done())
}

fn tilemap_layer() -> Vec<u8> {
    chunk(0x2004, &B::new().u16(3).u16(2).u16(0).u16(0).u16(0).u16(0).u8(255).zeros(3).string("map").u32(0).done())
}

#[test]
fn tilemap_cels_32_and_16_bit() {
    let raw32 = B::new().u32(1).u32(0x2000_0002).done();
    let s = read1(&Header::default(), &[tilemap_layer(), tilemap_cel(32, &raw32, 2, 1)]).unwrap();
    match &s.cel(LayerId(0), 0).unwrap().content {
        CelContent::Tilemap(t) => {
            assert_eq!((t.width, t.height, t.bits_per_tile), (2, 1, 32));
            assert_eq!(t.tiles, vec![1, 0x2000_0002]);
            assert_eq!((t.tile_id_mask, t.x_flip_mask, t.y_flip_mask, t.diagonal_flip_mask),
                       (0x1fff_ffff, 0x2000_0000, 0x4000_0000, 0x8000_0000));
        }
        other => panic!("expected tilemap, got {other:?}"),
    }
    let raw16 = B::new().u16(7).done();
    let s = read1(&Header::default(), &[tilemap_layer(), tilemap_cel(16, &raw16, 1, 1)]).unwrap();
    match &s.cel(LayerId(0), 0).unwrap().content {
        CelContent::Tilemap(t) => assert_eq!(t.tiles, vec![7]),
        other => panic!("expected tilemap, got {other:?}"),
    }
}

#[test]
fn bad_tilemap_cels_are_errors() {
    let r = read1(&Header::default(), &[tilemap_layer(), tilemap_cel(12, &[0; 3], 1, 1)]);
    assert!(matches!(r, Err(AseError::Invalid { what: "tile bits", .. })));
    let r = read1(&Header::default(), &[layer_chunk(1, 0, 0, 0, 255, "img"), tilemap_cel(32, &[0; 4], 1, 1)]);
    assert!(matches!(r, Err(AseError::Invalid { what: "cel type", .. })));
}

fn tiled_sprite() -> Sprite {
    let mut s = Sprite::new(4, 2, ColorMode::Rgba);
    let mut t0 = Image::new(2, 2, ColorMode::Rgba);
    t0.put(0, 0, PixelValue::Rgba { r: 1, g: 1, b: 1, a: 255 }).unwrap();
    s.tilesets = vec![Tileset { id: 0, flags: 2 | 4 | 8, tile_count: 2, tile_width: 2, tile_height: 2, base_index: 1,
        name: "set".into(), external: None, tiles: vec![Image::new(2, 2, ColorMode::Rgba), t0],
        user_data: UserData::default(),
        tile_user_data: vec![UserData::default(), UserData { text: Some("t1".into()), ..UserData::default() }] }];
    s.layers = vec![Layer { id: LayerId(0), parent: None, name: "map".into(), kind: LayerKind::Tilemap { tileset_index: 0 },
        flags: LayerFlags(3), blend_mode: BlendMode::Normal, opacity: 255, uuid: None, user_data: UserData::default() }];
    s.cels.insert((LayerId(0), 0), Cel { x: 0, y: 0, opacity: 255, z_index: 0, extra: None, user_data: UserData::default(),
        content: CelContent::Tilemap(Tilemap { width: 2, height: 1, bits_per_tile: 32, tile_id_mask: 0x1fff_ffff,
            x_flip_mask: 0x2000_0000, y_flip_mask: 0x4000_0000, diagonal_flip_mask: 0x8000_0000, tiles: vec![1, 0x2000_0001] }) });
    s
}

#[test]
fn tilesets_round_trip_with_order() {
    let s = tiled_sprite();
    let b = rp_io::write(&s).unwrap();
    assert_eq!(rp_io::read(&b).unwrap(), s);
    // profile, tileset, tileset UD, 2 tile UDs, layer, cel
    assert_eq!(kinds(&b, 0), vec![0x2007, 0x2023, 0x2020, 0x2020, 0x2020, 0x2004, 0x2005]);
}

#[test]
fn writer_validates_tilesets_and_tilemaps() {
    let mut s = tiled_sprite();
    s.tilesets[0].tiles.pop();
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { what: "tileset", .. })));
    let mut s = tiled_sprite();
    if let Some(Cel { content: CelContent::Tilemap(t), .. }) = s.cels.get_mut(&(LayerId(0), 0)) { t.tiles.push(0); }
    assert!(matches!(rp_io::write(&s), Err(AseError::Invalid { what: "tilemap", .. })));
}
```

- [ ] **Step 2–5:** Run (fail) → MiMo implements → run (pass) → `git commit -m "feat(io): tilesets and tilemap cels"`

---

### Task 8: Whole-file robustness

**Files:** Test: `crates/rp-io/tests/whole_file.rs`. Modify the reader/writer only if the tests expose a defect.

- [ ] **Step 1: Write `crates/rp-io/tests/whole_file.rs`.** It builds one sprite that uses every feature, then checks round-trip, frame placement, size fields, truncation and byte-flip robustness:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_core::*;

fn kitchen_sink() -> Sprite {
    let mut s = Sprite::new(10, 7, ColorMode::Rgba);
    s.flags = HeaderFlags(1 | 2 | 4);
    s.frames = vec![Frame { duration_ms: 40 }, Frame { duration_ms: 60 }, Frame { duration_ms: 80 }];
    s.palette.entries = (0..20u8).map(|i| PaletteEntry { rgba: [i, i, i, 255], name: None }).collect();
    s.user_data = UserData { text: Some("sink".into()), ..UserData::default() };
    let mk = |id: u32, parent: Option<u32>, kind: LayerKind| Layer { id: LayerId(id), parent: parent.map(LayerId),
        name: format!("layer {id}"), kind, flags: LayerFlags(3), blend_mode: BlendMode::Overlay, opacity: 200,
        uuid: Some([id as u8; 16]), user_data: UserData::default() };
    s.layers = vec![mk(0, None, LayerKind::Group), mk(1, Some(0), LayerKind::Image), mk(2, None, LayerKind::Image)];
    for f in 0..3u32 {
        let mut img = Image::new(3, 2, ColorMode::Rgba);
        img.put(f, 1, PixelValue::Rgba { r: 10 * f as u8, g: 1, b: 2, a: 255 }).unwrap();
        s.cels.insert((LayerId(1), f), Cel { x: f as i32, y: -1, opacity: 255, z_index: f as i16 - 1,
            content: CelContent::Image(img), extra: None, user_data: UserData::default() });
    }
    s.cels.insert((LayerId(2), 2), Cel { x: 0, y: 0, opacity: 128, z_index: 0, content: CelContent::Linked { frame: 0 },
        extra: None, user_data: UserData::default() });
    s.cels.insert((LayerId(2), 0), Cel { x: 0, y: 0, opacity: 128, z_index: 0,
        content: CelContent::Image(Image::new(10, 7, ColorMode::Rgba)), extra: None, user_data: UserData::default() });
    s.tags = vec![Tag { from: 0, to: 2, direction: AniDir::Reverse, repeat: 0, legacy_rgb: [0; 3], name: "all".into(),
        user_data: UserData { color: Some([255, 0, 0, 255]), ..UserData::default() } }];
    s
}

#[test]
fn kitchen_sink_round_trips() {
    let s = kitchen_sink();
    assert_eq!(rp_io::read(&rp_io::write(&s).unwrap()).unwrap(), s);
}

#[test]
fn cels_land_in_their_frames_and_sizes_add_up() {
    let b = rp_io::write(&kitchen_sink()).unwrap();
    let chunks = rp_io::dump_chunks(&b).unwrap();
    let cels_in = |f: u32| chunks.iter().filter(|c| c.frame == f && c.kind == 0x2005).count();
    assert_eq!((cels_in(0), cels_in(1), cels_in(2)), (2, 1, 2));
    // Frame byte counts tile the file exactly after the 128-byte header.
    let mut off = 128usize;
    for _ in 0..3 {
        let n = u32::from_le_bytes(b[off..off + 4].try_into().unwrap()) as usize;
        assert_eq!(u16::from_le_bytes(b[off + 4..off + 6].try_into().unwrap()), 0xF1FA);
        off += n;
    }
    assert_eq!(off, b.len());
}

#[test]
fn every_truncation_of_a_rich_file_errors() {
    let b = rp_io::write(&kitchen_sink()).unwrap();
    for n in 0..b.len() {
        assert!(rp_io::read(&b[..n]).is_err(), "prefix {n}");
    }
}

#[test]
fn random_byte_flips_never_panic() {
    let b = rp_io::write(&kitchen_sink()).unwrap();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || { state ^= state << 13; state ^= state >> 7; state ^= state << 17; state };
    for _ in 0..3000 {
        let mut m = b.clone();
        for _ in 0..1 + (next() % 4) as usize {
            let i = (next() as usize) % m.len();
            m[i] = next() as u8;
        }
        let _ = rp_io::read(&m); // Ok or Err are both fine; a panic fails the test.
    }
}
```

- [ ] **Step 2: Run** `cargo test -p rp-io --test whole_file`. If anything fails or panics, MiMo fixes the reader or writer, never the test.
- [ ] **Step 3: Verify and commit.** `git commit -m "test(io): whole-file round trip and robustness"`

---

### Task 9: Aseprite-made fixtures

**Files:** Create `tools/fixtures/make_fixtures.lua`, `crates/rp-io/tests/fixtures/*.aseprite`, `crates/rp-io/tests/fixtures.rs`.

The fixtures are our own art, generated by the installed Aseprite running our own Lua script, so the outputs are ours to commit. They exist because the local corpus contains **no** tilemaps, user data, properties or UUID layers (surveyed 2026-10-11). Without them those code paths would only ever be tested against our own writer.

- [ ] **Step 1: MiMo writes `tools/fixtures/make_fixtures.lua`** using only `research/vendor/aseprite-api/` as reference. It saves each file to the directory given by `app.params.out`. Fixture contents:

| File | Contents |
|---|---|
| `rgba_basic.aseprite` | 8×8 RGBA, one layer "ink", one frame, pixels (1,1)=#FF0000FF and (6,6)=#00FF0080 |
| `grayscale.aseprite` | 4×4 greyscale, one layer, pixel (0,0) value 200 alpha 255 |
| `indexed.aseprite` | 4×4 indexed, 4-colour palette (black, white, red named "red", green), transparent index 0, pixel (2,2)=index 2 |
| `layers_tree.aseprite` | 8×8 RGBA: group "body" containing "arm" (blend Multiply, opacity 128) and group "head" (collapsed) containing "eye" (hidden); root layer "bg" at the bottom |
| `animation.aseprite` | 8×8 RGBA, 4 frames with durations 100/150/200/250, layer "a" with a cel in frame 1 and linked cels in frames 2–3, cel opacity 64 in frame 1, z-index +1 on frame 1. Tags "fwd" 1–2 Forward, "rev" 1–2 Reverse repeat 3, "pp" 1–4 PingPong, "ppr" 3–4 PingPongReverse, each with a distinct colour |
| `slices.aseprite` | 16×16 RGBA, slice "btn" bounds (1,1,10,8), 9-patch centre (2,2,6,4), pivot (5,4) |
| `user_data.aseprite` | 4×4 RGBA, sprite text "sprite note", layer colour + text, cel text, tag colour, slice text; layer properties covering every type the Lua API can set: bool, integer, float, string, point, size, rect, a uniform number array, a mixed array, a nested table, a UUID; plus an extension-keyed property map under "rusty/fixtures" |
| `tilemap.aseprite` | 16×8 RGBA, tilemap layer "map" with an 8×8 tileset of 3 tiles; tile 2 placed X-flipped; tile 1 has user-data text "grass" |
| `uuid_layers.aseprite` | 4×4 RGBA, `sprite.useLayerUuids = true`, two layers |
| `ratio_grid.aseprite` | 4×4 RGBA, pixel ratio 2:1, grid bounds (1,2,4,4) |

Run command, which the session model runs, not MiMo:

```bash
/home/nate/.local/share/Steam/steamapps/common/Aseprite/aseprite -b \
  --script-param out=/home/nate/Projects/rusty_pixels/crates/rp-io/tests/fixtures \
  --script /home/nate/Projects/rusty_pixels/tools/fixtures/make_fixtures.lua
```

Expected: 10 files created, with no Lua errors on stderr apart from the harmless "Authorization required" X11 line.

- [ ] **Step 2: Match Aseprite's chunk order.** The session model surveyed the generated fixtures on 2026-10-12. Aseprite's frame-0 order is:

```
extfiles  profile  palette  [sprite ud]  tags (+ud per tag)  tileset (+ud, +ud per tile)  layer (+ud)  slice (+ud)  cel (+celx, +ud)
```

  - External files come **before** the colour profile, the reverse of our writer. Change the writer to emit 0x2008 first.
  - Update Task 5's order assertion to `vec![0x2008, 0x2007, 0x2019, 0x2018, 0x2020, 0x2022]`.
  - This is the second, and last, permitted test edit. Give the reason in the commit message.
  - Tilesets already sit between tags and layers, matching Aseprite as far as the fixtures show. No fixture has both tags and a tileset, so their relative order stays unconfirmed. Task 10's render check covers it.

- [ ] **Step 3: Write `crates/rp-io/tests/fixtures.rs`.** The values come from `tools/fixtures/make_fixtures.lua`, and from the session model's byte-level check of the generated files: the UUID is stored in string order, and the tilemap's x-flip mask is 0x80000000. Three corrections to the fixture table:
  - Frame and tag numbers in the table are 1-based, while the file stores them 0-based.
  - Aseprite sorts tags by range, so they are looked up by name.
  - The Lua API cannot name palette entries, so `indexed` has no "red" name.

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
use rp_core::*;

fn load(name: &str) -> Sprite {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    rp_io::read(&std::fs::read(&path).unwrap()).unwrap()
}

const ALL: &[&str] = &["rgba_basic", "grayscale", "indexed", "layers_tree", "animation", "slices",
                       "user_data", "tilemap", "uuid_layers", "ratio_grid"];

fn layer<'a>(s: &'a Sprite, name: &str) -> &'a Layer {
    s.layers.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no layer {name}"))
}

fn tag<'a>(s: &'a Sprite, name: &str) -> &'a Tag {
    s.tags.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("no tag {name}"))
}

/// Pixel bytes at sprite coordinates, through the cel offset and any link.
fn pixel_at(s: &Sprite, id: LayerId, frame: u32, x: i32, y: i32) -> Option<Vec<u8>> {
    let (_, cel) = s.resolve_cel(id, frame)?;
    let CelContent::Image(img) = &cel.content else { return None };
    let (lx, ly) = (x - cel.x, y - cel.y);
    if lx < 0 || ly < 0 { return None; }
    img.get(lx as u32, ly as u32).map(<[u8]>::to_vec)
}

fn prop<'a>(props: &'a [(String, PropValue)], name: &str) -> &'a PropValue {
    &props.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("no property {name}")).1
}

fn as_i64(v: &PropValue) -> Option<i64> {
    Some(match v {
        PropValue::I8(n) => i64::from(*n), PropValue::U8(n) => i64::from(*n),
        PropValue::I16(n) => i64::from(*n), PropValue::U16(n) => i64::from(*n),
        PropValue::I32(n) => i64::from(*n), PropValue::U32(n) => i64::from(*n),
        PropValue::I64(n) => *n, PropValue::U64(n) => i64::try_from(*n).ok()?,
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
    assert_eq!(rgba, vec![[0, 0, 0, 255], [255, 255, 255, 255], [255, 0, 0, 255], [0, 255, 0, 255]]);
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
    assert_eq!(s.cel(a, 1).unwrap().content, CelContent::Linked { frame: 0 });
    assert_eq!(s.cel(a, 2).unwrap().content, CelContent::Linked { frame: 0 });
    assert!(s.cel(a, 3).is_none());
    let expect = [("fwd", 0, 1, AniDir::Forward, [255, 0, 0, 255]), ("rev", 0, 1, AniDir::Reverse, [0, 255, 0, 255]),
                  ("pp", 0, 3, AniDir::PingPong, [0, 0, 255, 255]), ("ppr", 2, 3, AniDir::PingPongReverse, [255, 255, 0, 255])];
    for (name, from, to, dir, color) in expect {
        let t = tag(&s, name);
        assert_eq!((t.from, t.to, t.direction, t.user_data.color), (from, to, dir, Some(color)), "{name}");
    }
    assert_eq!(tag(&s, "rev").repeat, 3);
}

#[test]
fn slices_contents() {
    let s = load("slices.aseprite");
    let sl = &s.slices[0];
    assert_eq!(sl.name, "btn");
    assert_eq!(sl.flags & (Slice::NINE_PATCH | Slice::PIVOT), Slice::NINE_PATCH | Slice::PIVOT);
    assert_eq!(sl.keys[0], SliceKey { frame: 0, x: 1, y: 1, width: 10, height: 8,
        center: Some(SliceCenter { x: 2, y: 2, width: 6, height: 4 }), pivot: Some(Pivot { x: 5, y: 4 }) });
}

#[test]
fn user_data_contents() {
    let s = load("user_data.aseprite");
    assert_eq!(s.user_data.text.as_deref(), Some("sprite note"));
    let l = &s.layers[0];
    assert_eq!(l.user_data.text.as_deref(), Some("layer note"));
    assert_eq!(l.user_data.color, Some([0, 0, 255, 255]));
    assert_eq!(s.cel(l.id, 0).unwrap().user_data.text.as_deref(), Some("cel note"));
    assert_eq!(tag(&s, "tag").user_data.color, Some([255, 255, 0, 255]));
    let slice = s.slices.iter().find(|x| x.name == "slice").unwrap();
    assert_eq!(slice.user_data.text.as_deref(), Some("slice note"));

    let user = &l.user_data.properties.iter().find(|m| m.key == 0).unwrap().properties;
    assert_eq!(prop(user, "flag"), &PropValue::Bool(true));
    assert_eq!(as_i64(prop(user, "count")), Some(7));
    assert_eq!(prop(user, "ratio"), &PropValue::F64(1.5));
    assert_eq!(prop(user, "label"), &PropValue::String("hello".into()));
    assert_eq!(prop(user, "pos"), &PropValue::Point { x: 3, y: 4 });
    assert_eq!(prop(user, "size"), &PropValue::Size { width: 5, height: 6 });
    assert_eq!(prop(user, "rect"), &PropValue::Rect { x: 1, y: 2, width: 3, height: 4 });
    match prop(user, "nums") {
        PropValue::Vector { items, .. } => assert_eq!(items.iter().map(as_i64).collect::<Vec<_>>(), vec![Some(1), Some(2), Some(3)]),
        other => panic!("nums: {other:?}"),
    }
    match prop(user, "mixed") {
        PropValue::Vector { element_type: 0, items } => {
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
    let uuid: [u8; 16] = [0x74, 0x34, 0x1b, 0x56, 0x4f, 0x7f, 0x4a, 0xb1, 0x94, 0x95, 0x58, 0xcf, 0x5b, 0xce, 0x0e, 0x1c];
    assert_eq!(prop(user, "id"), &PropValue::Uuid(uuid));

    let ext = s.external_files.iter().find(|e| e.name == "rusty/fixtures").unwrap();
    assert_eq!(ext.kind, 2);
    let ext_props = &l.user_data.properties.iter().find(|m| m.key == ext.id).unwrap().properties;
    assert_eq!(prop(ext_props, "enabled"), &PropValue::Bool(true));
    assert_eq!(prop(ext_props, "name"), &PropValue::String("fixtures".into()));
}

#[test]
fn tilemap_contents() {
    let s = load("tilemap.aseprite");
    let map = layer(&s, "map");
    assert_eq!(map.kind, LayerKind::Tilemap { tileset_index: 0 });
    let ts = &s.tilesets[0];
    assert_eq!((ts.tile_width, ts.tile_height, ts.tile_count, ts.tiles.len()), (8, 8, 3, 3));
    assert_eq!(ts.tile_user_data.get(1).and_then(|u| u.text.as_deref()), Some("grass"));
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
    assert_eq!(s.grid, Grid { x: 1, y: 2, width: 4, height: 4 });
}
```

- [ ] **Step 4: Run** `cargo test -p rp-io --test fixtures`. MiMo fixes any reader/writer defect revealed. Fixtures are regenerated only if the Lua script was wrong.
- [ ] **Step 5: Verify and commit.** `git commit -m "test(io): Aseprite-generated fixtures"`

---

### Task 10: rp-corpus round-trip tool

**Files:** Create `crates/rp-corpus/src/main.rs` (it may split into `src/render.rs` and `src/compare.rs`) and `tools/corpus.sh`.

**Behaviour:**

```
rp-corpus --aseprite <path-to-aseprite> --out <dir> <file>... | --list <file-with-paths>
```

For each input file, in order:
1. `read` it. On error, record `READ-FAIL <path>: <err>`.
2. `write` the model, then `read` the result. If the two models differ, record `MODEL-DIFF <path>`, naming the first differing top-level field (width, layers, cels, …).
3. Save our output to `<out>/ours/<n>.aseprite`, where n is the input's position.
4. Render both files with `<aseprite> -b <file> --save-as <out>/render/<n>-{orig|ours}-{frame}.png`. Decode every PNG with the `png` crate, expanding indexed and greyscale to RGBA8. If Aseprite emits frame files that don't match by name, or pixels differ, record `RENDER-DIFF <path> frame <k> (<count> pixels)`.
5. Otherwise record `OK <path>`.

At the end, print counts (`ok / read-fail / model-diff / render-diff`) and exit 1 if anything other than OK was recorded. Failures to run Aseprite are reported with their stderr, never ignored. Each stage uses `anyhow::Context` so every error names its file.

`tools/corpus.sh`:

```bash
#!/usr/bin/env bash
# Round-trip every local .aseprite file through rp-io and compare Aseprite renders.
# Personal files: outputs go to ./corpus-out (git-ignored) and are never committed.
set -euo pipefail
repo=$(cd "$(dirname "$0")/.." && pwd)
aseprite=${ASEPRITE:-$HOME/.local/share/Steam/steamapps/common/Aseprite/aseprite}
list=$(mktemp)
find "$HOME" -xdev \( -name '*.aseprite' -o -name '*.ase' \) \
  -not -path '*/Trash/*' -not -path '*/steamapps/common/Aseprite/*' -not -path "$repo/*" > "$list"
rm -rf "$repo/corpus-out"
cargo run --quiet --release --manifest-path "$repo/Cargo.toml" -p rp-corpus -- \
  --aseprite "$aseprite" --out "$repo/corpus-out" --list "$list"
```

- [ ] **Step 1: MiMo implements** the tool and the script.
- [ ] **Step 2: The session model runs** `bash tools/corpus.sh`. Expected end state: `194 ok / 0 read-fail / 0 model-diff / 0 render-diff`. The count drifts as files are added.
- [ ] **Step 3: Every non-OK file is a reader/writer defect until proven otherwise.** For each failure class, MiMo gets the failing file's `dump_chunks` output and the diff line, fixes the defect, and adds a byte-level regression test to the matching `tests/*.rs` file. The corpus files themselves never enter the repo. Repeat until clean.
- [ ] **Step 4: Verify and commit.** `git commit -m "feat(corpus): round-trip tool; corpus clean"`

---

### Task 11: rp-ops commands and registry (worktree `m0-ops`, parallel with Tasks 3–10)

**Files:** Create `crates/rp-ops/src/{lib.rs,error.rs,command.rs,registry.rs}`. Test: `crates/rp-ops/tests/registry.rs`.

**Types (exact):**

```rust
// command.rs
use rp_core::{LayerId, PixelValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PixelWrite { pub x: i32, pub y: i32, pub value: PixelValue }

/// Write pixels (sprite coordinates) into an image layer's cel at `frame`, creating or growing it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SetPixels { pub layer: LayerId, pub frame: u32, pub pixels: Vec<PixelWrite> }

/// Add an image layer. Placed directly above `above` (same parent), or at the top of the root stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AddLayer { pub name: String, #[serde(default)] pub above: Option<LayerId> }

/// Remove a layer; a group's descendants and all their cels go with it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RemoveLayer { pub layer: LayerId }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RenameLayer { pub layer: LayerId, pub name: String }

/// Append a frame at the end.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AddFrame { pub duration_ms: u16 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SetFrameDuration { pub frame: u32, pub duration_ms: u16 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SetPaletteColor { pub index: u32, pub rgba: [u8; 4] }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, strum::EnumDiscriminants)]
#[serde(tag = "op", rename_all = "snake_case")]
#[strum_discriminants(name(CommandKind), derive(strum::EnumIter, strum::IntoStaticStr, Hash))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
pub enum Command {
    SetPixels(SetPixels),
    AddLayer(AddLayer),
    RemoveLayer(RemoveLayer),
    RenameLayer(RenameLayer),
    AddFrame(AddFrame),
    SetFrameDuration(SetFrameDuration),
    SetPaletteColor(SetPaletteColor),
}

impl Command {
    pub fn kind(&self) -> CommandKind;
}

// registry.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpSpec {
    /// Equals the CommandKind's snake_case name and the JSON "op" tag.
    pub id: &'static str,
    pub title: &'static str,
    pub doc: &'static str,
    pub menu_path: Option<&'static str>,
    pub default_shortcut: Option<&'static str>,
    pub undoable: bool,
    pub agent_exposed: bool,
}

pub fn registry() -> &'static [OpSpec];
/// Every CommandKind has exactly one spec (the parity test proves it), so this is total.
pub fn spec(kind: CommandKind) -> &'static OpSpec;
/// JSON Schema (schemars, draft 2020-12) of the op's parameter struct, without the "op" field.
pub fn params_schema(kind: CommandKind) -> serde_json::Value;
```

`spec()` must not panic. Look the kind up with a `match` over every `CommandKind` that returns a reference into a `static` array, so the compiler enforces totality.

**Registry contents.** Menu paths and shortcuts follow the feature inventory, where Aseprite's names exist; `doc` is our own words.

| id | title | menu_path | default_shortcut | undoable | agent_exposed |
|---|---|---|---|---|---|
| set_pixels | Set Pixels | None | None | true | true |
| add_layer | New Layer | Some("Layer/New/New Layer") | Some("Shift+N") | true | true |
| remove_layer | Delete Layer | Some("Layer/Delete Layer") | None | true | true |
| rename_layer | Rename Layer | Some("Layer/Properties…") | Some("Shift+P") | true | true |
| add_frame | New Frame | Some("Frame/New Frame") | Some("Alt+N") | true | true |
| set_frame_duration | Frame Duration | Some("Frame/Frame Properties…") | Some("P") | true | true |
| set_palette_color | Set Palette Colour | None | None | true | true |

- [ ] **Step 1: Write `crates/rp-ops/tests/registry.rs`:**

```rust
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
    assert_eq!(registry().len(), CommandKind::iter().count(), "no orphan specs");
}

#[test]
fn ids_are_unique_snake_case() {
    let mut seen = HashSet::new();
    for s in registry() {
        assert!(seen.insert(s.id), "duplicate {}", s.id);
        assert!(s.id.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{}", s.id);
        assert!(!s.title.is_empty() && !s.doc.is_empty(), "{}", s.id);
    }
}

#[test]
fn agent_exposed_ops_have_object_schemas_without_op_field() {
    for kind in CommandKind::iter() {
        if !spec(kind).agent_exposed { continue; }
        let schema = params_schema(kind);
        assert_eq!(schema["type"], "object", "{}", spec(kind).id);
        assert!(schema["properties"].get("op").is_none(), "{}", spec(kind).id);
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
    let add: Command = serde_json::from_value(serde_json::json!({"op": "add_layer", "name": "ink"})).unwrap();
    assert_eq!(add, Command::AddLayer(AddLayer { name: "ink".into(), above: None }));
    assert!(serde_json::from_value::<Command>(serde_json::json!({"op": "nope"})).is_err());
}

#[test]
fn shortcuts_are_unique() {
    let mut seen = HashSet::new();
    for s in registry() {
        if let Some(k) = s.default_shortcut { assert!(seen.insert(k), "{k} bound twice"); }
    }
}
```

- [ ] **Step 2–5:** Run (fail) → MiMo implements → run (pass) → `git commit -m "feat(ops): command types and registry"`

---

### Task 12: rp-ops engine, undo/redo, transactions, revisions (worktree `m0-ops`)

**Files:** Create `crates/rp-ops/src/{change.rs,engine.rs}`; modify `error.rs`, `lib.rs`. Test: `crates/rp-ops/tests/engine.rs`.

**Types (exact):**

```rust
// error.rs
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpError {
    #[error("no layer {0:?}")]
    UnknownLayer(rp_core::LayerId),
    #[error("no frame {0}")]
    UnknownFrame(u32),
    #[error("layer {0:?} is not an image layer")]
    NotAnImageLayer(rp_core::LayerId),
    #[error("({x}, {y}) is outside the {width}x{height} canvas")]
    OutOfCanvas { x: i32, y: i32, width: u16, height: u16 },
    #[error("a {pixel:?} pixel cannot go in a {sprite:?} sprite")]
    ModeMismatch { sprite: rp_core::ColorMode, pixel: rp_core::ColorMode },
    #[error("palette has {len} colours; index {index} is out of range")]
    PaletteIndex { index: u32, len: usize },
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("canvas changed: you saw revision {expected}, it is now {actual}")]
    StaleRevision { expected: u64, actual: u64 },
    #[error("a transaction is already open")]
    TransactionOpen,
    #[error("no transaction is open")]
    NoTransaction,
}

// engine.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin { Human, Agent { session: String } }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome { pub revision: u64, pub created_layer: Option<rp_core::LayerId> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry { pub label: String, pub origin: Origin }

#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry { pub revision: u64, pub origin: Origin, pub command: Command }

pub struct Engine { /* private */ }

impl Engine {
    /// next layer id = max existing id + 1 (0 if none); revision 0; empty history.
    pub fn new(sprite: rp_core::Sprite) -> Engine;
    pub fn sprite(&self) -> &rp_core::Sprite;
    pub fn revision(&self) -> u64;
    pub fn apply(&mut self, cmd: Command, origin: Origin) -> Result<Outcome, OpError>;
    /// Like apply, but fails with StaleRevision unless revision() == base_revision.
    pub fn apply_at(&mut self, cmd: Command, origin: Origin, base_revision: u64) -> Result<Outcome, OpError>;
    pub fn begin(&mut self, label: &str, origin: Origin) -> Result<(), OpError>;
    pub fn commit(&mut self) -> Result<(), OpError>;
    pub fn rollback(&mut self) -> Result<(), OpError>;
    /// Ok(None) when there is nothing to undo. TransactionOpen while a transaction is open.
    pub fn undo(&mut self) -> Result<Option<String>, OpError>;
    pub fn redo(&mut self) -> Result<Option<String>, OpError>;
    /// Undo stack, oldest first.
    pub fn history(&self) -> Vec<HistoryEntry>;
    /// Every successfully applied command, in order (audit/replay record; not used for undo).
    pub fn log(&self) -> &[LogEntry];
}
```

**Behaviour rules:**
- **Atomicity.** A command that returns `Err` leaves the sprite and the revision exactly as they were. Validate first; mutate only once the command can't fail.
- **Revision.** Increments by 1 for each successful `apply`, `undo` or `redo` that returns `Some`, and for each `rollback` that reverted at least one change. `LogEntry.revision` is the revision *after* that command was applied.
- **History.**
  - An `apply` outside a transaction pushes one history entry labelled with the spec's title, prefixed `"Agent: "` when the origin is `Agent`.
  - Inside a transaction, changes accumulate. `commit` pushes one entry labelled with the `begin` label (with the same prefix rule, using the `begin` origin); committing an empty transaction pushes nothing.
  - `rollback` reverts the accumulated changes and pushes nothing.
  - Any new history entry clears the redo stack.
- **Change model** (`change.rs`). The enum records before/after state, and each variant implements `apply(&mut Sprite)` and `revert(&mut Sprite)`:
  - `Cel { layer, frame, before: Option<Cel>, after: Option<Cel> }`
  - `LayerInsert { index, layer }`
  - `LayerRemove { index, layers: Vec<Layer>, cels: Vec<((LayerId,u32),Cel)> }`
  - `LayerRename { id, before, after }`
  - `FrameAppend { frame }`
  - `FrameDuration { frame, before, after }`
  - `PaletteColor { index, before, after }`

  Whole-cel snapshots are M0's patch granularity. Tile-level patches are a later optimisation behind the same `Change` API (spec §4 asks for tile-level eventually).
- **`set_pixels`.**
  - The layer must exist and be `LayerKind::Image`, the frame must exist, every point must be inside the canvas, and every value's mode must equal the sprite's.
  - An empty `pixels` list is `InvalidArgument`.
  - The target cel is `resolve_cel` (it follows a link, so writing through a linked cel edits the source frame's cel). A new cel is created if there is none.
  - The cel image grows to the bounding-box union of its old bounds and the new points. Old pixels are copied in, and the new area starts zeroed.
  - A new cel has opacity 255 and z-index 0.
- **`add_layer`.**
  - `name` must not be empty, otherwise `InvalidArgument`.
  - New layer: `LayerKind::Image`, flags VISIBLE|EDITABLE, Normal blend, opacity 255. It gets a fresh uuid v4 iff the sprite's header has `LAYER_UUIDS`.
  - With `above: Some(id)` it is inserted at `subtree_range(id).end` with id's parent. With `None` it is pushed at the end with no parent.
  - Returns `created_layer`. Layer ids are never reused in one engine, even after undo.
- **`remove_layer`** removes `subtree_range` and all cels of those layers. Undo restores the exact positions and cels.
- **`rename_layer`:** an empty name is `InvalidArgument`.
- **`add_frame` / `set_frame_duration`:** a duration of 0 is `InvalidArgument`. An unknown frame is `UnknownFrame`.
- **`set_palette_color`:** an index past the end is `PaletteIndex`.
- **Concurrency.** `undo`, `redo` and `apply` all work while a transaction is open, except `undo`/`redo`, which return `TransactionOpen`. `begin` while open is `TransactionOpen`. `commit`/`rollback` with nothing open is `NoTransaction`.

- [ ] **Step 1: Write `crates/rp-ops/tests/engine.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
use rp_core::*;
use rp_ops::*;

const RED: PixelValue = PixelValue::Rgba { r: 255, g: 0, b: 0, a: 255 };
const BLUE: PixelValue = PixelValue::Rgba { r: 0, g: 0, b: 255, a: 255 };
fn agent() -> Origin { Origin::Agent { session: "s1".into() } }
fn engine() -> Engine { Engine::new(Sprite::new(8, 8, ColorMode::Rgba)) }
fn add(e: &mut Engine, name: &str, above: Option<LayerId>) -> LayerId {
    e.apply(Command::AddLayer(AddLayer { name: name.into(), above }), Origin::Human).unwrap().created_layer.unwrap()
}
fn px(e: &mut Engine, layer: LayerId, frame: u32, pts: &[(i32, i32, PixelValue)]) -> Result<Outcome, OpError> {
    e.apply(Command::SetPixels(SetPixels { layer, frame,
        pixels: pts.iter().map(|&(x, y, value)| PixelWrite { x, y, value }).collect() }), Origin::Human)
}
fn image_of(e: &Engine, layer: LayerId, frame: u32) -> (i32, i32, Image) {
    let (_, cel) = e.sprite().resolve_cel(layer, frame).unwrap();
    match &cel.content { CelContent::Image(i) => (cel.x, cel.y, i.clone()), other => panic!("{other:?}") }
}

#[test]
fn add_layer_returns_id_and_bumps_revision() {
    let mut e = engine();
    let a = add(&mut e, "a", None);
    assert_eq!(e.revision(), 1);
    let l = e.sprite().layer(a).unwrap();
    assert_eq!((l.name.as_str(), l.kind, l.opacity, l.parent), ("a", LayerKind::Image, 255, None));
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
    assert_eq!(img.get(0, 2), Some(&[255u8, 0, 0, 255][..]), "old pixel kept at new offset");
    assert_eq!(img.get(3, 0), Some(&[0u8, 0, 255, 255][..]));
    assert_eq!(img.get(1, 1), Some(&[0u8, 0, 0, 0][..]), "new area is transparent");
}

#[test]
fn failed_commands_change_nothing() {
    let mut e = engine();
    let a = add(&mut e, "a", None);
    px(&mut e, a, 0, &[(0, 0, RED)]).unwrap();
    let before = e.sprite().clone();
    let rev = e.revision();
    let cases: Vec<(Result<Outcome, OpError>, &str)> = vec![
        (px(&mut e, a, 0, &[(1, 1, RED), (8, 0, RED)]), "out of canvas"),
        (px(&mut e, a, 0, &[(1, 1, PixelValue::Indexed { index: 1 })]), "mode"),
        (px(&mut e, a, 5, &[(1, 1, RED)]), "frame"),
        (px(&mut e, LayerId(99), 0, &[(1, 1, RED)]), "layer"),
        (px(&mut e, a, 0, &[]), "empty"),
        (e.apply(Command::RenameLayer(RenameLayer { layer: a, name: String::new() }), Origin::Human), "empty name"),
        (e.apply(Command::SetFrameDuration(SetFrameDuration { frame: 0, duration_ms: 0 }), Origin::Human), "zero duration"),
        (e.apply(Command::SetPaletteColor(SetPaletteColor { index: 0, rgba: [0; 4] }), Origin::Human), "palette"),
    ];
    for (r, what) in cases {
        assert!(r.is_err(), "{what} should fail");
        assert_eq!(e.sprite(), &before, "{what} mutated the sprite");
        assert_eq!(e.revision(), rev, "{what} bumped the revision");
    }
    assert!(matches!(px(&mut e, a, 0, &[(9, 9, RED)]), Err(OpError::OutOfCanvas { x: 9, y: 9, width: 8, height: 8 })));
}

#[test]
fn set_pixels_rejects_group_layers() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.layers.push(Layer { id: LayerId(0), parent: None, name: "g".into(), kind: LayerKind::Group,
        flags: LayerFlags(3), blend_mode: BlendMode::Normal, opacity: 255, uuid: None, user_data: UserData::default() });
    let mut e = Engine::new(s);
    assert_eq!(px(&mut e, LayerId(0), 0, &[(0, 0, RED)]), Err(OpError::NotAnImageLayer(LayerId(0))));
}

#[test]
fn writing_through_a_linked_cel_edits_the_source() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.frames.push(Frame { duration_ms: 100 });
    s.layers.push(Layer { id: LayerId(0), parent: None, name: "a".into(), kind: LayerKind::Image,
        flags: LayerFlags(3), blend_mode: BlendMode::Normal, opacity: 255, uuid: None, user_data: UserData::default() });
    let base = Cel { x: 0, y: 0, opacity: 255, z_index: 0, content: CelContent::Image(Image::new(4, 4, ColorMode::Rgba)),
        extra: None, user_data: UserData::default() };
    s.cels.insert((LayerId(0), 0), base.clone());
    s.cels.insert((LayerId(0), 1), Cel { content: CelContent::Linked { frame: 0 }, ..base });
    let mut e = Engine::new(s);
    px(&mut e, LayerId(0), 1, &[(1, 1, RED)]).unwrap();
    assert_eq!(e.sprite().cel(LayerId(0), 1).unwrap().content, CelContent::Linked { frame: 0 });
    assert_eq!(image_of(&e, LayerId(0), 0).2.get(1, 1), Some(&[255u8, 0, 0, 255][..]));
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
    let a = e.apply(Command::AddLayer(AddLayer { name: "a".into(), above: None }), agent()).unwrap().created_layer.unwrap();
    e.apply(Command::SetPixels(SetPixels { layer: a, frame: 0, pixels: vec![PixelWrite { x: 0, y: 0, value: RED }] }), agent()).unwrap();
    assert!(matches!(e.undo(), Err(OpError::TransactionOpen)));
    assert!(matches!(e.begin("x", Origin::Human), Err(OpError::TransactionOpen)));
    e.commit().unwrap();
    assert_eq!(e.history(), vec![HistoryEntry { label: "Agent: outline".into(), origin: agent() }]);
    assert_eq!(e.undo().unwrap(), Some("Agent: outline".into()));
    assert!(e.sprite().layers.is_empty());
}

#[test]
fn single_agent_command_is_labelled() {
    let mut e = engine();
    e.apply(Command::AddLayer(AddLayer { name: "a".into(), above: None }), agent()).unwrap();
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
    let r = e.apply_at(Command::AddLayer(AddLayer { name: "agent".into(), above: None }), agent(), seen);
    assert_eq!(r, Err(OpError::StaleRevision { expected: seen, actual: seen + 1 }));
    assert_eq!(e.sprite().layers.len(), 1);
    assert!(e.apply_at(Command::AddLayer(AddLayer { name: "ok".into(), above: None }), agent(), seen + 1).is_ok());
}

#[test]
fn add_layer_above_goes_after_subtree_with_same_parent() {
    // g { child }, then add "x" above g -> after child, root level.
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    let mk = |id: u32, parent: Option<u32>, kind: LayerKind| Layer { id: LayerId(id), parent: parent.map(LayerId),
        name: format!("{id}"), kind, flags: LayerFlags(3), blend_mode: BlendMode::Normal, opacity: 255, uuid: None,
        user_data: UserData::default() };
    s.layers = vec![mk(0, None, LayerKind::Group), mk(1, Some(0), LayerKind::Image), mk(2, None, LayerKind::Image)];
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
    s.layers.insert(0, Layer { id: LayerId(50), parent: None, name: "g".into(), kind: LayerKind::Group,
        flags: LayerFlags(3), blend_mode: BlendMode::Normal, opacity: 255, uuid: None, user_data: UserData::default() });
    s.layers[1].parent = Some(LayerId(50));
    let mut e = Engine::new(s);
    let before = e.sprite().clone();
    e.apply(Command::RemoveLayer(RemoveLayer { layer: LayerId(50) }), Origin::Human).unwrap();
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
    s.palette.entries = vec![PaletteEntry { rgba: [0, 0, 0, 255], name: None }];
    let mut e = Engine::new(s);
    e.apply(Command::AddFrame(AddFrame { duration_ms: 50 }), Origin::Human).unwrap();
    assert_eq!(e.sprite().frames, vec![Frame { duration_ms: 100 }, Frame { duration_ms: 50 }]);
    e.apply(Command::SetFrameDuration(SetFrameDuration { frame: 1, duration_ms: 70 }), Origin::Human).unwrap();
    assert_eq!(e.sprite().frames[1].duration_ms, 70);
    assert!(matches!(e.apply(Command::SetFrameDuration(SetFrameDuration { frame: 9, duration_ms: 70 }), Origin::Human),
                     Err(OpError::UnknownFrame(9))));
    e.apply(Command::SetPaletteColor(SetPaletteColor { index: 0, rgba: [1, 2, 3, 4] }), Origin::Human).unwrap();
    assert_eq!(e.sprite().palette.entries[0].rgba, [1, 2, 3, 4]);
    let a = add(&mut e, "a", None);
    e.apply(Command::RenameLayer(RenameLayer { layer: a, name: "ink".into() }), Origin::Human).unwrap();
    assert_eq!(e.sprite().layer(a).unwrap().name, "ink");
    for _ in 0..5 { e.undo().unwrap(); }
    assert_eq!(e.sprite().frames.len(), 1);
    assert_eq!(e.sprite().palette.entries[0].rgba, [0, 0, 0, 255]);
}

#[test]
fn log_records_every_applied_command() {
    let mut e = engine();
    add(&mut e, "a", None);
    let _ = px(&mut e, LayerId(77), 0, &[(0, 0, RED)]); // fails: not logged
    e.apply(Command::AddFrame(AddFrame { duration_ms: 10 }), agent()).unwrap();
    let log = e.log();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].origin, Origin::Human);
    assert_eq!(log[1].origin, agent());
    assert_eq!(log[1].revision, 2);
    assert!(matches!(log[1].command, Command::AddFrame(_)));
}
```

- [ ] **Step 2–5:** Run (fail) → MiMo implements → run (pass) → `git commit -m "feat(ops): engine, undo/redo, transactions, revisions"`
- [ ] **Step 6: Merge.** In the main checkout: `git -C /home/nate/Projects/rusty_pixels merge --no-ff m0-ops -m "merge: rp-ops stream"`. Then run `bash .claude/verify.sh` and remove the worktree.

---

### Task 13: M0 gate

- [x] **Step 1:** `bash .claude/verify.sh`. Expected: exit 0.
- [x] **Step 2:** `bash tools/corpus.sh`. Expected: zero non-OK files.
- [x] **Step 3:** `cargo test --workspace` and quote the totals in the report.
- [x] **Step 4: Independent review.** The `reviewer` agent (Opus, not MiMo) reviews the whole M0 diff for correctness bugs, panics on hostile input, clean-room hygiene and spec drift. It reports every finding with confidence and severity. Fixes go back to MiMo; re-review stops when severity has decayed (see the adversarial-review memory).
- [x] **Step 5:** Update `CLEANROOM.md` with any new sources, and check `NOTICE` covers anything ported.
- [ ] **Step 6: Ask Nate** before creating the GitHub repo and pushing. Pushing is outward-facing.

---

## Spec coverage check (M0 rows of spec §6)

| Spec requirement | Task |
|---|---|
| Cargo workspace | 1 |
| rp-core document model (all listed entities) | 2 |
| rp-ops registry: id, title, docs, schema, shortcut, menu path, undoable, agent-exposed | 11 |
| Registry parity test | 11 (every kind ↔ one spec; schemas). The menu/shortcut ↔ GUI half of the parity test needs a GUI and lands in M2 |
| Undo, transactions, single history, origin labels, revision checks | 12 |
| Command log as audit/replay record | 12 (`log()`) |
| rp-io .aseprite read/write, lossless per spec definition | 3–8 (model), 9 (Aseprite fixtures), 10 (Aseprite renders) |
| Core crates portable, no filesystem in rp-io | 3 (bytes-only API) + Android check in 1 |
| CI on 3 OS + Android check | 1 |
| Clean-room log, licences, NOTICE | 1, 13 |
| Tile-level undo patches | **Deferred** to the first milestone with real brush strokes (M2); M0 uses whole-cel snapshots behind the same `Change` API |
| `Storage` trait | **Deferred** to M1, the first milestone with file access (rp-io takes bytes, so M0 needs none) |

## Review findings carried forward

Status as of 2026-10-12, after the Task 13 independent review (Opus `reviewer` agent; its proof tests sat in `/tmp/rp-review`, outside the repo).

### Fixed in M0

| Finding | Fix | Test |
|---|---|---|
| H1: a tileset with a huge `tile_count` and no tiles made `write()` emit one user-data chunk per tile (a 197-byte input became 30 MB; a 3-byte mutation hit a 1.17 GB abort) | The writer emits only existing `tile_user_data`; counters use checked arithmetic | `rp-io/tests/review_fixes.rs` |
| H2, M1 and Task 4 zlib cap: tileset, tilemap and cel decompression bombs | One `DECODE_BUDGET` (1 GiB) per file, charged with declared sizes *before* inflating, plus `OBJECT_OVERHEAD` per tile | same |
| M2: old palette packets were unbounded | An index of 256 or more is an error | same |
| M5: unknown chunks were dropped silently | Kept verbatim in `Sprite.raw_chunks` and re-emitted at the end of their frame, with any user data that follows them | same |
| M6: palettes in later frames collapsed into frame 0 | Palette chunks after frame 0 are kept raw in their frame | same |
| M7: layer handling was quadratic (80k layers took 8.7 s to write) | Direct indexing on read; one id→index map on write | same (30k layers, under 8 s in a debug build) |
| L1 a–f, L3, L4, L11, L12: reader/writer asymmetries and test gaps | Each now rejected or fixed, and pinned by a test | same |
| M4 (it was rated "Low/low" above as "set_pixels 65535²"; the real trigger is a far off-canvas cel on any canvas) | Growth cap: the union area may not exceed canvas area plus the old cel's area | `rp-ops/tests/review_fixes.rs` |
| L5: layer ids saturated and were reused | `next_layer_id: Option<u32>`; exhaustion is an error | same |
| L8: a zero-size old cel widened the bounds | A zero-area image contributes no bounds | same |
| L18: copied preference types and defaults plus personal notes in `research/`; an incomplete clean-room log | Types and defaults stripped; personal notes moved to the git-ignored `.private/`; `CLEANROOM.md` and `NOTICE` completed | (docs) |
| M8: the corpus check saw only visible pixels | Task 13c: `--all-layers` renders, an Aseprite metadata diff, and per-file errors | `rp-corpus` unit tests |

### Carried forward

| Found | Severity / confidence | Issue | Fix by |
|---|---|---|---|
| Review M3 | **High for M1** / high | Transactions are global, not per origin. An agent's `rollback` reverts the human's concurrent edits, and on `commit` the human's stroke is labelled as the agent's. This contradicts spec §4/§5. | M1, first task: give each origin its own transaction, record the origin on every change, revert only your own changes, and detect conflicts by revision |
| Task 12 review | Medium / high | `Engine::log()` is an audit trail, not a replay record: it has no undo, redo or rollback events | M1, before anything replays |
| Review L14 | Medium / high | `Uuid::new_v4()` is generated inside the engine and isn't carried by the command, so replay isn't deterministic | M1, with the log fix: put the UUID in the logged command |
| Task 12 review | Low / medium | Pop-before-revert in undo/redo/rollback. The reviewer's randomised run never triggered it | M1 |
| Review L15 | Medium / high | `OpSpec.undoable`/`agent_exposed` are never consulted; command JSON accepts unknown fields (an agent typo is silently ignored) | M1 (the MCP surface): enforce the flags; use `deny_unknown_fields`, checking it works with the internally tagged enum |
| Review L16 | Medium / high | There is no command to grow a palette, so `set_palette_color` can't succeed on a new sprite; indexed `set_pixels` isn't checked against the palette length | M1 command set |
| Review L6 | Low / high | Layer ids are renumbered on save and reload, so an agent's cached ids go stale | M1: document it; consider persisting ids via layer UUIDs |
| Review L7 | Low / high | The engine accepts states the writer can't store (cel x beyond i16, more than 65535 frames, names over 65535 bytes) | M1: validate against format limits in the engine |
| Review L13 | Low / medium | Aseprite shares opacity (and probably position) across linked cels; our model stores them per link, so a link's x/y can go stale after `set_pixels` through it | M2 (linked-cel editing) |
| Review L17 | Low / high | Undo history is unbounded and stores two whole-cel snapshots per edit | M2: tile-level patches plus a memory cap (Aseprite has an undo-size preference) |
| Review L9 | Low / high | `Image::new` panics on a size overflow | M1: make it fallible or saturate; no untrusted caller exists today |
| Review L10 | Low / low | The 0x0011 6-bit palette scaling (`v*255/63`) is unverified against Aseprite | Only if a real file using 0x0011 turns up |
| Task 4 review | Low / medium | Linked cels aren't validated on read. The corpus showed only valid links | M1 |
| Review L2 | Low / high | Chunks past a frame's declared count, and bytes after the last frame, are ignored | M1: keep as raw data or reject |
| Git history | Medium / high | Commits before `ecb5a2b` still contain the copied preference dump and personal notes | Before the first public push: squash or rewrite history (needs the owner's decision) |

### Second review pass (2026-10-12)

This pass reviewed only the fix code. Its behaviour claims about Aseprite were re-established black-box before any fix (see the `CLEANROOM.md` incident entry).

Fixed in Tasks 13d and 13e:
- **R1:** tile user data is padded to `tile_count` when tiles are in the file. Observed: otherwise Aseprite warns "Unexpected chunk type".
- **R2:** unknown chunks don't redirect user data. Observed.
- **R9:** user data at the start of frame 0 goes to the sprite, and sprite user data no longer needs a palette. Observed.
- **R13:** "anything readable is writable" is now a test invariant.
- **R4 and R4b:** frame emission is linear.
- **R8:** inflate buffers are reserved exactly.
- **O1:** the area cap rejected normal painting. Replaced by `MAX_CEL_BYTES` plus the format limits.
- **C1:** `first_diff` is exhaustive, including `raw_chunks`.
- **C2:** the corpus reads Aseprite's stdout diagnostics. Observed: load warnings go to stdout and the exit code is 0.
- **C3:** the metadata export uses `--all-layers`. Observed: without it, hidden layers are omitted.
- **C4:** a run where every original fails exits 1.

| Found | Severity / confidence | Issue | Fix by |
|---|---|---|---|
| Review 2 R5 | Low / medium | A second user-data chunk for one owner is rejected; Aseprite reportedly overwrites. Not observed black-box yet | M1: craft a file, observe, match |
| Review 2 R6 | Low / medium | Out-of-range tags are rejected on read; Aseprite reportedly clamps on save. Affects files from other tools only | M1: observe, then clamp or reject |
| Review 2 R10 | Low / medium | The old-palette reset when a 0x2019 chunk is present may differ from Aseprite (layering vs replacing). Not observed | M1: observe with a crafted file |
| Review 2 R7 | Medium / high | `DECODE_BUDGET` (1 GiB) rejects legitimately huge files (e.g. 300 full-HD RGBA cels) and has no override | M1: make the budget a read option with a safe default |
| Review 2 R3, R11 | Low / medium | Raw chunks lose their position within a frame. The writer accepts model-built raw chunks of kinds the reader decodes | M1 |
| Review 2 R12 | Low / high | Raw chunks aren't charged to the decode budget (about 10× model amplification, linear in input) | M1 |
| Review 2 O2 | Low / high | `MAX_CEL_BYTES` bounds one cel; there is no total memory budget across many edits | M2, with undo memory caps |
| Review 2 C5 | Low / medium | Running `rp-corpus` directly with a reused `--out` can compare stale outputs; `corpus.sh` clears it | M1: clear or refuse a non-empty `--out` |

### Gate result (2026-10-12)

- `verify.sh` exits 0: fmt, clippy `-D warnings`, the Android check, and 137 tests passed with 0 failed.
- `corpus.sh` exits 0: 194 files OK and no other bucket.
- The merge of 13d and 13e needed three repairs:
  - `orphan_user_data_is_an_error` still expected an error for user data at the start of frame 0. R9 made that the sprite's user data. The test now pins the remaining ownerless case: a later frame keeps the chunk raw.
  - The 13e worker opted rp-ops out of the workspace lints to silence `assertions_on_constants`. It is back on the workspace lints, and the test uses a `const { assert!(..) }` block.
  - `corpus.sh` scanned `target/` in our worktrees and picked up review probe files. It now excludes them.
- **The review loop stops here.** Pass 1 found High defects. Pass 2 reviewed only fix code, and its worst finding was Medium. The only unreviewed surface is the 13d/13e fix code, which the gate and the corpus cover. Open items are in the tables above, each with a target milestone.
