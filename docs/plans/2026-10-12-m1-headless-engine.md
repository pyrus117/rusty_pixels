# M1 Headless Engine Implementation Plan

**Goal (spec §6 M1):** an agent creates, draws, animates and exports with no GUI, through `rusty_pixels --headless-mcp`, and a batch CLI covers the core export flags. This replaces the batch-file Aseprite MCP.

**Execution model:** as in M0 (`docs/plans/2026-10-11-m0-foundations.md`, "Execution model" and "MiMo prompt template"), with one change. **The session model commits each task's test files to the task branch before MiMo starts**, instead of pasting test code into the plan. The plan states the behaviour; the committed tests are the contract. MiMo may not edit them.

- Worktrees live at `/home/nate/Projects/rusty_pixels-<branch>`.
- No more than two MiMo processes run at once.
- Review after each task: verify.sh, read the diff, check it against the behaviour rules, then merge with `--no-ff`.

**Phases.** Phase A's three streams are independent and touch different crates. Phases B to D are planned in detail once A merges, because they build on A's APIs.

| Phase | Stream | Branch | Crates |
|---|---|---|---|
| A | A1 engine semantics | `m1-ops` | rp-ops (and `Image::new` in rp-core) |
| A | A2 compositor | `m1-render` | new `rp-render` |
| A | A3 export encoders | `m1-export` | rp-io `export` module |
| B | B1 drawing and animation commands | `m1-cmds` | new `rp-tools`, rp-ops |
| B | B2 documents and storage | `m1-doc` | rp-ops or new `rp-doc`, `Storage` trait |
| C | C1 MCP server | `m1-mcp` | new `rp-mcp`, binary `rusty_pixels` |
| D | D1 batch CLI | `m1-cli` | new `rp-cli` |
| D | M1 gate | main | review, corpus, an end-to-end agent session |

Shared prerequisite, done on main before Phase A: `rp_core::RgbaImage` (flattened straight-alpha RGBA8, `MAX_RGBA_BYTES` cap) is the type that A2 produces and A3 consumes.

---

## Black-box observations used by this plan (Aseprite 1.3.18.6, 2026-10-12)

Each one comes from running the installed Aseprite on files we built ourselves:
- `--save-as name_{frame}.png` numbers frames from 0, and a single-frame sprite still gets `_0`.
- GIF frame delay is `floor(duration_ms / 10)` centiseconds. Observed: 100→10, 55→5, 56→5, 33→3, 19→1, 5→0.
- In a GIF, a pixel is transparent only when its alpha is 0. Any alpha above 0 (2, 127, 129 and 254 were seen) becomes opaque with its RGB unchanged.
- The sprite-sheet JSON layouts are recorded in A3 below. Frame names default to `"<title> <frame>.aseprite"`, where title is the file stem. `--list-tags` adds `meta.frameTags`, with direction as a lowercase word and color as `#rrggbbaa`. `--list-layers` adds `meta.layers` (name, opacity, blendMode).
- Batch-mode Aseprite prints X11 "Authorization required" on stderr. It is noise.

---

## A1: engine semantics (`m1-ops`)

Fixes the carried-forward findings that block an MCP surface: M3 (High), the replay log, L14, pop-before-revert, L15, L16, L7 and L9. The tests are in `crates/rp-ops/tests/m1_engine.rs`. Existing tests that encode the old global-transaction API are updated on the branch, and each change carries its reason in the commit message.

