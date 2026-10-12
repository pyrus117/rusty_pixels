#!/usr/bin/env bash
# Regenerate the rp-render oracle: Aseprite builds the sprites (make_render_fixtures.lua),
# then its CLI flattens every frame to <name>_<frame>.png. Both are committed; CI never runs Aseprite.
set -euo pipefail
repo=$(cd "$(dirname "$0")/../.." && pwd)
aseprite=${ASEPRITE:-$HOME/.local/share/Steam/steamapps/common/Aseprite/aseprite}
out="$repo/crates/rp-render/tests/fixtures"
[[ -x $aseprite ]] || { echo "render_oracle.sh: no Aseprite at $aseprite" >&2; exit 2; }

rm -rf "$out"
mkdir -p "$out"
"$aseprite" -b --script-param out="$out" --script "$repo/tools/fixtures/make_render_fixtures.lua"
# The rp-io fixtures add linked cels, tilemaps and layer trees to the oracle for free.
for f in "$repo"/crates/rp-io/tests/fixtures/*.aseprite; do
  cp "$f" "$out/io_$(basename "$f")"
done
for f in "$out"/*.aseprite; do
  name=$(basename "$f" .aseprite)
  # Aseprite prints X11 auth noise on stderr in batch mode; real failures show as missing PNGs below.
  "$aseprite" -b "$f" --save-as "$out/${name}_{frame}.png" 2>/dev/null
  [[ -f $out/${name}_0.png ]] || { echo "render_oracle.sh: no render for $name" >&2; exit 1; }
done
echo "render_oracle.sh: $(ls "$out"/*.aseprite | wc -l) sprites, $(ls "$out"/*.png | wc -l) renders" >&2
