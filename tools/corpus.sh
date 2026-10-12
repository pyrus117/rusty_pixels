#!/usr/bin/env bash
# Round-trip every local .aseprite file through rp-io and compare Aseprite renders.
# Personal files: outputs go to ./corpus-out (git-ignored) and are never committed.
set -euo pipefail
repo=$(cd "$(dirname "$0")/.." && pwd)
aseprite=${ASEPRITE:-$HOME/.local/share/Steam/steamapps/common/Aseprite/aseprite}
list=$(mktemp)
# Clear old outputs first so the scan below can never pick up our own rewrites.
rm -rf "$repo/corpus-out"
# Unreadable directories are expected under $HOME; skip them rather than abort.
# Our own fixtures and outputs (in any checkout or worktree) are excluded.
find "$HOME" -xdev \( -name '*.aseprite' -o -name '*.ase' \) \
  -not -path '*/Trash/*' -not -path '*/steamapps/common/Aseprite/*' \
  -not -path '*/rusty_pixels*/crates/*' -not -path '*/rusty_pixels*/corpus-out/*' \
  -not -path '*/rusty_pixels*/target/*' \
  > "$list" 2>/dev/null || true
count=$(wc -l < "$list")
[ "$count" -gt 0 ] || { echo "corpus.sh: no .aseprite files found under $HOME" >&2; exit 1; }
echo "corpus.sh: $count files" >&2
cargo run --quiet --release --manifest-path "$repo/Cargo.toml" -p rp-corpus -- \
  --aseprite "$aseprite" --out "$repo/corpus-out" --list "$list"