**Behaviour rules**
1. **One open transaction per origin.** The calls are `begin(label, origin)`, `commit(&origin)` and `rollback(&origin)`. `TransactionOpen` means *this* origin already has one open. `NoTransaction` means this origin has none. Origins are compared by equality, so `Agent{session:"a"}` and `Agent{session:"b"}` are different.
2. `apply(cmd, origin)` puts the change into that origin's open transaction if it has one. Otherwise the change becomes its own history entry. **Other origins' transactions do not capture it.**
3. **Commit** pushes one history entry labelled with the transaction's own origin (an `"Agent: "` prefix for agents). An empty transaction adds nothing.
4. **Rollback** reverts only that origin's changes, newest first. It **fails with `OpError::Conflict(String)`, changing nothing and leaving the transaction open**, if any change by someone else applied after one of the transaction's changes, and still applied, *touches* something that transaction change wrote.
5. **Undo and redo are allowed while transactions are open.** `TransactionOpen` is no longer returned from them. Undo fails with `Conflict`, changing nothing, if a still-applied change made after the entry touches what the entry wrote. That later change must come from an open transaction, because anything in the history above it would have been undone first. Redo fails with `Conflict` if anything applied after the undo, and still applied, touches what the entry writes. A change that was later rolled back no longer counts.
6. **"Touches"** means reads or writes, using these keys. Implement them as you like; the tests check the behaviour.
   - `set_pixels(L, f)`: writes cel (L, resolved frame f); reads layer L's existence and frame f's existence.
   - `add_layer`: writes the layer order and the new layer's existence and properties.
   - `remove_layer`: writes the layer order, plus the existence and properties of every removed layer and every removed cel.
   - `rename_layer` (and later property setters): writes layer L's properties; reads L's existence.
   - `add_frame`: writes the new frame's existence. `set_frame_duration`: writes that frame's properties; reads its existence.
   - `set_palette_color(i)`: writes palette entry i. `resize_palette`: writes the palette length and every entry it adds or drops.

   For example, an agent transaction that adds a layer can still be rolled back after the human paints on a *different* layer. It cannot be rolled back after the human paints on the agent's new layer.
7. **Undo, redo and rollback are atomic.** If a revert or apply fails partway, the sprite, both stacks and the transaction are exactly as they were before the call, and the error is returned. Any pop happens only after success.
8. **The log is a replay record.** `log()` returns `&[LogEntry]`, and `LogEntry` becomes an enum, `Serialize + Deserialize`:
   - `Apply { revision, origin, command }`
   - `Undo { revision }`, `Redo { revision }`
   - `Begin { origin, label }`, `Commit { origin }`, `Rollback { revision, origin }`

   Only successful calls are logged. `Origin` gets `Serialize/Deserialize`, with a snake_case `kind` tag: `{"kind":"human"}`, `{"kind":"agent","session":"s1"}`. `Engine::replay(initial: Sprite, log: &[LogEntry]) -> Result<Engine, OpError>` re-runs a log and produces an engine whose sprite, revision and history equal the original's.
9. **Deterministic UUIDs (L14).** `AddLayer` gains `#[serde(default)] uuid: Option<[u8;16]>`. When the sprite has `LAYER_UUIDS` and the field is `None`, the engine generates one and **logs the command with that UUID filled in**. When the field is given, it is used as is. When the sprite has no `LAYER_UUIDS` flag, a given uuid is an `InvalidArgument`.
10. **Strict command JSON (L15).** Every parameter struct is `#[serde(deny_unknown_fields)]`. A misspelt field is an error, and the `"op"` tag still works.
11. **Registry flags (L15).** There is a new public function, `pub fn permitted(spec: &OpSpec, origin: &Origin) -> Result<(), OpError>`, which returns `OpError::NotAgentExposed(spec.id)` for an `Agent` origin when `agent_exposed` is false. `apply` calls it before doing anything else. A command with `undoable: false` is applied and logged, but never enters history or a transaction. No such command exists yet, so that path is reviewed by reading the code rather than tested.
12. **Palette growth (L16).** There is a new command, `ResizePalette { len: u32 }`, op `resize_palette`, titled "Palette Size", menu `None`, undoable, agent-exposed.
    - `len` must be 1 to 256.
    - Growing appends `[0,0,0,255]` entries with no name. Shrinking drops entries from the end.
    - In an indexed sprite, `set_pixels` rejects an index ≥ the palette length with `PaletteIndex`, unless that index is `transparent_index`, which is always allowed.
13. **Format limits (L7).** `add_frame` fails when there are already 65535 frames. Layer names over 65535 bytes fail on `add_layer` and `rename_layer`. All of these are `InvalidArgument`.
14. **L9.** `Image::new` returns `Result<Image, CoreError>`, with `TooLarge` when the byte size overflows or exceeds `rp_core::MAX_RGBA_BYTES`. Update every caller, including tests in rp-core, rp-io, rp-ops and rp-corpus. This is the only change allowed to existing test files outside rp-ops: `Image::new(..)` becomes `Image::new(..).unwrap()`.

Commit message: `feat(ops): per-origin transactions, replay log, strict commands, palette resize`.

---

## A2: compositor (`m1-render`)

