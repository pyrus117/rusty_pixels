# Rust ecosystem research: pixel-art editor with full MCP integration

Research date: 2026-10-11. Versions and dates come from the crates.io API and the GitHub API queried today. Statements marked (memory) come from background knowledge and were not re-checked today; verify before committing to them.

## 0. Recommendation (read this first)

- **GUI: egui 0.36 via eframe, with egui_tiles 0.17 (or egui_dock 0.21) for docking.** Canvas is a custom widget. Timeline, palette and layer panels are plain egui code.
- **Rendering: CPU compositing into an RGBA buffer, uploaded as one texture; wgpu only displays it (egui-wgpu is the backend anyway).** Sprites are tiny. A GPU compositor adds a second implementation of every blend mode, and the MCP "read canvas pixels" feature needs a CPU-exact result regardless.
- **MCP: `rmcp` 3.5.1, Streamable HTTP server running in-process on 127.0.0.1 inside the GUI process.** Ship a tiny optional stdio shim binary that proxies to that endpoint, for clients that only launch stdio servers.
- **Architecture: a headless `core` crate owns the document. Every mutation is a serialisable `Command` sent through one channel. The GUI and the MCP server are both clients of that bus.** The MCP tool list is generated from the same command registry as the menus and shortcuts.
- **Biggest risk: pen pressure.** egui 0.36 pins winit 0.30.13, which has no pen/tablet events on desktop. winit 0.31 has them on Wayland, Windows and Web, but it is still beta, and egui has not moved to it. See section 1.
- **Licence traps:** `imagequant` is GPL-3.0-or-later (use `quantette`, MIT/Apache, instead). Aseprite source is under a proprietary EULA, not reusable. `rx` is GPL-3.0 and dormant.

## 1. GUI toolkits

