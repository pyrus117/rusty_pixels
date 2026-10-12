# Rusty Pixels

Rust pixel-art editor with Aseprite-level features and full MCP control. Spec: `docs/specs/`. Current plan: `docs/plans/`. Gate: `.claude/verify.sh`.

## Clean room — applies to every agent in this repo, including reviewers and researchers
- Never open, download, fetch or search Aseprite's source code (anything under github.com/aseprite/aseprite/src, any Aseprite `.cpp`/`.h`), and never decompile or disassemble the Aseprite binary. This holds even to "check a claim". The spec's NOTE.5 links to Aseprite source; do not follow it.
- To learn how Aseprite behaves, run the installed Aseprite on files we craft (black-box) and observe its output. That is allowed and preferred.
- Allowed references: `research/vendor/ase-file-specs.md` (published format spec), `research/vendor/aseprite-api/` (MIT API docs), public user docs, and observed behaviour.
- Never read `.aseprite` files outside this repository: they are the owner's private files. Only `tools/corpus.sh`, run by the orchestrating session, touches them.
- Log every new source in `CLEANROOM.md`.

## MiMo workers
Code is written by MiMo through `tools/mimo.sh`, in a git worktree, against tests it may not edit. A MiMo worker implements the task itself and never delegates.