A new crate, `crates/rp-render`, portable (no filesystem), depending on rp-core only. The oracle is in `crates/rp-render/tests/fixtures/`, generated by `tools/fixtures/render_oracle.sh`: Aseprite-built sprites plus Aseprite's own flattened `<name>_<frame>.png`. The test is `crates/rp-render/tests/oracle.rs`, plus unit cases in `crates/rp-render/tests/render.rs`.

**API**
```rust
pub enum LayerFilter { Visible, All, Only(Vec<LayerId>) }
pub struct RenderOptions { pub layers: LayerFilter }   // Default: Visible
pub fn render_frame(sprite: &Sprite, frame: u32, opts: &RenderOptions) -> Result<RgbaImage, RenderError>;
pub fn scale_nearest(img: &RgbaImage, factor: u32) -> Result<RgbaImage, RenderError>;
#[derive(Debug, thiserror::Error)] pub enum RenderError {
    UnknownFrame(u32), UnknownLayer(LayerId), BadCel { layer: LayerId, frame: u32, reason: String },
    Image(#[from] rp_core::CoreError), BadFactor(u32) }
```

**Behaviour rules**
1. The output is canvas-sized, starts fully transparent, and draws layers bottom to top (`sprite.layers` order). Cels are clipped to the canvas, and cel positions may be negative or past any edge.
2. **Visibility.** `Visible` draws a layer only if it and every ancestor are visible. `All` ignores visibility flags. `Only(ids)` draws only layers that are in `ids` or have an ancestor in `ids`. It ignores visibility for those layers, but the group opacity and blend of their ancestors still apply. An id that doesn't exist is `UnknownLayer`.
3. **Groups** composite their children into their own buffer, which is then blended onto what lies below with the group's blend mode and opacity. Use the oracle to settle any detail of this.
4. **Opacity.** The effective opacity combines layer opacity and cel opacity, and pixel alpha on top of that. Layer opacity is used only when the header has `LAYER_OPACITY_VALID`; otherwise it counts as 255. Group blend and opacity are used only with `GROUP_BLEND_VALID`; otherwise a group acts as Normal at 255. Match the oracle's rounding exactly.
5. **All 19 blend modes** must match the oracle pixel for pixel, on both "full" and "op" variants. Reference formulas: the W3C Compositing and Blending Level 1 spec (public), and Pixelorama's MIT-licensed blend code if needed. Attribute anything ported in `NOTICE`, and log the source in `CLEANROOM.md`. **Never consult Aseprite source.** The oracle PNGs are the ground truth.
6. **Colour modes.** Grayscale pixels (v, a) become (v, v, v, a). Indexed pixels look up the palette, and `transparent_index` is fully transparent. An index past the palette is transparent too, never a panic.
7. **Linked cels** render their target's content, through `Sprite::resolve_cel`, at the linking cel's position and opacity if they differ. Use the oracle. **Tilemap cels** render each tile from the layer's tileset at grid positions, honouring the flip masks. Tile 0 is empty. A tile id past the tileset is `BadCel`.
8. A frame with no cels renders fully transparent. A frame index ≥ the frame count is `UnknownFrame`.
9. `scale_nearest`: factor 1 to 64, otherwise `BadFactor`. Each pixel becomes a factor×factor block. Output larger than `MAX_RGBA_BYTES` is an `Image` error.
10. **No panics on hostile sprites.** That covers corrupt cel images (wrong data length), parent cycles and missing tilesets: return `BadCel` or skip, as the tests say.

**Comparison rule in the oracle test:** two pixels are equal if both have alpha 0, or if all four channels match. The test prints the file, frame, mismatch count and first mismatch for every failing case.

Commit message: `feat(render): CPU compositor matching the Aseprite oracle`.

---

## A3: export encoders (`m1-export`)

These are additions to `rp-io`. They stay bytes-only, with no filesystem, as a new `export` module re-exported from the crate root. Dependency: `gif = "0.14"` in the workspace. The tests are in `crates/rp-io/tests/export.rs`.

