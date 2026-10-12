# Rusty Pixels — design spec

Status: approved 2026-10-11. M0 (foundations) is in progress; see `docs/plans/2026-10-11-m0-foundations.md`. All decisions are recorded in §8.
Names: the display name is **Rusty Pixels**. The binary and top-level crate are `rusty_pixels`, and the internal crates are `rp-*`.

## 1. Goal

A desktop pixel-art editor in Rust that covers Aseprite's functionality and workflows, where **every action a human can take is also an MCP tool**, and the agent and the human edit the *same live document* at the same time.

The second half is the real reason to build this rather than keep using Aseprite. Existing Aseprite MCP servers (the one used here has 116 tools) drive Aseprite through batch processes on *saved files*. It cannot see unsaved work, cannot share an undo history with you, and cannot show you what it's doing while it does it.

## 2. Clean-room rule

Aseprite's EULA §2(c) prohibits reverse engineering, decompiling and disassembling, and §2(g) limits the source code to personal builds and upstream contributions. So:

| Source | Use |
|---|---|
| Aseprite binary, Aseprite source | **Never read, never port.** |
| Shipped data files (`gui.xml`, `pref.xml`, `strings/en.ini`, `widgets/*.xml`) | Read for the feature inventory. Names and options only; never copied into the project. |
| `.aseprite` file-format spec | Implement it. It is published so other tools can read and write the format. |
| Lua API docs (MIT) | Template for the MCP surface. |
| Running the installed Aseprite (GUI or CLI) | Yes. This is normal use, and it produces pixel-exact reference outputs (§7). |
| Pixelorama (MIT) | Read and port algorithms, with attribution in `NOTICE`. |
| LibreSprite (GPL-2) | Run it to observe behaviour. Don't read its source, because the project is MIT/Apache. |

Features and workflows aren't copyrightable. Code, artwork, text, and the Aseprite name and logo are. `CLEANROOM.md` logs every source consulted, starting with the three research files below.

Research, all in `research/`:
- `feature-inventory.md`: 26 tools, 169 commands plus 25 more found only in the strings file, 344 preferences, 61 dialogs, 19 blend modes, 57 CLI flags.
- `formats-and-api.md`: the full chunk-level `.aseprite` spec, the Lua API object model, the CLI, licensing, and existing Rust crates.
- `rust-ecosystem.md`: GUI, rendering and MCP options, with versions checked 2026-10-11.

### 2a. Look and feel: as close as is safe

Licence: **MIT/Apache-2.0, public on GitHub.**

**Match freely.** These are functional, and an Aseprite user should feel at home:
- Panel arrangement: tools on the left, timeline along the bottom, palette and colour bar beside the canvas, and the same docking behaviour.
- Workflows and the order of steps.
- Default keyboard shortcuts.
- Menu structure and command names.
- Option names in tool bars and dialogs.
- Timeline conventions such as cel dots and link markers.
- A chunky, pixel-art styled UI drawn at integer scale. That style is a genre, not Aseprite's property.

**Make our own:**
- Every icon, drawn fresh rather than traced from Aseprite's.
- The theme sprite sheet.
- The UI font: an SIL OFL pixel font, not Aseprite's.
- The colour scheme, which should be clearly different from Aseprite's default greys and blues.
- The logo and splash screen.
- All help and dialog text, written in our own words.

**Never:**
- Extract, copy or trace anything from Aseprite's `data/` folder. That covers the theme art, fonts, cursors, icons and wording. The bundled palettes are included too, unless a palette is separately licensed.
- Use the name "Aseprite" anywhere except to say the app opens `.aseprite` files.

## 3. Scale

Aseprite represents about 15 years of work. Full parity is a long-haul project, so it is planned as a series of independently useful milestones (§6), not one build. Each milestone gets its own spec, then a plan, then a build.

## 4. Architecture

```
rp-core     Document model: sprite, layers (image/group/tilemap/reference), cels, linked cels,
            frames, tags, slices, palettes, tilesets, selection, user-data properties.
rp-ops      THE COMMAND REGISTRY. Each operation is defined once: id, title, docs, parameter schema,
            handler, default shortcut, menu path, undoable?, agent-exposed?
rp-render   CPU compositor: blend modes, opacity, indexed/greyscale/RGB, onion skin, tiled preview.
rp-tools    Brush engine, inks, pixel-perfect, fill, shapes, gradients, selection ops, transforms.
rp-io       Our own .aseprite reader and writer (no maintained crate writes it), plus PNG, GIF,
            sprite-sheet + JSON, and palette formats.
rp-mcp      MCP server. Builds its tool list from rp-ops.
rp-gui      egui app: canvas, timeline, layers, palette, docking.
rp-cli      Batch CLI using Aseprite's flag names, so existing export scripts keep working.
rusty_pixels  The binary: GUI by default, plus --headless-mcp and CLI modes.
```