| Toolkit | Version (crates.io) | Licence | Docking | Custom wgpu canvas | Verdict |
|---|---|---|---|---|---|
| egui + eframe | 0.36.2 (2026-09-08), repo pushed 2026-10-09, 31k stars | MIT OR Apache-2.0 | `egui_dock` 0.21.1 (MIT, 2026-08-06), `egui_tiles` 0.17.1 (MIT OR Apache-2.0, 2026-08-18, from Rerun) | Excellent: `egui-wgpu` paint callbacks, or just upload a texture | **Pick** |
| iced | 0.14.0 (2025-12-07) | MIT | None built in; panes (`pane_grid`) is split-only, no tabs/floating | `iced::widget::shader` gives wgpu access | Release cadence slowed; no docking |
| Slint | 1.18.1 (2026-09-21) | GPL-3.0 OR Royalty-free-2.0 OR commercial | None built in | Image-based canvas is easy; wgpu integration is more awkward | Licence is awkward for a possible later release; no docking |
| Vizia | 0.4.0 (2026-04-23), ~6.7k downloads | MIT | None | Possible but small ecosystem | Too immature for a dense pro tool |
| Xilem / Masonry | 0.4.0 (2025-10-29) | Apache-2.0 | None | Vello-based | Alpha; repo last pushed 2026-09-14; no docking |
| gpui (Zed) | 0.2.2 on crates.io (2025-10-22, stale); main branch is live | Apache-2.0 | None built in (Zed's dock is in the `workspace` crate, not gpui) | Custom elements possible | Usable standalone via git dependency (README documents `gpui_platform` standalone setup) but pre-1.0 with constant breakage; no widget set |
| Dioxus native / Blitz | dioxus 0.7.10; blitz 0.2.1 (2026-08-24) | MIT/Apache | Whatever HTML/CSS you build | CSS layout, wgpu via Vello | Blitz is 0.x, ~2k downloads; not ready |
| Tauri 2 + web canvas | tauri 2.12.3 (2026-10-10) | MIT/Apache | Web libs (dockview, golden-layout) | Canvas2D/WebGL/WebGPU in webview | Pen pressure works for free via Pointer Events. Cost: Rust/JS split, IPC for every canvas op, WebKitGTK on Arch is the weak link. Strong fallback if pressure matters most |

### Docking detail
- `egui_dock` (tabs, drag to split, floating windows) is the more finished dock UI. `egui_tiles` is lower level, with tabs and splits in a tree you serialise yourself, and is maintained by Rerun, so it gets real use. Either works. I would start with egui_tiles for the layout-as-serialisable-data property (workspace save/restore), but check that tear-off floating windows meet your needs (memory: egui_tiles has no native floating windows).
- egui is immediate mode: no retained widget state, so "keyboard-shortcut-heavy" is straightforward (read `ctx.input` each frame, map to action IDs). Custom theming is via `Visuals`/`Style`. Dense layouts are the weak point: text layout cost, and no real scrollable virtualised grid for a long timeline (write the timeline as a custom painter widget, which you would do anyway).
- High DPI: handled by egui via winit scale factor. Pixel-perfect canvas needs nearest-neighbour sampling and integer zoom, which you control in your own texture/callback.

### Pen/tablet pressure (the real constraint)
- Today's winit stable is **0.30.13** (2026-09-04 crates.io). egui main pins `winit = "0.30.13"` (checked in egui's workspace Cargo.toml). In 0.30 there is no pen device concept; `Touch` carries `Force` only for touchscreens.
- winit **0.31.0-beta.3** (2026-09-04; beta.1 and beta.2 were 2025-11-16): the pointer event overhaul adds `PointerKind`/`PointerSource` and "Pen input support on Wayland, Windows, and Web" (beta.1 notes), Apple Pencil on iOS (beta.3), `ToolAngle` for tilt. Open winit issues today: #4633 "X11: implement tablet", #4593 "AppKit: Add tablet tool support", #4741/#4742 Windows per-sample (frame-locked) pressure. A closed fix "winit-x11: don't classify VM tablet pointers as pens" suggests some X11 pen classification exists on master, but X11 tablet support is explicitly still an open item. Treat X11 and macOS pen as **not available**.
- egui issues about tablets are open and old: #2104 "Wacom Stylus Input Driver Support for content creation" (2025-05), #134 "Wacom Pen Input" (2025-05), #8157 "Support tablet pressure in the browser" (2026-05). egui has no first-party pen pressure path.
- Graphite already runs on `winit = "0.31.0-beta.2"` (workspace Cargo.toml) plus wgpu 29 and vello 0.10, so beta is workable in production for them.
- **Options for pressure:** (a) stay on egui and accept no pressure initially (pixel art is mostly 1px pencil; pressure matters mainly for brush size/opacity); (b) fork or patch egui-winit onto winit 0.31 beta (Graphite shows it is feasible, but you carry the fork); (c) read pen events out-of-band (Wayland `zwp_tablet_v2`, XInput2, evdev/libinput) and inject them as custom events into the command stream; (d) Tauri, where Pointer Events give pressure everywhere WebKitGTK supports it (WebKitGTK pressure behaviour: not verified).
- Recommendation: start with (a), design the input layer so a `PointerSample { pos, pressure, tilt, time }` type exists from day one, and revisit when egui adopts winit 0.31 stable.

## 2. Rendering: wgpu vs CPU