**API**
```rust
pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, ExportError>;              // RGBA8 PNG
pub struct GifFrame<'a> { pub image: &'a RgbaImage, pub duration_ms: u16 }
pub fn encode_gif(frames: &[GifFrame], looping: bool) -> Result<Vec<u8>, ExportError>;
pub enum SheetType { Horizontal, Vertical, Rows { columns: u32 }, Columns { rows: u32 } }
pub struct SheetFrame<'a> { pub image: &'a RgbaImage, pub name: String, pub duration_ms: u16 }
pub struct Sheet { pub image: RgbaImage, pub rects: Vec<SheetRect> }
pub struct SheetRect { pub x: u32, pub y: u32, pub w: u32, pub h: u32 }
pub fn layout_sheet(frames: &[SheetFrame], ty: SheetType) -> Result<Sheet, ExportError>;
pub enum JsonFormat { Hash, Array }
pub struct SheetMeta<'a> { pub image_name: &'a str, pub tags: &'a [rp_core::Tag], pub layers: Option<&'a [rp_core::Layer]> }
pub fn sheet_json(frames: &[SheetFrame], sheet: &Sheet, format: JsonFormat, meta: &SheetMeta) -> Result<String, ExportError>;
#[derive(Debug, thiserror::Error)] pub enum ExportError { Empty, MixedSizes, BadLayout(String), Encode(String), Image(#[from] rp_core::CoreError) }
```

**Behaviour rules**
1. `encode_png` writes 8-bit RGBA, non-interlaced, and decodes back to the same bytes.
2. `encode_gif`:
   - Canvas size is the frames' size. Different sizes give `MixedSizes`; an empty slice gives `Empty`. Sizes over 65535 give `Encode`.
   - Each frame's delay is `duration_ms / 10` (integer division, as observed).
   - Alpha 0 is transparent; any other alpha is opaque with its RGB unchanged (observed).
   - If a frame has 255 or fewer distinct opaque colours, the palette is exact and the frame decodes losslessly under that rule. Otherwise quantise; the gif crate's built-in quantiser is fine. That case is only tested for not failing.
   - `looping` writes infinite repeat. Frames must composite correctly when decoded in sequence, so use a disposal method that makes each frame stand alone.
3. `layout_sheet`:
   - All frames must be the same size (`MixedSizes`), and there must be at least one (`Empty`).
   - Horizontal: one row. Vertical: one column.
   - `Rows { columns }` fills left to right, then wraps. `Columns { rows }` fills top to bottom, then wraps. Both use ceil for the other dimension.
   - A count above the frame count is clamped to the frame count (observed: 3 frames with `--sheet-columns 5` give a 3-wide sheet).
   - `SheetFrame` derives `Clone`.
   - `columns` or `rows` of 0 is `BadLayout`. Unused cells are transparent. There is no padding.
4. `sheet_json` matches the observed Aseprite layout, key for key and in this order.
   - Each frame: `filename` (Array only; in Hash the name is the key), `frame {x,y,w,h}`, `rotated: false`, `trimmed: false`, `spriteSourceSize {x:0,y:0,w,h}`, `sourceSize {w,h}`, `duration`.
   - `meta`: `app` is `"https://github.com/pyrus117/rusty_pixels"`, `version` is the rp-io crate version, then `image`, `format: "RGBA8888"`, `size {w,h}`, `scale: "1"`.
   - `frameTags` appears only if `tags` is non-empty. Each tag is `{name, from, to, direction}`, with direction `forward|reverse|pingpong|pingpong_reverse`.
   - `layers` appears only if it is `Some`, listing the given layers in order (the caller drops hidden layers, as Aseprite does without `--all-layers`).
     - An image layer is `{name, group?, opacity, blendMode}`. A group layer is `{name, group?}`.
     - `group` is the parent's name, and appears only when the parent is in the given slice.
     - blendMode names were observed: `normal multiply screen overlay darken lighten color_dodge color_burn hard_light soft_light difference exclusion hsl_hue hsl_saturation hsl_color hsl_luminosity addition subtract divide`.
   - Tag `color` and `repeat` are left out in M1.
   - The tests parse the JSON and check structure and key order. Whitespace is free.
   - serde_json's default `Map` sorts keys, so write the JSON in order: either by hand, using `serde_json::to_string` for string escaping, or with ordered serialisation that doesn't turn on `preserve_order` for the whole workspace.
5. Observed for D1: a single-frame sprite's frame is named `"<title>.aseprite"` with no number.

Commit message: `feat(io): PNG, GIF and sprite-sheet export`.

---

## B1a: drawing geometry (`m1-tools`)