**One command bus.**
- The GUI, the MCP server, the CLI and any future scripting are all *clients* of one engine.
- A menu click, a keyboard shortcut and an MCP call produce the same `Command`.
- Each `Command` carries all its parameters, with no hidden "current layer" lookups, so replays are deterministic.
- This makes full MCP integration structural rather than a separate API to keep in sync.
- **Registry parity test.** The build fails if:
  - any menu item or shortcut lacks a registry entry;
  - any agent-exposed entry is missing from `tools/list`;
  - any GUI-reachable command can be reached *only* by keyboard.

**Undo** stores tile-level before/after pixel patches (plus structural diffs), grouped into transactions. One brush stroke is one undo step; one MCP call is one step, labelled e.g. "Agent: outline layer 2". Both share a single history, so Ctrl+Z undoes the agent's work too. The serialised command log is kept separately as an audit and replay record.

**Human and agent editing at once.**
- The engine is the single writer, and every command carries its origin (`Human` or `Agent(session)`).
- Agent calls that depend on something read earlier carry the document revision they saw. If the human has edited since, the call fails with "canvas changed" instead of silently painting over the human's work.

**Rendering** happens on the CPU into one RGBA buffer, which is uploaded as a nearest-neighbour texture. Sprites are small, and the same buffer feeds export, undo diffs and the MCP `render` tool, so there is one source of truth for what a pixel is.

### 4a. Pressure and a future Android port

**Pressure.**
- Every input event becomes a `PointerSample { pos, pressure: Option<f32>, tilt: Option<(f32, f32)>, time, source: Mouse | Touch | Pen }`.
- Brush dynamics map pressure or velocity to size, angle and gradient, with a stabiliser. They are built and tested in M5, even though no device sends pressure yet.
- MCP stroke tools accept per-point pressure, so the test suite exercises dynamics without a tablet.
- When egui moves to winit 0.31, wiring real pens in only touches the input adapter in `rp-gui`.

**Android-ready rules, from M0:**
- **Core crates stay portable.** `rp-core`, `rp-ops`, `rp-render`, `rp-tools` and `rp-io` contain no direct filesystem, async-runtime or windowing code. File access goes through a small `Storage` trait, because Android hands apps content URIs rather than paths.
- **Desktop-only crates live in `rp-gui` or the binary**, never in the core crates. For example, `rfd` file dialogs have no Android backend.
- **No action depends on a keyboard, hover or right-click.** A basic registry-driven command palette ships in M2 so that holds from the first GUI build.
- **CI checks the Android build.** `cargo check --target aarch64-linux-android` runs on the core crates.
- **Deferred to the port itself:** touch gestures (two-finger pan and zoom), a touch tool layout, and MCP on Android, which would need to listen beyond localhost and therefore needs real authentication.
- **To verify at M0:** current eframe/egui Android support and its build tooling.

## 5. MCP design

- **Transport.** A Streamable HTTP server (rmcp on axum) inside the running process, bound to 127.0.0.1 only.
  - It requires a bearer token, written to the per-user runtime directory (`$XDG_RUNTIME_DIR` on Linux, the platform equivalent elsewhere).
  - It checks Host/Origin headers so a web page can't drive it.
  - A stdio shim is optional, only if a target client needs it.
  - This deliberately avoids the spawn-per-client stdio model that leaked 430 processes and 10.5 GB with the old Aseprite MCP.
- **Headless mode.** `rusty_pixels --headless-mcp` runs the same engine and server with no window, for agents working alone (e.g. the Godot asset pipeline). This is the M1 deliverable.
- **Tools.** Generated from the registry, but grouped so an agent sees roughly 40–60 tools rather than 300. For example, one `draw` tool with a `shape` enum, and one `filter` tool with a `kind` enum.
- **Read tools, from M1:**
  - `render`: PNG of a frame, layer, region, or a zoomed view with grid.
  - `get_pixels`: compact and palette-index aware.
  - `describe_document`.
- **Read tools, from M2:**
  - `screenshot_window`: what the human is seeing.
  - `get_ui_state`.