- wgpu is at **30.0.1** (2026-08-22). egui 0.36's `egui-wgpu` tracks it. Graphite uses 29, so pick whatever egui pins and do not add a second wgpu version.
- What a sprite editor needs: documents are typically 16x16 to 512x512 at 1-3 bytes/pixel, with maybe 10-50 layers and up to hundreds of frames. A full composite of a 512x512, 20-layer document is ~5M pixel blends (low single-digit ms on one core, trivially fast with dirty rects). Onion skin and tiled-mode preview add a few more composites.
- CPU compositing keeps one authoritative implementation of blend modes, indexed colour, and layer alpha. That same buffer feeds export, the MCP "get pixels" tool and screenshot, and undo diffs. A GPU compositor would need a CPU twin for those anyway.
- Upload path: one `egui::TextureHandle` updated via partial `set` (dirty rect) each change, drawn with `TextureOptions::NEAREST`. Overlays (grid, selection marching ants, tool cursor, tile boundaries) are drawn with egui's painter or a custom `egui_wgpu::Callback`.
- Reach for GPU compositing only if you add very large canvases (4096+ px), real-time filters/shaders, or heavy brush engines. `tiny-skia` 0.12.0 (BSD-3, 2026-02-02) is useful for anti-aliased vector shapes (e.g. shape tools in non-pixel modes, UI icons, text export), not for pixel-exact pencil work; do pixel tools with integer algorithms (Bresenham, flood fill, Aseprite-like pixel-perfect line cleanup) on your own buffers.
- `vello` 0.11.0 (2026-10-02) and `femtovg` 0.27.0 are vector renderers; wrong tool for a raster pixel editor.

## 3. MCP in Rust

### rmcp (official SDK, modelcontextprotocol/rust-sdk)
- **rmcp 3.5.1 / rmcp-macros 3.5.1** (2026-10-05), Apache-2.0, MSRV 1.88. 34.5M downloads. 3.x is a breaking release (migration guide linked from README).
- README states it implements the stable MCP **2026-07-28** spec and stays compatible with **2025-11-25** and earlier. New in 2026-07-28 per the README: server discovery and negotiation (`server/discover`), transport-neutral subscriptions (`subscriptions/listen`, replacing `resources/subscribe`), long-running tasks, response caching, multi-round-trip requests, standard HTTP routing headers. Client lifecycle modes `Initialize` / `Discover` / `Auto` exist.
- **Transports (README table):** stdio (`transport-io`), child-process client, **Streamable HTTP server** (`transport-streamable-http-server`, exposed as a Tower service mountable on axum/hyper), Streamable HTTP client (`transport-streamable-http-client-reqwest`), worker/in-process (`transport-worker`). The legacy 2024-11-05 HTTP+SSE transport is deliberately not provided.
- **Macros:** `#[tool_router]`, `#[tool]`, `#[tool_handler]`; `#[tool_router(server_handler)]` for tools-only servers. Parameters via `Parameters<T>` with `schemars::JsonSchema` (JSON Schema 2020-12).
- **Image content in tool results:** `ContentBlock::image(base64_png, "image/png")`, also audio and embedded resources. This is how the agent "sees" the canvas.
- **Resources:** `list_resources`, `read_resource`, `list_resource_templates`; text or base64 blob with MIME type. Good fit for `sprite://doc/1/frame/3/layer/2.png` style URIs.
- **Subscriptions/notifications:** `accepted_subscription_filter` + `listen(SubscriptionContext)` with a sink offering e.g. `notify_tool_list_changed()`. Use for "canvas changed" and "tool list changed" pushes.
- **Elicitation:** feature flag `elicitation`; `ctx.peer.elicit::<T>(msg)` (form, schema-validated) and `elicit_url`. Useful for "ask the human to confirm a destructive operation" or "pick a palette".
- Also supports tasks (long-running calls, e.g. big GIF export), `request-state` integrity helper, OAuth (`auth` feature; unnecessary on localhost).
- Streamable HTTP snippet in the README refers to a "Stateless Streamable HTTP" section for a full server example; I did not extract the exact `StreamableHttpService::new(...)` signature. Pull it from `examples/servers` in the repo before coding.

### Embedding inside a running GUI app
Two options:

