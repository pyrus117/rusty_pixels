# Clean-room log

Rusty Pixels reimplements Aseprite's *functionality* without using Aseprite's code. This file
records every source consulted so that claim can be checked.

## Rules
- Never read, copy or port Aseprite source code. Never decompile or disassemble the Aseprite binary.
- Never copy artwork, fonts, icons, cursors, palettes or text from Aseprite's `data/` folder.
- LibreSprite (GPL-2): source code not read. Behaviour may be observed by running it.
- Running the installed Aseprite to produce reference outputs is normal use and is allowed.
- Code-writing agents are told these rules in every task prompt. Enforcement is by instruction
  and review; the agents' tools do not technically block web or file access.

## Sources consulted
| Date | Source | Licence / status | Used for |
|---|---|---|---|
| 2026-10-11 | Aseprite `data/` files: gui.xml, pref.xml, strings/en.ini, widgets/*.xml, convmatr.def, bundled extension manifests and palette names. Names and options only. Types and defaults were removed from the research notes on 2026-10-12. | Shipped app data, read not copied | `research/feature-inventory.md` |
| 2026-10-11 | `aseprite --help` output | Program output | CLI flag names |
| 2026-10-11 | The owner's local Aseprite config: preference file, `user.aseprite-layouts` and `user.aseprite-brushes` (shape only), `palettes/default.ase` (format only) | User's own files | Feature prioritisation (kept outside the repo); config file shapes |
| 2026-10-11 | ase-file-specs.md (published file-format spec; it carries no explicit licence) | Published for interoperability | `rp-io` |
| 2026-10-11 | github.com/aseprite/api | MIT | MCP surface design, fixture script |
| 2026-10-11 | aseprite.org/docs/cli, the Aseprite EULA, and the author's public gists showing sprite-sheet JSON shapes | Public docs | CLI flag names, licensing, export formats |
| 2026-10-11 | LibreSprite repository metadata and file-tree listing (file names only, to check whether tilemaps exist). No source file opened. | GPL-2.0 | Licensing and feature-gap notes in `research/formats-and-api.md` |
| 2026-10-11 | Pixelorama README (feature list) | MIT | Feature reference |
| 2026-10-11 | Rust crate docs (egui, rmcp, …) | Various OSS | `research/rust-ecosystem.md` |
| 2026-10-11 | asefile / aseprite-loader READMEs and changelogs | MIT / MIT-or-Apache | Survey of existing crates; no code used |
| 2026-10-12 | Installed Aseprite run in batch mode with our own Lua script (`tools/fixtures/make_fixtures.lua`), producing the files in `crates/rp-io/tests/fixtures/` | Normal use; outputs are our own art | Test fixtures; chunk order observed from those outputs |
| 2026-10-12 | Installed Aseprite rendering local `.aseprite` files to PNG (`tools/corpus.sh`) | Normal use; outputs never committed | Black-box round-trip oracle |
| 2026-10-12 | Installed Aseprite loading crafted files in batch mode (second-review probes, kept outside git in the `m0-fix2-ops` worktree's `target/ase-probe/`) | Normal use; files are our own | Behaviour behind R1, R2, R9, C2 and C3 |
| 2026-10-12 | W3C Compositing and Blending Level 1 (public spec) | W3C Document Licence | Blend-mode formulas in `rp-render` (soft light, SetSat/SetLum/ClipColor); integer rounding fitted to the oracle fixtures |

## Incidents

**2026-10-12: a review agent read Aseprite source.** During the second M0 review pass, the `reviewer` agent downloaded seven Aseprite source files to a temporary directory: the .aseprite decoder and encoder, the CLI processor, the open-file command, the console and the document exporter. It used them to back behaviour claims in its report. The review prompt had banned reading private `.aseprite` files but had not banned reading Aseprite source; that omission was the orchestrator's.

Response:
- The downloaded files and the agent's proof tests were deleted the same day.
- No code was copied, and no implementer saw the files.
- The source-derived claims were not passed to the implementer. Every behaviour they described was re-established black-box by running the installed Aseprite on crafted files before any fix was made, and fixes cite those observations only.
- The first review pass was checked: it only quoted the spec's own NOTE.5 link and read no source.
- Prevention: the rule is now in the repo's `CLAUDE.md`, which every agent session loads, and in every review prompt.