A new crate, `crates/rp-tools`, that is pure geometry with no dependencies. It decides which pixels a stroke covers; rp-ops turns those pixels into cel changes in B1c. The oracle is `crates/rp-tools/tests/fixtures/`, made by `tools/fixtures/make_tool_fixtures.lua` with the real Aseprite: 23 point pairs × 5 shape tools, plus 6 paint-bucket fills on a maze. The tests are `tests/oracle.rs` and `tests/tools.rs`.

**API**
```rust
pub fn line(x0: i32, y0: i32, x1: i32, y1: i32, bounds: (u32, u32)) -> Vec<(i32, i32)>;
pub fn rectangle(x0: i32, y0: i32, x1: i32, y1: i32, filled: bool, bounds: (u32, u32)) -> Vec<(i32, i32)>;
pub fn ellipse(x0: i32, y0: i32, x1: i32, y1: i32, filled: bool, bounds: (u32, u32)) -> Vec<(i32, i32)>;
pub enum Connectivity { Four, Eight }
pub fn flood_fill(seed: (u32, u32), bounds: (u32, u32), connectivity: Connectivity,
                  same: impl Fn(u32, u32) -> bool) -> Vec<(u32, u32)>;
```

**Behaviour rules**
1. Shapes use inclusive corner points, and corner order doesn't matter for rectangles and ellipses. A line is drawn from point 0 to point 1, and Aseprite's tie-breaking is whatever the oracle shows.
2. Results contain only points inside `[0,w) × [0,h)`, each point once, in any order.
3. **The work scales with the bounds, not the shape.** Clip analytically; never walk a 120,000-pixel edge to keep 64 points. Any i32 coordinate is valid input: use i64/i128 internally so nothing overflows, and never panic.
4. `flood_fill` returns the region connected to the seed in which `same(x, y)` holds. A seed outside the bounds, or one that fails `same`, gives an empty result. It must not recurse (use an explicit stack or scanlines) and must take linear time. Aseprite's paint bucket default is 4-connected (observed).

Commit message: `feat(tools): drawing geometry matching the Aseprite oracle`.

---

## Phases B to D (outline, detailed after Phase A merges)

- **B1 commands.** Covers: layer visibility, opacity and blend; add group; move layer; insert, remove and duplicate frame (re-keying cels); add, remove and set tag; clear cel; link cels; set cel opacity; and drawing as commands built on `rp-tools` (line with Bresenham, rectangle, ellipse, flood fill in 4- or 8-connected modes, each with a fill option). Every drawing command produces an ordinary cel change, so undo and conflict keys come free.
- **B2 documents.**
  - A `Storage` trait (read, write, rename, list, all bytes and paths) with a desktop implementation in the binary.
  - `Document { engine, path, dirty }`, with open, save and save-as through rp-io.
  - Before any overwrite, the previous file is copied to `.rusty_pixels/history/` beside it. The last 10 are kept, and both the count and the feature are configurable.
  - The `DECODE_BUDGET` read option (R7).
- **C1 MCP.**
  - Streamable HTTP via rmcp 3.5 with a manual `ServerHandler`. The tool list is built from the registry and grouped to roughly 40–60 tools.
  - A flat object root schema for each tool: no root `oneOf` (Claude Code rejects it).
  - 127.0.0.1 only. Bearer token in `$XDG_RUNTIME_DIR/rusty_pixels/token`, falling back to the temp dir. rmcp's `allowed_hosts` and `allowed_origins` are set explicitly.
  - Read tools: `render`, `get_pixels`, `describe_document`. Transactions: begin, commit, rollback. Undo and redo.
  - `rusty_pixels --headless-mcp`, plus a test client that calls every agent-exposed op and checks the result, undo and redo (the MCP parity test).
- **D1 CLI.** `rusty_pixels -b` with Aseprite's flag names for: `--save-as` (png, gif, aseprite), `--sheet`, `--data`, `--format json-hash|json-array`, `--sheet-type`, `--sheet-columns`, `--sheet-rows`, `--list-tags`, `--list-layers`, `--frame-range`, `--layer`, `--all-layers`, `--scale`. The tests compare against Aseprite CLI output on our fixtures.
- **M1 gate.** verify.sh; corpus.sh; Opus review with the clean-room prompt; and an end-to-end session in which a real agent connected to `--headless-mcp` draws and animates a sprite, exports a GIF and sheet, and the files open correctly in Aseprite.
