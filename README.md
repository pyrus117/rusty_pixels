# Rusty Pixels

A pixel-art editor written in Rust. The goal is Aseprite-level functionality — sprite animation, layers, timeline, tilemaps, palettes, sprite-sheet export — with one addition: every action a person can take is also available to an AI agent through MCP (Model Context Protocol).

## AI + human, same document

Person and agent edit the same live document with a shared undo history. Agent edits appear in undo as labelled steps. If the person edits while an agent is mid-task, the agent's stale call fails cleanly instead of painting over their work.

## Status

**Pre-alpha.** There is no GUI yet and nothing usable for drawing.

**M0 (foundations) — in progress**
Document model, a lossless `.aseprite` file reader/writer, the command registry with undo/redo, CI.

**M1 — next**
Headless engine with an MCP server and a command-line tool.

**M2**
First GUI.

## .aseprite file support

Rusty Pixels opens and saves `.aseprite` files, implemented from the publicly published file-format specification.

## Building

Requires **Rust 1.96+**.

```
cargo test --workspace
```

## Platforms

Linux first. CI also builds Windows and macOS. An Android port is possible later.

## Documentation

- Design spec: `docs/specs/2026-10-11-rusty-pixels-design.md`
- Current plan: `docs/plans/2026-10-11-m0-foundations.md`
- Clean-room sources: `CLEANROOM.md`

## Not affiliated with Aseprite

Rusty Pixels is an independent project. It is not affiliated with or endorsed by Aseprite or Igara Studio and does not use Aseprite's source code. "Aseprite" is a trademark of Igara Studio S.A.

## License

Dual-licensed under MIT or Apache 2.0, at your option.

- `LICENSE-MIT`
- `LICENSE-APACHE`