1. **In-process Streamable HTTP on 127.0.0.1 (recommended).** Spawn a tokio runtime on a background thread at app start, mount `StreamableHttpService` in axum (axum 0.8.9 is current). Tool handlers send `Command` plus a `oneshot` reply over a channel to the app's UI/engine thread. Clients (Claude Code, Claude Desktop, Cursor) connect by URL (`claude mcp add --transport http pixels http://127.0.0.1:PORT/mcp`).
   - Pros: no second process, shared state is the live document, human and agent see the same thing, no startup race.
   - Cons: server only exists while the GUI runs; port management (write chosen port + token to a lock file in `$XDG_RUNTIME_DIR`); must guard against browser/DNS-rebinding attacks on a localhost server (bind 127.0.0.1 only, check Host/Origin, require a bearer token). I did not verify which of these rmcp enforces out of the box; check `StreamableHttpServerConfig` docs.
2. **stdio shim.** A second binary speaks MCP over stdio to the client and forwards each call to the running app (over HTTP, a Unix socket, or the same MCP client transport via rmcp's client). Needed for clients that can only spawn stdio servers, and it can launch the GUI on demand if none is running. It is small: rmcp has both client and server roles, so the shim is mostly a pass-through proxy.
- Build option 1 first; add the shim only if a target client needs it.

### Threading model
egui runs on the main thread, rmcp on tokio. Do not share `Arc<Mutex<Document>>` for mutation from both sides. Use the command bus (section 4): the MCP thread sends `Command`, the engine applies it, replies with a result, and calls `ctx.request_repaint()` (egui `Context` is `Send + Sync`) so the human sees the change immediately.

## 4. Architecture: one command bus

### How others do it
- **Blender:** every operator is `bpy.ops.*`, registered with a poll function and undo flags; UI buttons, menus and Python all call operators. Properties are RNA, introspectable. Source of "UI = scripting surface" fidelity.
- **Krita:** actions (`QAction`) plus a Python API (`krita` module) over the same document model; commands are `KUndo2Command` objects.
- **Godot editor:** `EditorUndoRedoManager` (do/undo method calls), EditorPlugin API, and the GDScript `@tool` layer. Third-party MCP servers drive it through plugin sockets.
- **Zed:** `actions!` macro; every user-visible operation is a typed Action dispatched by name; keymaps bind keys to action names via JSON; command palette lists them. This is the closest to what you want for UI-to-command mapping (memory).
- **Graphite (Rust, Apache-2.0, 27.5k stars, pushed 2026-10-10):** `editor/src/messages/` is a hierarchy of `Message` enums (`portfolio`, `tool`, `input_mapper`, `broadcast`, `frontend`, `layout`, `preferences`, `viewport`, ...) processed by a dispatcher; handlers return follow-up messages onto a queue. Input mapping is itself a message translation layer. Document history is stored as messages/transactions (memory). The node graph executes through a `wgpu-executor` and Vello (workspace deps: `vello = "0.10"`, `wgpu = "29.0"`, `winit = "0.31.0-beta.2"`, `cef = "151"` for the desktop web UI). Their model, "everything is a message dispatched through one queue", is exactly the pattern, but it grew organically and is heavy for a sprite editor.

### Recommended pattern for this project
- `core` crate (no GUI deps): `Document` (sprite, layers, cels, frames, tags, palette, selection), pure functions to render/composite.
- `Command` enum, `serde` + `schemars` derived. Examples: `DrawPixels{layer, frame, points, color}`, `SetSelection`, `AddLayer`, `MoveCel`, `SetPalette`, `Export`. Each variant carries all its parameters (no implicit "current tool/layer" reads) so replaying is deterministic. Sessions can also carry an explicit "current context" (active layer/frame), as Aseprite's scripting API does, but resolve that to IDs before the command is logged.
- `Engine` owns `Document` and a history. `engine.apply(cmd) -> Result<Outcome, Error>`; `Outcome` includes a dirty rect, inverse patch, and optional return value (e.g. new layer ID).
- **Undo:** the common sprite-editor approach is storing a sparse pixel patch (before/after tile diffs) per command, rather than inverse operations. Group by "transaction" (a whole brush stroke = one undo step; an MCP tool call = one undo step, optionally labelled "Agent: draw line"). The `undo` crate (0.52.0, MIT/Apache, last published 2025-03-08, stable but quiet) implements `Edit`/`Record`/`History` with merging and could save time. Hand-rolling is also reasonable (~300 lines) and avoids fitting your patch model to its trait. The command log (list of serialised commands) is an *audit/replay* record; undo itself should use patches because replaying from the start is too slow.
- **Live drawing:** a mouse stroke produces `BeginTransaction`, many `StrokePoint`, `EndTransaction`. The GUI applies strokes locally for latency but through the same engine entry points. The agent normally sends a whole polyline in one command.
- **Human + agent concurrently:** serialise through the engine (single writer). Each command carries an origin (`Human`/`Agent(session)`), shown in the undo history. Add optimistic revision checks (`base_revision`) for agent commands that depend on pixel reads, so the agent learns when the human changed the canvas in between and the call fails cleanly rather than silently overwriting. Provide a `get_revision` and a "document changed" notification.
- **Keeping MCP in sync with the UI automatically:**
  - Define each operation once in a registry: `{id, title, doc, params schema (schemars), handler, default shortcut, menu path, undoable, mcp_exposed}`. The menu bar, command palette, shortcut map, and MCP `tools/list` are all generated from this registry.
  - Use rmcp's `ServerHandler` with a dynamic `list_tools`/`call_tool` that reads the registry, rather than one `#[tool]` method per feature. Keep `#[tool]` macros for a handful of hand-written read tools (screenshot, get_pixels, document summary) that return images.
  - A CI test that asserts every registry entry with `mcp_exposed` appears in `tools/list` and every menu item maps to a registry ID prevents drift. Tool count matters to agents; group related variants (a single `draw` tool with a `shape` enum) rather than exposing 300 tools.
- **Read side for the agent:** tools returning PNG image content (flattened frame, a layer, a zoomed region with grid), `get_pixels(rect)` as compact RLE or palette-index text, `describe_document` JSON; plus resources for the same data, with `resources/updated` style notifications so a subscribed client sees edits live.

## 5. Existing Rust raster/pixel editors

| Project | State | Licence | Reusable? |
|---|---|---|---|
| rx (cloudhead) | Last push **2024-01-31**, 3.5k stars, dormant | GPL-3.0 | Ideas only (modal/command-line UX, `.rx` scripting). GPL blocks code reuse unless your project is GPL |
| Graphite | Very active (2026-10-10) | Apache-2.0 (LICENSE-APACHE + LICENSE-MIT present) | Architecture reference; pieces such as `wgpu-sync`, `wgpu-executor` and the message system are readable but not designed for reuse; vector/node focus, not pixel-exact |
| Aseprite | Active (pushed 2026-10-08) | Proprietary EULA (Igara Studio), source-available | Do not copy code. File format `.aseprite` is publicly documented in `docs/ase-file-specs.md` in the repo (the spec is separate from the code licence question; check) |
| LibreSprite | Active (pushed 2026-10-10) | GPL-2.0 | Fork of Aseprite from before the relicence; useful as behavioural reference only, copyleft |
| Pixelorama (Godot/GDScript) | not rechecked | MIT (memory) | Feature-list reference (non-Rust) |
| Rerun (egui_tiles users), Lapce/floem | n/a | | Good egui_tiles and non-egui retained-GUI references |

## 6. Supporting crates

| Need | Crate | Version / date | Licence | Note |
|---|---|---|---|---|
| Image I/O | `image` | 0.25.10 (2026-03-10) | MIT/Apache-2.0 | PNG/GIF/JPEG/WebP/BMP/TGA etc. |
| PNG with exact control | `png` | 0.18.1 (2026-02-14) | MIT/Apache-2.0 | Indexed PNG + PLTE/tRNS output; use directly for palette PNGs |
| GIF | `gif` | 0.14.2 (2026-04-09) | MIT/Apache-2.0 | Per-frame palettes, disposal, delays, loop count; you must supply quantised indexed frames |
| Palette quantisation | `quantette` | 0.6.0 (2026-05-15) | MIT OR Apache-2.0 | Wu and k-means, dithering, palette extraction; **use this** |
| | `imagequant` | 4.4.1 (2025-07-13) | **GPL-3.0-or-later** | Best-known quality, but copyleft; avoid unless the app is GPL (or buy the commercial licence from the author) |
| | `color_quant` | 2.0.0 (2026-05-09) | MIT | NeuQuant, simple fallback |
| Colour science | `palette` | 0.7.7 (2026-08-02) | MIT/Apache-2.0 | Lab/Oklab/HSV conversions for palette generation, nearest-colour, ramp generation |
| Lua scripting | `mlua` | 0.12.2 (2026-10-03) | MIT | Features include `lua54`, `lua55`, `luajit`, `luau`, `vendored`, `serialize`, `async`. Aseprite scripts use Lua 5.3 (memory). A compatibility layer is a large project: reimplementing `app.sprite`, `Image`, `Cel`, `Dialog` etc. Defer; once the command bus exists a Lua binding is a thin client of it (mlua with `vendored lua54`) |
| Undo | `undo` | 0.52.0 (2025-03-08) | MIT/Apache | See section 4 |
| File watching | `notify` | 8.2.0 (2026-08-30) | CC0/Artistic (memory) | For reloading palettes/exports edited externally; `notify-debouncer-*` for debounce |
| File dialogs | `rfd` | 0.17.2 (2026-01-12) | MIT | Native dialogs (xdg-portal on Wayland) |
| Text | `cosmic-text` | 0.19.0 (2026-04-22) | MIT/Apache | Only if you need bitmap-text tools beyond egui's fonts |
| Async runtime / HTTP | `tokio` 1.53.2, `axum` 0.8.9 | | MIT | For the MCP side |

Aseprite file format (`.ase`/`.aseprite`) reading and writing: I did not locate a current, maintained Rust crate during this pass. Search crates.io for "aseprite" before writing a parser; the format spec is simple enough that a parser is a few hundred lines.

## 7. Trade-offs summary

- **egui vs Tauri:** egui gives one language, one process, direct memory access to the canvas buffer (no IPC cost, simpler MCP sharing), and good Linux native behaviour. Cost: no pen pressure until winit 0.31 support lands, and immediate-mode UI limits (custom timeline needed). Tauri gives pen pressure and rich CSS theming, but splits the codebase and puts the canvas behind IPC.
- **egui vs gpui/iced/Slint:** none of the others has docking, and egui's ecosystem (dock crates, wgpu integration) is the most mature. gpui's crates.io release is a year old, so it needs a git pin, and it has no widget library.
- **In-process HTTP vs stdio shim:** HTTP in-process is simplest and gives true live co-editing. The shim is only a compatibility add-on.
- **CPU vs GPU compositing:** CPU now; the compositing function is isolated in `core`, so a GPU path can be added later without touching the command layer.
- **Do not depend on `imagequant`** unless GPL is acceptable.

## 8. Open items to verify before coding

1. Exact `rmcp` Streamable HTTP server API and host/origin protection (read `crates/rmcp` examples at tag 3.5.1).
2. Whether egui's `egui-winit` can be pointed at winit 0.31 beta with a small patch (Graphite uses `0.31.0-beta.2`, so wgpu surface integration works; egui-winit itself has to be adapted).
3. Pen support completeness on X11 (winit #4633 open) and macOS (#4593 open) for the winit 0.31 line.
4. A maintained Rust `.aseprite` parser, if file compatibility is a goal.
5. egui_tiles floating-window support versus egui_dock.
