# Formats, scripting API, CLI, licensing, crates (clean-room research)

Compiled 2026-10-11 from public documents only. No Aseprite source or binary was read. Aseprite source code is NOT reproduced here; the spec below is a re-statement of the published format document (field layouts are facts needed for interoperability).

Sources fetched:
- Spec: https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md (raw: https://raw.githubusercontent.com/aseprite/aseprite/main/docs/ase-file-specs.md)
- API docs: https://github.com/aseprite/api (MIT, see section 3), site https://www.aseprite.org/api
- CLI: https://www.aseprite.org/docs/cli/
- EULA: https://raw.githubusercontent.com/aseprite/aseprite/main/EULA.txt
- LibreSprite: https://github.com/LibreSprite/LibreSprite ; Pixelorama: https://github.com/Orama-Interactive/Pixelorama
- Crates: crates.io API + the repos' READMEs/CHANGELOGs

---------------------------------------------------------------------------

## 1. The .ase / .aseprite file format

### 1.1 Terms of the spec document
- The spec is a markdown file in the Aseprite repo, published explicitly so third parties can read/write the format (it names the .ase/.aseprite format as the thing to implement; Rust, Go, JS, C# implementations exist under MIT/Apache).
- The file carries no separate licence header. The repo as a whole is under the Aseprite EULA (section 5). Practical reading: implementing the format from the spec is the intended use; do not copy Aseprite source. Confidence: high on intent, medium on legal formality (the doc has no explicit grant). If certainty is needed, ask Igara Studio or rely on the fact that every third-party loader (asefile, aseprite-loader) is MIT/Apache and links this spec in its README.
- Latest-version note in spec text: Aseprite v1.3.5 changed when the old palette chunk is written. Spec is "living": it is edited in place; no explicit format version number in the file beyond the magic number and flags.

### 1.2 Primitive types (all little-endian)
BYTE u8, WORD u16, SHORT i16, DWORD u32, LONG i32, FIXED 16.16 i32, FLOAT f32, DOUBLE f64, QWORD u64, LONG64 i64. STRING = WORD byte-length + UTF-8 bytes (no NUL). POINT = 2xLONG. SIZE = 2xLONG. RECT = POINT+SIZE. UUID = BYTE[16].
PIXEL by colour depth: RGBA = 4 bytes R,G,B,A (non-premultiplied); Grayscale = 2 bytes Value,Alpha; Indexed = 1 byte index. TILE = 8/16/32-bit (in practice 32-bit) with bitmasks.

### 1.3 Header (128 bytes)
```
DWORD file size | WORD magic 0xA5E0 | WORD frames | WORD width | WORD height
WORD colour depth (32 RGBA, 16 Gray, 8 Indexed)
DWORD flags: 1 = layer opacity valid; 2 = blend mode/opacity valid for groups (composite groups separately); 4 = layers carry UUID
WORD speed (ms, DEPRECATED; use per-frame duration)
DWORD 0, DWORD 0
BYTE transparent palette index (indexed sprites, non-background layers)
BYTE[3] ignore
WORD number of colours (0 = 256 for old files)
BYTE pixel width, BYTE pixel height (0 on either = 1:1)
SHORT grid x, SHORT grid y, WORD grid w, WORD grid h (0 = no grid; default 16x16 in app)
BYTE[84] zero
```

### 1.4 Frame header (16 bytes), then chunks
```
DWORD bytes in frame | WORD magic 0xF1FA
WORD old chunk count (0xFFFF => use new field)
WORD frame duration ms | BYTE[2] zero
DWORD new chunk count (0 => use old field)
```
Chunk: `DWORD size (includes these 6 header bytes) | WORD type | data`. Skip unknown types by size (forward compatible).

Legacy rule: set every frame duration to header "speed"; override where frame duration > 0.

### 1.5 Chunk types (hex)
| Type | Name | Notes |
|---|---|---|
| 0x0004 | Old palette (0-255 RGB packets) | ignore if 0x2019 present; v1.3.5 writes it when <=256 colours and no alpha |
| 0x0011 | Old palette (0-63 range) | ignore if 0x2019 present |
| 0x2004 | Layer | first frame defines whole layer stack; see below |
| 0x2005 | Cel | image/linked/tilemap placement |
| 0x2006 | Cel extra | precise (fixed-point) bounds for latest cel |
| 0x2007 | Color profile | none / sRGB / ICC, optional fixed gamma |
| 0x2008 | External files | palettes, tilesets, extension names; ID referenced elsewhere |
| 0x2016 | Mask | DEPRECATED |
| 0x2017 | Path | never used |
| 0x2018 | Tags | followed by one user-data chunk per tag, in order |
| 0x2019 | Palette | new palette, supersedes the old ones |
| 0x2020 | User data | attaches to the previously read object |
| 0x2022 | Slice | keys per frame, 9-patch, pivot |
| 0x2023 | Tileset | tile images + flags |

**Layer 0x2004**: WORD flags (1 visible, 2 editable, 4 lock movement, 8 background, 16 prefer linked cels, 32 group collapsed, 64 reference layer); WORD type (0 image, 1 group, 2 tilemap); WORD child level (nesting relative to previous layer chunk); WORD default w, h (ignored); WORD blend mode (0 Normal, 1 Multiply, 2 Screen, 3 Overlay, 4 Darken, 5 Lighten, 6 ColorDodge, 7 ColorBurn, 8 HardLight, 9 SoftLight, 10 Difference, 11 Exclusion, 12 Hue, 13 Saturation, 14 Color, 15 Luminosity, 16 Addition, 17 Subtract, 18 Divide); BYTE opacity; BYTE[3] zero; STRING name; if tilemap: DWORD tileset index; if header flag 4: UUID. Layer index = order of appearance in file (NOTE.2); opacity only valid if header flag 1; blend/opacity for groups only if header flag 2.

**Cel 0x2005**: WORD layer index; SHORT x; SHORT y; BYTE opacity; WORD cel type (0 raw, 1 linked, 2 zlib image, 3 zlib tilemap); SHORT z-index; BYTE[5] zero; then
- type 0: WORD w, WORD h, raw pixels
- type 1: WORD frame position to link with
- type 2: WORD w, WORD h, zlib(raw pixels) (zlib per RFC1950/1951)
- type 3: WORD w tiles, WORD h tiles, WORD bits per tile (always 32), DWORD tile-id mask (e.g. 0x1fffffff), DWORD x-flip mask, DWORD y-flip mask, DWORD diagonal-flip mask, BYTE[10] reserved, zlib(TILE[] row-major)
- Z-order: render sort key `layerIndex + zIndex`, ties broken by smaller zIndex first (NOTE.5).

**Cel extra 0x2006**: DWORD flags (1 = precise bounds set); FIXED x, y, w, h; BYTE[16] zero.

**Color profile 0x2007**: WORD type (0 none, 1 sRGB, 2 ICC); WORD flags (1 = fixed gamma); FIXED gamma; BYTE[8]; if ICC: DWORD len + bytes.

**External files 0x2008**: DWORD count; BYTE[8]; entries { DWORD id; BYTE type (0 palette, 1 tileset, 2 extension name for properties, 3 extension name for tile management); BYTE[7]; STRING name or extension id `publisher/ExtName` }.

**Tags 0x2018**: WORD count; BYTE[8]; per tag { WORD from; WORD to; BYTE direction (0 fwd, 1 rev, 2 ping-pong, 3 ping-pong rev); WORD repeat (0 = unspecified: infinite in UI, once on export); BYTE[6]; BYTE[3] legacy RGB; BYTE 0; STRING name }. Real colour comes from the following user-data chunk.

**Palette 0x2019**: DWORD new size; DWORD first index; DWORD last index; BYTE[8]; entries { WORD flags (1 = has name); R,G,B,A bytes; optional STRING name }.

**Slice 0x2022**: DWORD key count; DWORD flags (1 nine-patch, 2 pivot); DWORD reserved; STRING name; per key { DWORD frame; LONG x; LONG y; DWORD w; DWORD h (0 = hidden); if 9-patch: LONG cx, cy; DWORD cw, ch; if pivot: LONG px, py }.

**Tileset 0x2023**: DWORD id; DWORD flags (1 external link, 2 tiles in file, 4 tile 0 = empty (modern; otherwise empty is 0xffffffff), 8/16/32 auto-match X/Y/D flips); DWORD tile count; WORD tile w; WORD tile h; SHORT base index (UI only, default 1); BYTE[14]; STRING name; if flag 1: DWORD external file id, DWORD tileset id in it; if flag 2: DWORD compressed length + zlib(image of size tileW x (tileH*count), tiles stacked vertically).

**User data 0x2020**: DWORD flags (1 text, 2 colour, 4 properties); STRING text; RGBA colour; properties block: DWORD total byte size (includes itself + count), DWORD map count, then maps { DWORD key (0 = user properties, else external-file entry ID of an extension), DWORD property count, properties { STRING name; WORD type; value } }.
Property types: 1 bool, 2 i8, 3 u8, 4 i16, 5 u16, 6 i32, 7 u32, 8 i64, 9 u64, 0x0A FIXED, 0x0B FLOAT, 0x0C DOUBLE, 0x0D STRING, 0x0E POINT, 0x0F SIZE, 0x10 RECT, 0x11 vector {DWORD n; WORD elem type (0 = mixed, each element then prefixed by its own WORD type); values}, 0x12 nested map {DWORD n; properties}, 0x13 UUID. Max nesting depth 128.
User-data attachment rules: applies to the last-read object (layer, cel, tags in order, slice, tileset). After a Tags chunk: one user-data per tag, same order. After a Tileset chunk: optional user data for the tileset, then one per tile in index order (absent in old files). v1.3 sprite-level user data: a user-data chunk in frame 0 right after the palette chunk.

### 1.6 Implementation notes for the plan
- Reader order of work: header -> frames -> chunks; keep "last object" pointer for user data and cel-extra. Layer tree rebuilt from child level. Linked cels reference earlier frame's cel in the same layer.
- Writer should emit: Layer chunks + palette + color profile + tags/slices/tilesets + external files in frame 0, cels per frame, and the new 0x2019 palette (skip old 0x0004/0x0011 unless back-compat needed). Compression = zlib (use `flate2` / `miniz_oxide`).
- Indexed sprites: transparent index applies only to non-background layers.
- Fidelity traps: premultiplication (stored non-premultiplied), group blend flag handling, z-index sort, empty-tile convention (flag 4), per-frame duration fallback, cel opacity x layer opacity compositing.
- Tile flip masks are given in the cel chunk, not fixed constants; always read them.

---------------------------------------------------------------------------

## 2. Other Aseprite-related formats

| Format | Shape | Confidence |
|---|---|---|
| Sprite sheet JSON, `json-hash` (default) | `{"frames": { "<name>": {frame{x,y,w,h}, rotated, trimmed, spriteSourceSize{x,y,w,h}, sourceSize{w,h}, duration} }, "meta": {app, version, image, format:"RGBA8888", size{w,h}, scale:"1", frameTags?, layers?, slices?}}` | verified against author's gists (db18e5747a4b6e208d3c) |
| Sprite sheet JSON, `json-array` | same but `"frames": [ {"filename": ..., frame, ...}, ...]` | verified (gist a32adb9248320326733a) |
| meta.frameTags | `[{"name","from","to","direction":"forward|reverse|pingpong|pingpong_reverse","repeat"?,"color"?}]` (CLI doc shows name/from/to; direction is emitted by recent versions) | medium |
| meta.layers (`--list-layers`/`--list-layer-hierarchy`) | `[{"name"}]`, plus `"group":"Group 1"` for children; recent versions add opacity, blendMode, color, data, cels | CLI doc shows name/group; rest medium |
| meta.slices (`--list-slices`) | `[{"name","color":"#0000ffff","keys":[{"frame","bounds":{x,y,w,h},"center":{...},"pivot":{x,y}}]}]` | verified from CLI doc |
| `--filename-format` / `--tagname-format` | template tokens `{fullname} {path} {name} {title} {extension} {layer} {tag} {innertag} {outertag} {frame} {frame1} {frame000} {tagframe} {duration}` | verified |
| Palette files | Aseprite reads/writes GIMP `.gpl`, JASC `.pal`, `.hex` (one RRGGBB per line, Lospec style), `.act`/`.col` (Photoshop/Animator), palettes inside `.png`/`.gif`/`.ase`/`.aseprite` images. Default palette lives at `~/.config/aseprite/palettes/default.ase` (confirmed on this machine: a normal .ase file). There is no dedicated ".aseprite-palette" format. | medium (from public docs/usage, not the source; verify each by round-trip tests) |
| Preset palettes | shipped as `.gpl` in the app data (LibreSprite's repo, GPL-2, has `data/palettes/*.gpl` as a visible sample) | verified for LibreSprite |
| `user.aseprite-layouts` | XML. Root `<layouts>`, one `<layout id= name=>` containing `<colorbar side= width=/>`, `<statusbar side=/>`, nested `<dock>` containing `<contextbar>`, `<toolbar>`, `<timeline side= width= height=/>`, `<workspace/>`. Local file seen: only the `_default_` layout. | verified from local file (shape only) |
| `user.aseprite-brushes` | XML, root `<brushes/>` (local file empty). Custom brushes are stored as entries inside it (not inspected beyond empty root) | shape only |
| `.aseprite-keys` | XML keyboard-shortcut file; Aseprite UI exports/imports it. Not present locally; format not verified. | low |
| `aseprite.ini` | INI preferences under `~/.config/aseprite/` | verified exists |
| Extensions | `package.json` (name, displayName, version, publisher, contributes: scripts/themes/palettes/languages/dithering-matrices/...) zipped as `.aseprite-extension`. Local example `extensions/godot-agent-studio-save-bridge/package.json` + `main.lua`. | verified |
| Session files / data-recovery | `~/.config/aseprite/sessions/` | exists; format unknown |
| Other image I/O | PNG, GIF (animated), JPEG, BMP, PCX, TGA, WebP, QOI, FLI/FLC, ICO, SVG import (via extension) | from README/CLI docs |

---------------------------------------------------------------------------

## 3. Lua scripting API (template for the MCP surface)

Licence: the API docs repo https://github.com/aseprite/api is MIT (Copyright Igara Studio S.A. 2018-2024, David Capello 2015-2018) - verified from LICENSE.txt. Reproducing names/signatures for design reference is therefore permitted with the MIT notice if text is copied; this file only lists names. API version: `app.apiVersion` = 39 as of Aseprite 1.3.17 (Changes.md).

### 3.1 Object model by area (names exactly as documented)

| Area | Members | Capability (MCP tool candidate) |
|---|---|---|
| app | site, range, cel, frame, image, layer, sprite, tag, tool, brush, editor, window, pixelColor, version, apiVersion, fgColor, bgColor, fgTile, bgTile, isUIAvailable, sprites, params, clipboard, command, preferences, fs, theme, os, uiScale, defaultPalette, events; alert(), tip(), open(), exit(), transaction(), refresh(), undo(), redo(), useTool(); legacy activeSprite/Layer/Frame/Cel/Image/Tag/Tool/Brush | document list, active-selection state, undo transaction grouping, tool-driven strokes |
| Sprite | Sprite(), width, height, bounds, gridBounds, pixelRatio, selection, filename, id, isModified, isValid, hasAssociatedFile, colorSpace, colorMode, spec, useLayerUuids, undoHistory.undoSteps/redoSteps, frames, palettes, layers, cels, tags, slices, tilesets, backgroundLayer, transparentColor, color, data, properties, tileManagementPlugin, events; :resize :crop :saveAs :saveCopyAs :close :loadPalette :setPalette :assignColorSpace :convertColorSpace :newLayer :newGroup :deleteLayer :newFrame :newEmptyFrame :deleteFrame :newCel :deleteCel :newTag :deleteTag :newSlice :deleteSlice :newTileset :deleteTileset :newTile :deleteTile :flatten | document CRUD, structure edits, export |
| Layer | sprite, name, opacity, blendMode, layers, parent, stackIndex, uuid, isImage/isGroup/isTilemap/isTransparent/isBackground/isEditable/isVisible/isContinuous/isCollapsed/isExpanded/isReference, cels, color, data, properties, tileset; :cel(frame) | layer tree read/write |
| Cel | sprite, layer, frame, frameNumber, image, bounds, position, opacity, zIndex, color, data, properties | per-frame placement and pixels |
| Image | Image(), :clone, id, version, width, height, bounds, colorMode, spec, cel, context, bytes, rowStride, bytesPerPixel; :clear :drawPixel :getPixel :drawImage :drawSprite :isEqual :isEmpty :isPlain :pixels :putPixel :putImage :putSprite :saveAs :resize :shrinkBounds :flip | raw pixel read/write, compositing, blend |
| ImageSpec | colorMode, width, height, colorSpace, transparentColor | |
| Frame | sprite, frameNumber, duration, previous, next | timing |
| Tag | sprite, fromFrame, toFrame, frames, name, aniDir, color, repeats, data, properties | animation tags |
| Slice | bounds, center (9-patch), color, data, properties, name, pivot, sprite | slices/9-patch/pivots |
| Palette | Palette(), #len, :resize :getColor :setColor, frame, :saveAs | palette edit |
| Color | Color(), alpha, red/green/blue, hsvHue/Saturation/Value, hslHue/Saturation/Lightness, hue, saturation, value, lightness, index, gray, rgbaPixel, grayPixel | colour conversions |
| Selection | Selection(), bounds, origin, isEmpty; :deselect :select :selectAll :add :subtract :intersect :contains | masks |
| Tileset | name, baseIndex, color, data, properties, grid; :tile(i), :getTile | tilemap data |
| Tile | index, image, color, data, properties | |
| Range | type, isEmpty, sprite, layers, frames, cels, images, editableImages, colors, tiles, slices; :contains :containsColor :clear | timeline/palette selection |
| Site | sprite, layer, cel, frame, frameNumber, image, tilemapMode, tilesetMode | current context |
| Brush / Tool / Ink | Brush(), type, size, angle, image, center, pattern, patternOrigin; Tool.id; enums BrushType, BrushPattern, Ink | tool state |
| Editor | sprite, spritePos, mousePos, zoom, scroll, :askPoint, :cancel | viewport (UI only) |
| Dialog | Dialog(), data, bounds, autofit, sizeHint; :button :check :close :color :combobox :entry :label :modify :newrow :number :radio :separator :shades :show :slider :tab :endtabs :file :canvas :repaint; GraphicsContext, MouseEvent/KeyEvent/TouchEvent | UI scripting (not MCP-relevant, but plugin UI) |
| Events | :on(name, fn), :off; on app (`sitechange`, `beforecommand`, `aftercommand`, ...), Sprite (`change`, `filenamechange`), Dialog/Window | MCP notifications/subscriptions |
| Plugin | name, displayName, version, path, preferences; :newCommand :newMenuGroup :newMenuSeparator :newFileFormat | extension points |
| Timer, WebSocket | Timer(), :start :stop, interval, isRunning; WebSocket with :connect :sendText :sendBinary :sendPing :close | async/network |
| app.clipboard | text, image, content, hasText, hasImage, clear() | |
| app.fs | pathSeparator, filePath/fileName/fileExtension/fileTitle/filePathAndTitle, normalizePath, joinPath, currentPath, appPath, tempPath, userDocsPath, userConfigPath, isFile, isDirectory, fileSize, listFiles, makeDirectory, makeAllDirectories, removeDirectory | sandboxed file ops |
| app.os | name, version, fullName, windows, macos, linux, x64, x86, arm64 | |
| app.preferences | tool(Tool), document(Sprite) trees | settings |
| app.theme | color, dimension, styleMetrics | |
| Misc types | Point, Size, Rectangle, Version, Uuid, json (encode/decode), enums: Align, AniDir, BlendMode, ColorMode, FilterChannels, FlipType, MouseButton, MouseCursor, PixelColor, RangeType, SelectionMode, SpriteSheetType, SpriteSheetDataFormat, TilemapMode, TilesetMode, WebSocketMessageType | |
| Properties | `.properties` on Sprite/Layer/Cel/Tag/Slice/Tileset/Tile: user properties + per-extension properties map (matches the user-data properties block) | |

Security model worth copying: scripts must be granted file/exec/network access per script via prompt (Changes.md v1.3.17 fixed a grant-leak between scripts) - relevant to MCP permissions.

### 3.2 app.command names (from app_command.md; those marked * need UI, [P] have a documented parameter page)
About*, AddColor[P], AdvancedMode*, AutocropSprite[P], BackgroundFromLayer, BrightnessContrast[P], Cancel, CanvasSize[P], CelOpacity[P], CelProperties*, ChangeBrush[P]*, ChangeColor[P]*, ChangePixelFormat[P], Clear, ClearCel, ClearRecentFiles, CloseAllFiles*, CloseFile*, ColorCurve[P], ColorQuantization[P], ContiguousFill, ConvolutionMatrix[P], Copy, CopyColors[P], CopyMerged, CropSprite[P], Cut, DeselectMask, Despeckle[P], DeveloperConsole*, DiscardBrush*, DuplicateLayer, DuplicateSprite[P], DuplicateView, Exit, ExportSpriteSheet[P], ExportTileset[P], Eyedropper, Fill, FitScreen, FlattenLayers[P], Flip[P], FrameProperties[P], FrameTagProperties[P], FullscreenPreview, GotoFirstFrame, GotoFirstFrameInTag, GotoFrame, GotoLastFrame, GotoLastFrameInTag, GotoNextFrame, GotoNextFrameWithSameTag, GotoNextLayer, GotoNextTab, GotoPreviousFrame, GotoPreviousFrameWithSameTag, GotoPreviousLayer, GotoPreviousTab, GridSettings, Home, HueSaturation[P], ImportSpriteSheet[P], InvertColor[P], InvertMask, KeyboardShortcuts, Launch, LayerFromBackground, LayerLock, LayerOpacity[P], LayerProperties, LayerVisibility, LinkCels, LoadMask[P], LoadPalette[P], MaskAll, MaskByColor[P], MaskContent, MergeDownLayer, ModifySelection[P], MoveCel, MoveColors[P], MoveMask[P], NewBrush, NewFile[P], NewFrame[P], NewFrameTag, NewLayer[P], NewSpriteFromSelection, OpenBrowser, OpenFile[P], OpenGroup, OpenInFolder, OpenScriptFolder, OpenWithApp, Options[P], Outline[P], PaletteEditor, PaletteSize[P], Paste, PasteText, PixelPerfectMode, PlayAnimation, PlayPreviewAnimation, Redo, Refresh, RemoveFrame, RemoveFrameTag, RemoveLayer, RemoveSlice, RepeatLastExport, ReplaceColor[P], ReselectMask, ReverseFrames, Rotate[P], RunScript, SaveFile[P], SaveFileAs, SaveFileCopyAs, SaveMask[P], SavePalette[P], Scroll[P], ScrollCenter, SelectionAsGrid, SelectTile, SetColorSelector, SetInkType[P], SetLoopSection[P], SetPaletteEntrySize, SetSameInk, ShowAutoGuides, ShowBrushPreview, ShowExtras, ShowGrid, ShowLayerEdges, ShowOnionSkin, ShowPixelGrid, ShowSelectionEdges, ShowSlices, SliceProperties, SnapToGrid, SpriteProperties, SpriteSize[P], Stroke, SwitchColors, SymmetryMode, TiledMode, Timeline, TogglePreview, ToggleTimelineThumbnails, ToggleWorkspaceLayout, Undo, UndoHistory, UnlinkCel, Zoom[P].
(The page also lists `CommandName` as a placeholder.) Note: the command list is the app's menu/command registry; it exposes features like Outline, Despeckle, ConvolutionMatrix, ColorCurve, Rotate, Flip, Replace Color, symmetry, tiled mode, pixel-perfect, onion skin.

### 3.3 ExportSpriteSheet parameters (public doc)
ui, recent, askOverwrite, type (HORIZONTAL/VERTICAL/ROWS/COLUMNS/PACKED), columns, rows, width, height, bestFit, textureFilename, dataFilename, dataFormat (JSON_HASH/JSON_ARRAY), filenameFormat, borderPadding, shapePadding, innerPadding, trimSprite, trim, trimByGrid, extrude, ignoreEmpty, mergeDuplicates, openGenerated, layer, tag, splitLayers, splitTags, splitGrid, listLayers, listTags, listSlices, fromTilesets.

### 3.4 Gaps the Lua API has that MCP should fill
- Drawing is indirect: either write pixels to an Image or `app.useTool{tool, color, brush, points, ...}` (tool stroke simulation). MCP needs first-class stroke/shape/fill/gradient/selection-transform operations that return diffs.
- No structured read of undo history (only step counts), no render-to-bytes of arbitrary viewport state, UI-only commands, no event stream beyond Lua callbacks. MCP should add: snapshot/preview image return (PNG base64), transactional batches, undo/redo labelled steps, resource subscriptions, UI state (zoom, scroll, active tool, panels), and headless operation.

---------------------------------------------------------------------------

## 4. CLI (https://www.aseprite.org/docs/cli/)

`aseprite [OPTIONS] [FILES]...` - options apply to files/state given earlier on the line, with exceptions (e.g. `--split-layers`, `--layer`, `--ignore-layer`, `--oneframe`, `--list-*` must precede the sprite).

Options (verified, full list from the page): `--shell` (Lua REPL), `-b/--batch`, `-p/--preview` (dry run), `--save-as <file>`, `--palette <file>`, `--scale <n>`, `--dithering-algorithm none|ordered|old`, `--dithering-matrix bayer8x8|bayer4x4|bayer2x2|<extension id>`, `--color-mode rgb|grayscale|indexed`, `--data <json>` (empty filename = stdout), `--format json-hash|json-array`, `--sheet <png>`, `--sheet-type horizontal|vertical|rows|columns|packed`, `--sheet-width`, `--sheet-height`, `--sheet-columns`, `--sheet-rows`, `--sheet-pack`, `--split-layers`, `--split-tags`, `--split-slices`, `--split-grid`, `--layer/--import-layer <name>` (repeatable, `group/layer` paths), `--all-layers`, `--ignore-layer <name>`, `--tag/--frame-tag <name>`, `--frame-range from,to`, `--ignore-empty`, `--merge-duplicates`, `--border-padding`, `--shape-padding`, `--inner-padding`, `--trim`, `--trim-sprite`, `--trim-by-grid`, `--extrude`, `--crop x,y,w,h`, `--slice <name>`, `--filename-format`, `--tagname-format`, `--script <file>`, `--script-param name=value` (-> app.params), `--list-layers`, `--list-layer-hierarchy`, `--list-tags`, `--list-slices`, `--oneframe`, `--export-tileset`, `-v/--verbose` (log to ~/.config/aseprite/aseprite.log), `--debug`, `--noinapp`, `-?/--help`, `--version`.
Use cases: convert ase->png/gif, frame sequences (`frame1.png` pattern auto-numbers), multi-scale export, per-layer export, tag export (`{tag}`), sprite sheet + data JSON, recolour via `--palette`, mode conversion with dithering.
Implication: a Rust CLI with these flag names would give drop-in batch compatibility; the sheet exporter (packing, trim, padding, extrude, merge duplicates) is a self-contained module whose behaviour is documented only by outputs/examples, so test against real Aseprite output (as black-box fixtures) rather than source.

---------------------------------------------------------------------------

## 5. Licensing landscape

### 5.1 Aseprite
- Until 2016-08-26: GPLv2. Since then: proprietary EULA by Igara Studio S.A. (verified: https://raw.githubusercontent.com/aseprite/aseprite/main/EULA.txt).
- EULA key terms: install/use on a validly licensed OS; backups allowed; may NOT distribute copies (evaluation builds freely distributable); no reverse engineering / decompiling / disassembling "except where law expressly permits"; no rental; **2(g) Source code: "You may only compile and modify the source code ... for your own personal purpose or to propose a contribution to the SOFTWARE PRODUCT."** All rights not granted are reserved.
- Consequence: Aseprite source must not be copied, adapted or ported into a new project. Reading it is a clean-room hazard (contamination argument) even though reading is not explicitly banned; the brief's rule (no source, no binary inspection) is the safe course.
- Not covered by that restriction and freely usable: the file-format spec (published for interoperability), the Lua API docs repo (MIT), the CLI docs and public behaviour (facts and functional behaviour are not copyrightable; the UI "look and feel" and the Aseprite name/logo are). Do not use the Aseprite trademark/branding or imply compatibility endorsement; "reads/writes .aseprite files" is nominative description.
- Third-party libs Aseprite bundles are individually open (zlib, libpng, Lua, skia, etc.; list at https://github.com/aseprite/aseprite/blob/main/docs/LICENSES.md) - irrelevant to us except as precedent.

### 5.2 LibreSprite (https://github.com/LibreSprite/LibreSprite)
- GPL-2.0 (verified via GitHub licence API); forked from the last GPLv2 Aseprite commit (03be4aa23db465219962f4c62410f628e7392545) before the 2016 relicense.
- State (GitHub API, checked 2026-10-11): ~8.5k stars, ~92 open issues, last push 2026-10-10, latest tagged release "v1.3" published 2026-10-02 (prior v1.2 2025-03-02, v1.1 2023-12-03, v1.0 2021-11-16). Actively maintained but small team (contributors named in release notes: Nidrax, Samueru-sama, etc.). Release notes are mostly build/packaging/UI fixes.
- Feature position: Aseprite ~1.1.x era (2016). Source tree has `src/script` (V8-based JavaScript scripting, not Lua), file formats ase/bmp/fli/gif/ico/jpeg/pcx/png/qoi/tga/webp (+ web/Android/macOS builds). Repo tree contains NO tilemap/tileset files (searched the full 1674-entry git tree; none match). Therefore, vs Aseprite 1.3: lacks tilemap layers/tilesets, slices-era features (check), UUID layers, user-data properties, extension API (JS only), color-profile handling, many newer tools/commands. The .aseprite files it writes follow the pre-1.3 spec; it cannot represent tilesets or properties. Confidence: tilemaps absent = high (file-tree evidence); other missing features inferred from fork date = medium.
- Legal implication of using LibreSprite code: any code copied/ported (even translated into Rust) is a derivative work and must be GPL-2.0(-only or or-later as the file states) in the combined distributed work. For a closed or permissively licensed product that is unacceptable; for a GPL project it is fine, but GPL-2.0 is incompatible with Apache-2.0 and with GPLv3-only code. Also contaminates the licence of anything statically linked. Copying only ideas, algorithms described in documentation, and the file-format spec does not trigger GPL. Note LibreSprite contains the pre-2016 Aseprite code written by David Capello and contributors under GPL-2; it is legitimately GPL, but Igara retains the right to say nothing about that older code. A permissive Rust app should treat LibreSprite as behaviour reference only, with no code reuse and ideally no source reading if strict clean-room is wanted.

### 5.3 Pixelorama (https://github.com/Orama-Interactive/Pixelorama)
- MIT (verified), Godot 4.7.2, GDScript; very active (stars ~10.5k, last push 2026-10-08, releases v1.2.3 on 2026-09-15, v1.2.2 on 2026-09-09, v1.2.1 on 2026-08-19).
- Feature list (README): per-button tool mapping, layers+frames timeline, onion skin, audio sync, frame tags, draw during playback, pixel-perfect lines, indexed mode, pixel-art rotation/scaling (cleanEdge, OmniScale, rotxel), tilemap layers (rectangular, isometric, hexagonal), clipping masks, non-destructive layer effects (outline, gradient map, drop shadow...), palette management (premade/import/custom), image effects, guides, rectangular + isometric grids, tile mode, autosave backups, customisable UI, export PNG/APNG/spritesheets/GIF/video; also has its own `.pxo` project format and an extension system.
- MIT code can be read, adapted and ported (keep the copyright + permission notice). Best candidate to consult for algorithms (pixel-perfect, cleanEdge, rotxel, tilemap layer logic) and UX, in a licence-compatible way. Any ported snippet needs attribution in the repo's NOTICE.

### 5.4 What is legally safe as reference (summary table)
| Source | Read | Port code | Reproduce docs text |
|---|---|---|---|
| Aseprite source (EULA) | do not (clean-room) | NO | NO |
| Aseprite binary | do not | NO | n/a |
| ase-file-specs.md | yes | implement it | cite/summarise; no explicit licence, treat as published-for-interop |
| aseprite/api repo (MIT) | yes | n/a | yes with MIT notice |
| aseprite.org docs/CLI pages | yes | n/a | summarise, do not wholesale copy (site terms) |
| LibreSprite (GPL-2.0) | yes, but taints a clean-room claim | only if the new project is GPL-2 compatible | n/a |
| Pixelorama (MIT) | yes | yes with notice | yes |
| asefile / aseprite-loader crates (MIT / MIT-or-Apache) | yes | yes | yes |
Observable behaviour (how a tool should work, output formats) is free to re-implement. Keep a clean-room log: sources consulted, none being Aseprite source.

---------------------------------------------------------------------------

## 6. Rust crates for .aseprite

Checked on crates.io API 2026-10-11.

| Crate | Version / last release | Licence | Downloads (total / recent) | Read | Write | Tilemaps | Notes |
|---|---|---|---|---|---|---|---|
| asefile (alpine-alpaca) | 0.3.8, 2024-03-14 (changelog last entry: tag repeat) | MIT | 81.9k / 16.6k | yes | no | no mention (README says tileset/color-profile unsupported; targets spec as of Aseprite 1.2.25) | most used; image-crate integration; effectively stale: 2+ years |
| aseprite-loader (bikeshedder) | 0.4.2, 2026-02-19 | MIT OR Apache-2.0 | 31.1k / 8.9k | yes, zero-copy parser; `render_frame(...LayerSelection)` | no | `Unreleased: Add support for parsing UserData chunks`; 0.4.1 fixed properties-map size bug; tilemap support not documented in README (needs source check) | `unsafe` forbidden, MSRV 1.70, active |
| ase2 (sermuns) | 0.2.0, 2026-04-16 | not checked | 25 / 9 | yes | unclear | unknown | very new, tiny; repo path 404 on README fetch, treat as unverified |
| bevy_aseprite_ultra | 0.10.0, 2026-10-11 | not checked | 33.7k / 2.3k | reader for Bevy | no | unknown | consumer crate, not a general lib |
| aseprite (ggez) | 0.1.3, 2017-08-02 | - | 28.3k | yes | no | no | abandoned |
| ase (matheuslessarodrigues) | 0.1.3, 2019-01-11 | - | 5.8k | yes | no | no | abandoned |
| aseprite-reader, ase-rs, aseprite-rs, aseprite-writer | not on crates.io under those names | | | | | | |

Takeaways: no maintained Rust crate **writes** .aseprite files, and none documents full v1.3 coverage (tilemaps, properties, slices, tileset external links). Plan to write our own reader/writer crate (the spec in section 1 is sufficient), using `aseprite-loader` and `asefile` as MIT/Apache cross-checks and as test oracles. Use `flate2` or `miniz_oxide` for zlib.

---------------------------------------------------------------------------

## Open questions / blockers
1. Spec document has no explicit licence text; low practical risk but worth noting in project NOTICE. Resolving definitively would require contacting Igara Studio.
2. `.aseprite-keys` format, brushes XML schema (non-empty), session/recovery format, palette read/write extensions list were not verified from public specs; derive from black-box output (save a file in Aseprite and inspect) rather than source.
3. LibreSprite feature gaps beyond tilemaps are inferred, not tested.
4. aseprite-loader tilemap/tileset support was not confirmed from docs.
5. WebFetch was not used; all pages came from curl (raw/GitHub API/crates.io). aseprite.org docs HTML for sprite-sheet and palette pages had no extractable body text; JSON shapes were confirmed via the author's gists and the CLI page instead.
6. GitHub MCP server is unavailable this session (401); no impact, REST API via curl worked.