- **UI control, from M2:** active tool, zoom and scroll, panel visibility, current layer and frame. "Everything a human can do" includes driving the interface, not only the document.
- **Resources:** `sprite://{doc}/frame/{n}/layer/{id}.png`, with change notifications so a connected agent sees the human's edits live.
- **Transactions:** `begin` / `commit` / `rollback`, so an agent's multi-step edit is one undo step and can be abandoned cleanly.
- **Permissions:** the agent may do everything, disk writes included, without asking (§8.5).
  - Before any overwrite, the previous file is copied to `.rusty_pixels/history/` beside it. The last 10 versions are kept; the number is configurable and the feature can be turned off.
  - Confirm-before-overwrite through MCP elicitation is available as an opt-in setting.

## 6. Milestones

| # | Milestone | Usable result / done when |
|---|---|---|
| M0 | Cargo workspace, `rp-core`, `rp-ops` registry, undo, `rp-io` `.aseprite` read/write, CI on 3 OS + Android check | Every `.aseprite` file in the local corpus round-trips. "Lossless" means: re-reading our output gives an equal document model, and Aseprite renders it identically to the original (byte-identical files are not required; zlib output varies) |
| M1 | Engine + `--headless-mcp` + `rp-cli` (core export flags) | An agent creates, draws, animates and exports with no GUI; replaces the batch-file Aseprite MCP |
| M2 | GUI shell: canvas, zoom/pan, pencil/eraser/fill/eyedropper, layers, palette, basic command palette, live MCP co-editing, UI-control tools | First "draw alongside the agent" build |
| M3 | Animation: timeline, cels, linked cels, tags, onion skin, playback, preview, GIF and sprite-sheet export | Animate a character |
| M4 | Selection and transform: marquee, lasso, wand, move, RotSprite rotate, scale/skew, clipboard | |
| M5 | Full toolset: shapes, contour, gradients, spray, blur/jumble, symmetry, pixel-perfect, custom brushes, all inks, tiled mode, dynamics | |
| M6 | Colour: palette editor, colour wheels, ramps, sorting, colour-mode conversion with dithering | |
| M7 | Filters: outline, hue/saturation, curves, convolution, replace colour, despeckle, and the rest of the inventory's filter list | |
| M8 | Tilemaps: tilesets, auto/manual/stack modes, flips | |
| M9 | Slices and 9-slice, user-data properties, full sprite-sheet import/export, full CLI parity | |
| M10 | Preferences, shortcut editor, workspace layouts, themes, full command palette, extensions | |
| later | Lua scripting compatibility | Thin client of the bus; large job, deferred |

Agent-first ordering (M1 before M2) is deliberate. It proves the command bus before any UI code exists. Progress in M0–M1 is visible through `render` PNGs and through our files opening correctly in real Aseprite.

## 7. Verification

- **Reference oracle.** We draw fixture sprites ourselves, export them with the installed Aseprite CLI on this machine, and commit the resulting PNG/JSON as expected outputs. CI compares our output pixel for pixel against those files. CI never runs Aseprite, since it can't be redistributed.
- **Round-trip corpus.** Every `.aseprite` file on this machine: read, write, re-read, compare, and render through Aseprite's CLI. These files are personal, so they stay local and are never committed.
- **MCP parity test.** A test client calls every agent-exposed registry entry and checks the result, the undo, and the redo.
- **GUI (from M2).** The app drives itself through its own `screenshot_window` and UI-control tools, plus an `xvfb-run` screenshot harness. Unit tests alone don't count as proof that the GUI works.

## 8. Decisions (settled 2026-10-11)

1. **Licence and audience:** MIT/Apache, public on GitHub. LibreSprite is a behaviour reference only.
2. **Pen pressure:** mouse only for now, using egui. Pressure functionality is built in (§4a), and an Android port is possible later.
3. **Look:** as close as is safe (§2a).
4. **Order:** agent-first. M0 → M1 before any GUI.
5. **Agent permissions:** the agent may do everything without asking, with an automatic history backup before overwrites (§5).
6. **Platforms:** Linux is primary. CI builds and tests Linux, Windows and macOS from M0, plus an Android `cargo check` of the core crates. A red build on any platform blocks a merge. Windows and macOS get no polish or installers until later.
7. **Name:** Rusty Pixels.

## 9. Stack

- egui 0.36 + eframe, with egui_tiles for docking (check floating-window support against egui_dock at M2)
- wgpu, for display only
- rmcp 3.5 (Streamable HTTP) + axum 0.8 + tokio
- `png`, `gif`, `image`
- `quantette`, not `imagequant`, which is GPL
- `palette`
- `flate2`
- `serde` / `schemars`
- `rfd`, desktop only

Versions were checked on 2026-10-11 and are re-verified at the start of M0.

**Open items to verify at M0:**
- the exact rmcp Streamable HTTP server API, and whether it already enforces Host/Origin checks;
- eframe Android support;
- whether `aseprite-loader` covers tilemaps, for use as a cross-check oracle.
