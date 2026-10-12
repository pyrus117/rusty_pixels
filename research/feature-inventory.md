# Aseprite feature and workflow inventory (v1.3.18.6)

Clean-room requirements input for a from-scratch Rust pixel-art editor. Every statement below is derived from the shipped plain-text data files (gui.xml, pref.xml, strings/en.ini, widgets/*.xml, extension manifests, convmatr.def, palettes), the `--help` output, the bundled README, and the project owner's preference file (the personal-usage notes drawn from it are kept outside the repository). No binary inspection was done. Where the data files give only a name (e.g. "Octree" RGB mapping) and not behaviour, the behaviour is flagged **[behaviour not in data files]**: it comes from general knowledge of Aseprite and must be specified by us, not copied.

Provenance legend: **[data]** = read directly from a data file. **[inferred]** = deduced from names/options. **[general]** = common Aseprite knowledge not confirmed by data files.

---

## 1. Document model

### 1.1 Sprite
- Canvas of fixed width x height in pixels. New Sprite dialog defaults: 32x32, RGBA, transparent background. **[data: pref new_file]**
- Color mode: RGB(A) (32 bpp), Grayscale (gray + alpha, 16 bpp), Indexed (8 bpp, palette up to 256 entries). **[data: new_sprite tooltips]**
- Pixel aspect ratio: Square 1:1, Double-wide 2:1, Double-high 1:2 (stored per sprite; used by "Apply pixel ratio" on export and display). **[data]**
- Transparent color index (indexed mode only): palette entry treated as transparent in every non-background layer. **[data: sprite_properties]**
- Color profile per sprite: none / sRGB / embedded ICC; Assign and Convert actions in Sprite Properties. **[data]**
- Optional "Create UUID for layers": each layer gets a UUID persisted in .aseprite. **[data]**
- Sprite user data: text + color (the "User Data" button on Sprite/Layer/Cel/Tag/Slice properties). **[data]**
- Grid per sprite: bounds rect (x, y, w, h; default 16x16 at 0,0), colour, opacity, auto-opacity. Pixel grid has its own colour/opacity. Checkerboard background settings are per-document. **[data: pref document.grid, pixel_grid, bg]**
- Per-document persisted view state: current frame, current layer, tiled mode, symmetry mode + axes, onion-skin settings, timeline first-frame number, thumbnails, preview zoom/scroll, which "show" overlays are on. **[data: pref document.*]**
- Multiple Views onto one sprite (Duplicate View) with a shared document. **[data]**

### 1.2 Layers
Layer kinds **[data]**:
- Image (transparent) layer.
- Background layer: opaque, cannot be moved, cannot be deleted down to zero layers; converted to/from a normal layer via Layer > Convert To (Background / Layer) and `LayerFromBackground` / `BackgroundFromLayer`.
- Group layer: hierarchical, open/closed (OpenGroup), contains child layers; flatten/merge target.
- Tilemap layer: references a Tileset; cels hold tile indices instead of pixels. Created via New Tilemap Layer (asks for tileset: new or existing) or Convert To Tilemap.
- Reference layer: image-only, unmodifiable and non-transformable, created from file or clipboard (New Reference Layer / Paste Special); usable as eyedropper sample source ("First Reference Layer").

Layer properties **[data: layer_properties, commands]**: name (<=256 chars), blend mode, opacity (0-255, shown as % or 0-255 per preference), visible, locked ("Lock Layers"), solo, UUID (read-only, optional), user data, tileset (tilemap layers), z-index at the cel level.

Blend modes (19) **[data]**: Normal, Darken, Multiply, Color Burn, Lighten, Screen, Color Dodge, Addition, Overlay, Soft Light, Hard Light, Difference, Exclusion, Subtract, Divide, Hue, Saturation, Color, Luminosity.

Layer operations **[data]**: new (above/below active), new group, new via copy, new via cut, new from clipboard, new reference layer from file, new tilemap layer, delete, duplicate, merge down, flatten (all or visible-only), convert (background/layer/tilemap), open/close group, go to previous/next layer, lock, visibility, solo, set opacity to preset steps, "Toggle other layers opacity" (preview inactive layers dimmed), flash layer when selected (experimental).

### 1.3 Frames
- Ordered list; each frame has a duration in milliseconds (default per-frame; "Constant Frame Rate" sets all). Frame Properties dialog: frame number (read-only), duration ms. **[data]**
- GIF/other formats warn about <20 ms durations and non-multiples of 10 ms. **[data: alerts]**
- Timeline "first frame" number base configurable (default 1; 0-based option). **[data]**

### 1.4 Cels
- A cel = the image (or tile grid) of one layer at one frame, with x/y position, opacity (0-255), z-index (cel-level draw order tweak), and user data. **[data: cel_properties]**
- Linked cels: several frames share one image (LinkCels, UnlinkCel); duplicating frames can copy or link cels. **[data]**
- Cel operations: Clear Cel, Link/Unlink, Move/Copy by drag with a Move/Copy/Cancel popup, set cel opacity presets, Cel Properties. **[data]**
- Multi-cel transform: transforms apply to all selected cels in the timeline range (preference). **[data]**

### 1.5 Tags (frame tags)
Properties **[data: tag_properties, pref]**: name, from frame, to frame, colour, animation direction (Forward, Reverse, Ping-pong, Ping-pong Reverse), optional repeat count (checkbox "Repeat" + number), user data. Tags may nest/overlap (setting "Play subtags & repetitions"). Navigation: go to first/last frame in tag, previous/next frame with same tag, loop-through-tag for onion skin.

### 1.6 Slices
Properties **[data: slice_properties]**: name, bounds (x,y,w,h), optional 9-slice "center" rect (x,y,w,h), optional pivot (x,y), colour (default blue), user data. Slice tool creates/moves/resizes them; "Transform pixels along slice modification" option; Select/Deselect All Slices; Duplicate/Delete Slice; slice keys (per-frame keyframes, flagged "obsolete behaviour" via pref `slices.use_keys`). Slices are exportable as crops (CLI `--slice`, `--split-slices`) and listed in JSON metadata.

### 1.7 Palettes
- Up to 256 entries for indexed; RGBA entries incl. alpha. Palette can change between frames (GIF warns "Palette changes between frames"). **[data]**
- Entry ops: add fg/bg colour, move, copy, remap (re-map indexes), sort, gradient, reverse, resize palette (Palette Size dialog), select used/unused colours and tiles. See section 5.
- File formats: .gpl (GIMP palette with RGBA channel header), .ase (default palette ships as a user file), PNG/GIF/other image-as-palette, preset palettes. **[data/inferred]**

### 1.8 Tilesets and tilemaps
- Tileset: tile grid size (default grid), name, base index (default 1, 0 allowed), allowed flips for auto-matching (X, Y, Diagonal). Tilesets are owned by tilemap layers, can be shared, duplicated and deleted (blocked if used by tilemaps). **[data: tileset_selector, alerts]**
- Tileset modes when drawing on a tilemap: Manual (edit existing tiles only), Auto (modify/reuse tiles, create/delete as needed), Stack (never modify, stack new tiles). **[data]**
- Tile selection tools in the Color Bar tile view; tile fg/bg entries; Remap Tiles, Clear Tiles, Resize Tiles, Drag And Drop Tiles, Copy/Move Tiles; Select Tile (replace/add/subtract/intersect); double-click selects a grid tile; show tile numbers; Toggle Tiles Mode. Cache compressed tilesets option. Export Tileset command. **[data]**

### 1.9 User data / properties
Every object that can have it (sprite, layer, cel, tag, slice) exposes a User Data button: free text plus a colour. Visibility of an inline user-data editor per object type is a preference. Scripting exposes arbitrary typed properties (not described in data files). **[data + general]**

### 1.10 Native file ".aseprite"
Keeps everything: layers, groups, tilemaps, tags, slices, palettes (changing), user data, UUIDs, linked cels, colour profile. Cel content storage option: Compressed / Keep as in file / Raw. Sidecar `.aseprite-data` is used by theme sheets. **[data]**

---

## 2. Tools

26 tools in 12 groups defined in gui.xml. Table columns are the data-file attributes (ink = what the tool does to pixels; controller = input model; point shape = footprint; intertwine = how successive points join; trace policy = how the stroke accumulates).

| id | name | key | ink | controller | point shape | intertwine | trace policy | fill | def brush size |
|---|---|---|---|---|---|---|---|---|---|
| rectangular_marquee | Rectangular Marquee Tool | M | selection | two_points | pixel | as_rectangles | last | always |  |
| elliptical_marquee | Elliptical Marquee Tool | Shift+M | selection | two_points | pixel | as_ellipses | last | always |  |
| lasso | Lasso Tool | Q | selection | freehand | pixel | as_lines | accumulate | always |  |
| polygonal_lasso | Polygonal Lasso Tool | Shift+Q | selection | point_by_point | pixel | as_lines | last | always |  |
| magic_wand | Magic Wand Tool | W | selection | one_point | floodfill |  | accumulate | always |  |
| pencil | Pencil Tool | B | paint | freehand | brush | as_lines | accumulate |  |  |
| spray | Spray Tool | Shift+B | paint | freehand | spray |  | overlap |  |  |
| eraser | Eraser Tool | E | L:eraser R:replace_fg_with_bg | freehand | brush | as_lines | accumulate |  | 8 |
| eyedropper | Eyedropper Tool | I | L:pick_fg R:pick_bg | freehand | pixel |  |  |  |  |
| zoom | Zoom Tool | Z | zoom | freehand |  |  |  |  |  |
| hand | Hand Tool | H | scroll | freehand |  |  |  |  |  |
| move | Move Tool | V | move | freehand |  |  |  |  |  |
| slice | Slice Tool | Shift+C | slice | two_points | pixel | as_rectangles | last | none |  |
| paint_bucket | Paint Bucket Tool | G | paint | one_point | floodfill |  | accumulate |  |  |
| gradient | Gradient Tool | Shift+G | gradient | two_points | floodfill | first_point | last |  |  |
| line | Line Tool | L | paint | two_points | brush | as_lines | last |  |  |
| curve | Curve Tool | Shift+L | paint | four_points | brush | as_bezier | last |  |  |
| rectangle | Rectangle Tool | U | paint | two_points | brush | as_rectangles | last | optional |  |
| filled_rectangle | Filled Rectangle Tool | U | paint | two_points | brush | as_rectangles | last | always |  |
| ellipse | Ellipse Tool | Shift+U | paint | two_points | brush | as_ellipses | last | optional |  |
| filled_ellipse | Filled Ellipse Tool | Shift+U | paint | two_points | brush | as_ellipses | last | always |  |
| contour | Contour Tool | D | paint | freehand | brush | as_lines | accumulate | always |  |
| polygon | Polygon Tool | Shift+D | paint | point_by_point | brush | as_lines | last | always |  |
| blur | Blur Tool | R | blur | freehand | brush | as_lines | accumulate |  | 16 |
| jumble | Jumble Tool | R | jumble | freehand | brush | as_lines | overlap |  | 16 |
| text | Text Tool | T | text | two_points | pixel | as_rectangles | last |  |  |


Quick-tools (held modifiers): Eyedropper = Alt, Move = Ctrl (Cmd on mac), Hand = Space. Tool-group keys cycle through tools sharing a key (U: rectangle/filled rectangle, Shift+U: ellipse/filled ellipse, R: blur/jumble).

Tool group membership (a toolbar slot shows one tool of the group) **[data]**: selection (rect marquee, ellipse marquee, lasso, polygonal lasso, magic wand), pencil (pencil, spray), eraser, eyedropper, view (zoom, hand), move (move, slice), paint bucket (paint bucket, gradient), perfect traces (line, curve), shapes (rectangle, filled rectangle, ellipse, filled ellipse), contours (contour, polygon), effects (blur, jumble), text.

### 2.1 Context-bar options (per tool; from pref.xml `tool.*` and en.ini [context_bar])
Shared per-tool prefs: opacity (0-255), tolerance (0-255), contiguous, filled, filled preview, ink, freehand algorithm, corner radius, brush {type, size, angle}, spray {width, speed}, floodfill {stop at grid, refer to, pixel connectivity}, dynamics {...}. Each tool stores its own copy (`tool.<id>.*`), except where "Same in all tools" is set for ink and dynamics.

**Brush**
- Types: Circle, Square, Line (angled), Custom (captured from selection with New Brush, Ctrl+B), plus image brushes. Size 1..N px, angle degrees. Flip X/Y/Diagonal and Rotate 90 CW of the current brush by key. Brush slots 1-9 (Alt+1..9) storing parameters: type, size, angle, fg/bg/image colour, ink type, ink opacity, shade, pixel-perfect; "Locked" and "Delete/Delete All". Pattern modes for custom brushes: default, aligned to source, aligned to destination, paint brush ("Paint brush"). Discard Brush (Esc / on eyedropper use optional). Saved brushes persist in a user brush file. **[data]**
- Brush preview modes: None, Edges, Full, Full with all tools, Full + edges; Tile preview same set; brush preview also in Preview window. **[data]**
- Ctrl+Alt+drag changes brush size interactively (drag vector 4,0). Mouse wheel can change brush size, angle, tool, layer, frame, ink type, ink opacity, layer/cel opacity, fg/bg palette index or tile index, colour channels (alpha, HSL H/S/L, HSV H/S/V), zoom, scroll (vertical/horizontal). **[data: keyboard_shortcuts]**

**Ink types (user-selectable, 5)** **[data]**: Simple Ink, Alpha Compositing, Copy Alpha + Color, Lock Alpha, Shading. Internal inks used by tools: paint, eraser, replace_fg_with_bg, pick_fg, pick_bg, selection, zoom, scroll, move, slice, gradient, blur, jumble, text.
- Shading ink: a "shade" (ordered list of palette colours), Reverse Shade, Save Shade, select colours from palette; left click moves along the ramp one direction, right click the other. **[data + general]**
- Opacity slider = paint intensity.

**Freehand algorithm**: Regular, Pixel-perfect (removes L-shaped corner pixels), plus a third value exists (enum "FreehandAlgorithm" not fully listed in data files; likely Dots). **[inferred]** Stabilizer (radius/factor) to smooth strokes.

**Dynamics (pressure/velocity)** **[data: dynamics.xml, strings]**: sensors Pressure and Velocity can each drive Size, Angle and Gradient (fg-to-bg colour, direction from/to). Per-parameter min/max (size, angle). Sensor thresholds: min/max pressure (default 0.1/0.9), min/max velocity (0.1/0.9). Stabilizer toggle + factor. Dither matrix name for gradient dynamics (`matrix_name`).

**Symmetry** **[data]**: modes Horizontal, Vertical, Both, Right diagonal (45 deg), Left diagonal (-45 deg), Both diagonals, All (bit-flag enum 1,2,4,8). Per-document x/y axis positions (floating), "Reset symmetry to centre / to view centre", toggles per axis from the symmetry popup.

**Spray**: width (default 16) and speed (default 32). Trace policy "overlap".

**Paint bucket / floodfill** **[data]**: contiguous vs global; stop at grid (Never / If visible / Always); refer to active layer vs all (visible) layers; pixel connectivity 4 or 8; tolerance; pattern alignment; fill modes with tilemaps. Gradient tool: Linear and Radial, dithering via the matrix (Bayer 2x2/4x4/8x8 or custom bitmap), fg-to-bg colour range, ink aware.

**Shapes**: Rectangle/Ellipse outline or filled (Fill + Stroke + Stroke Width in the shape UI), corner radius for rectangles (hold C or show radius field), Square aspect (Shift), draw from centre (Ctrl), move origin while drawing (Space), rotate shape (Alt). Curve tool = 4-point Bezier. Contour = freehand closed fill. Polygon = point-by-point filled polygon.

**Eyedropper** **[data]**: Pick channel modes: Color+Alpha, Color, Alpha, RGBA, RGB, HSVA, HSV, HSLA, HSL, GrayA, Gray, Index. Sample: All layers / Current layer / First reference layer. Left picks foreground, right picks background. Option to discard custom brush on pick.

**Selection tools & modes** **[data]**: replace / add (Shift) / subtract (Alt+Shift) / intersect (Ctrl+Shift); right button subtracts. Magic wand: tolerance, contiguous, 4/8 connectivity, sample current/all layers. Lasso freehand; polygonal lasso. Mask By Color (Select > Color Range) with tolerance + preview. Selection modify: Border, Expand, Contract by N pixels using Circle or Square brush. Load/Save selection as `.msk`. Select all, deselect, reselect, invert, "mask content" (transform). Snap-to-grid for selection. Move selection edges (drag edge), move on add mode, opaque/transparent mode with auto adjust per layer type, transparent colour for transparent mode, keep selection after delete, auto show selection edges. Selection as Grid. New Brush from selection. New Sprite from Selection.
- Transform of selection contents **[data]**: translate, scale (handles; Shift keeps aspect; Alt scales from centre/pivot), rotate (angle snap with Shift; pivot with visible position among 9 anchors: NW, N, NE, W, Center, E, SW, S, SE), skew, fine/sub-pixel control (Ctrl), copy-on-drag (Ctrl), snap to grid (Alt), lock axis (Shift). Rotation algorithms: Fast Rotation vs RotSprite ("Force RotSprite even for right angles"). Apply (Enter) / apply & deselect / cancel (Esc) / undo within transform. Context bar edit fields: X, Y position, width, height, angle, skew.

**Move tool**: moves the layer/cel content (or selection). Auto Select Layer (Ctrl) picks the layer under the cursor. Moving across multiple selected cels.

**Slice tool**: drag to create rect; context bar: slice properties, delete, select/deselect all, transform-pixels toggle, 9-slice and pivot editing in properties.

**Blur tool**: brush default size 16. **Jumble tool**: scatter pixels within the brush (size 16).

**Text tool** **[data]**: font family (system, theme, pinned, loaded external .ttf), size (default 12), bold, italic, antialias, hinting (none/slight/full), ligatures, weight 100-1000 (Thin..Extra Black), "More Options". Also Edit > Insert Text: text, font, size, colour, antialias. Output is rasterised pixels (no live text layer).

**Zoom / Hand**: zoom tool click/drag; hand scrolls. Zoom presets 50%-3200% on keys; Fit on Screen (Ctrl+0); zoom with wheel / from centre / pinch.

---

## 3. Commands

Total command ids appearing in gui.xml (keys + menus) = 169; the localisation file names 194 distinct command ids. The table below covers every id with a keyboard entry or menu entry in gui.xml (171 rows including those that only exist as shortcuts) and is followed by ids that exist only in en.ini.

### 3.1 Menu bar structure (from gui.xml; menu text keys shown as `@.xxx`)

- **@.file**
  - NewFile - @.file_new
  - OpenFile - @.file_open
  - **@.file_open_recent**
    - ReopenClosedFile - @.file_reopen_closed
    - ClearRecentFiles - @.file_clear_recent_files
  - SaveFile - @.file_save
  - SaveFileAs - @.file_save_as
  - CloseFile - @.file_close
  - CloseAllFiles - @.file_close_all
  - **@.file_export**
    - SaveFileCopyAs - @.file_export_as
    - ExportSpriteSheet - @.file_export_sprite_sheet
    - ExportTileset - @.file_export_tileset
    - RepeatLastExport - @.file_repeat_last_export
  - **@.file_import**
    - ImportSpriteSheet - @.file_import_sprite_sheet
  - **@.file_scripts**
    - OpenScriptFolder - @.file_open_script_folder
    - Refresh - @.file_rescan_script_folder
    - Debugger - @.file_debugger
  - Exit - @.file_exit
- **@.edit**
  - Undo - @.edit_undo
  - Redo - @.edit_redo
  - UndoHistory - @.edit_undo_history
  - Cut - @.edit_cut
  - Copy - @.edit_copy
  - CopyMerged - @.edit_copy_merged
  - Paste - @.edit_paste
  - **@.edit_paste_special**
    - NewFile (fromClipboard=true) - @.edit_paste_special_new_sprite
    - NewLayer (fromClipboard=true) - @.edit_paste_special_new_layer
    - NewLayer (reference=true, fromClipboard=true) - @.edit_paste_special_new_ref_layer
  - Clear - @.edit_clear
  - Fill - @.edit_fill
  - Stroke - @.edit_stroke
  - **@.edit_rotate**
    - Rotate (target=mask, angle=180) - @.edit_rotate_180
    - Rotate (target=mask, angle=90) - @.edit_rotate_90cw
    - Rotate (target=mask, angle=-90) - @.edit_rotate_90ccw
  - Flip (target=mask, orientation=horizontal) - @.edit_flip_horizontal
  - Flip (target=mask, orientation=vertical) - @.edit_flip_vertical
  - MaskContent - @.edit_transform
  - **@.edit_shift**
    - MoveMask (target=content, direction=left, units=pixel, quantity=1, wrap=1) - @.edit_shift_left
    - MoveMask (target=content, direction=right, units=pixel, quantity=1, wrap=1) - @.edit_shift_right
    - MoveMask (target=content, direction=up, units=pixel, quantity=1, wrap=1) - @.edit_shift_up
    - MoveMask (target=content, direction=down, units=pixel, quantity=1, wrap=1) - @.edit_shift_down
  - NewBrush - @.edit_new_brush
  - NewSpriteFromSelection - @.edit_new_sprite_from_selection
  - ReplaceColor - @.edit_replace_color
  - InvertColor - @.edit_invert_color
  - **@.edit_adjustments**
    - BrightnessContrast - @.edit_adjustments_brightness_contrast
    - HueSaturation - @.edit_adjustments_hue_saturation
    - ColorCurve - @.edit_adjustments_color_curve
  - **@.edit_fx**
    - Outline - @.edit_fx_outline
    - ConvolutionMatrix - @.edit_fx_convolution_matrix
    - Despeckle - @.edit_fx_despeckle
  - PasteText - @.edit_insert_text
  - KeyboardShortcuts - @.edit_keyboard_shortcuts
  - Options - @.edit_preferences
- **@.sprite**
  - SpriteProperties - @.sprite_properties
  - **@.sprite_color_mode**
    - ChangePixelFormat (format=rgb) - @.sprite_color_mode_rgb
    - ChangePixelFormat (format=grayscale) - @.sprite_color_mode_grayscale
    - ChangePixelFormat (format=indexed) - @.sprite_color_mode_indexed
    - ChangePixelFormat - @.sprite_color_mode_more_options
  - DuplicateSprite - @.sprite_duplicate
  - SpriteSize - @.sprite_sprite_size
  - CanvasSize - @.sprite_canvas_size
  - **@.sprite_rotate_canvas**
    - Rotate (target=canvas, angle=180) - @.sprite_rotate_180
    - Rotate (target=canvas, angle=90) - @.sprite_rotate_90cw
    - Rotate (target=canvas, angle=-90) - @.sprite_rotate_90ccw
    - Flip (target=canvas, orientation=horizontal) - @.sprite_flip_canvas_horizontal
    - Flip (target=canvas, orientation=vertical) - @.sprite_flip_canvas_vertical
  - CropSprite - @.sprite_crop
  - AutocropSprite - @.sprite_trim
- **@.layer**
  - LayerProperties - @.layer_properties
  - LayerVisibility - @.layer_visible
  - LayerLock - @.layer_lock_layers
  - OpenGroup - @.layer_open_group
  - **@.layer_new**
    - NewLayer - @.layer_new_layer
    - NewLayer (group=true) - @.layer_new_group
    - NewLayer (viaCopy=true) - @.layer_new_layer_via_copy
    - NewLayer (viaCut=true) - @.layer_new_layer_via_cut
    - NewLayer (reference=true, fromFile=true) - @.layer_new_reference_layer_from_file
    - NewLayer (tilemap=true, ask=true) - @.layer_new_tilemap_layer
  - RemoveLayer - @.layer_delete_layer
  - **@.layer_convert_to**
    - ConvertLayer (to=background) - @.layer_convert_to_background
    - ConvertLayer (to=layer) - @.layer_convert_to_layer
    - ConvertLayer (to=tilemap) - @.layer_convert_to_tilemap
  - DuplicateLayer - @.layer_duplicate
  - MergeDownLayer - @.layer_merge_down
  - FlattenLayers - @.layer_flatten
  - FlattenLayers (visibleOnly=true) - @.layer_flatten_visible
- **@.frame**
  - FrameProperties (frame=current) - @.frame_properties
  - CelProperties - @.frame_cel_properties
  - NewFrame - @.frame_new_frame
  - NewFrame (content=empty) - @.frame_new_empty_frame
  - NewFrame (content=celcopies) - @.frame_duplicate_cels
  - NewFrame (content=cellinked) - @.frame_duplicate_linked_cels
  - RemoveFrame - @.frame_delete_frame
  - **@.frame_playback**
    - PlayAnimation - 
    - PlayPreviewAnimation - 
    - SetPlaybackSpeed (multiplier=0.25) - 
    - SetPlaybackSpeed (multiplier=0.5) - 
    - SetPlaybackSpeed (multiplier=1.0) - 
    - SetPlaybackSpeed (multiplier=1.5) - 
    - SetPlaybackSpeed (multiplier=2.0) - 
    - SetPlaybackSpeed (multiplier=3.0) - 
    - TogglePlayOnce - 
    - TogglePlayAll - 
    - TogglePlaySubtags - 
    - ToggleRewindOnStop - 
  - **@.frame_tags**
    - FrameTagProperties - @.frame_tags_tag_properties
    - NewFrameTag - @.frame_tags_new_tag
    - RemoveFrameTag - @.frame_tags_delete_tag
  - **@.frame_jump_to**
    - GotoFirstFrame - @.frame_jump_to_first_frame
    - GotoPreviousFrame - @.frame_jump_to_previous_frame
    - GotoNextFrame - @.frame_jump_to_next_frame
    - GotoLastFrame - @.frame_jump_to_last_frame
    - GotoFirstFrameInTag - @.frame_jump_to_first_frame_in_tag
    - GotoLastFrameInTag - @.frame_jump_to_last_frame_in_tag
    - GotoFrame - @.frame_go_to_frame
  - FrameProperties (frame=all) - @.frame_constant_frame_rate
  - ReverseFrames - @.frame_reverse_frames
- **@.select**
  - MaskAll - @.select_all
  - DeselectMask - @.select_deselect
  - ReselectMask - @.select_reselect
  - InvertMask - @.select_inverse
  - MaskByColor - @.select_color_range
  - **@.select_modify**
    - ModifySelection (modifier=border) - @.select_modify_border
    - ModifySelection (modifier=expand) - @.select_modify_expand
    - ModifySelection (modifier=contract) - @.select_modify_contract
  - LoadMask - @.select_load_from_file
  - SaveMask - @.select_save_to_file
- **@.view**
  - DuplicateView - @.view_duplicate_view
  - ToggleWorkspaceLayout - @.view_workspace_layout
  - RunCommand - @.view_run_command
  - ShowExtras - @.view_show_extras
  - **@.view_show**
    - ShowLayerEdges - @.view_show_layer_edges
    - ShowSelectionEdges - @.view_show_selection_edges
    - ShowGrid - @.view_show_grid
    - ShowAutoGuides - @.view_show_auto_guides
    - ShowSlices - @.view_show_slices
    - ShowPixelGrid - @.view_show_pixel_grid
    - ShowTileNumbers - @.view_show_tile_numbers
    - ShowBrushPreview - @.view_show_brush_preview
  - **@.view_grid**
    - GridSettings - @.view_grid_settings
    - SelectionAsGrid - @.view_grid_selection_as_grid
    - SnapToGrid - @.view_grid_snap_to_grid
  - **@.view_tiled_mode**
    - TiledMode (axis=none) - @.view_tiled_mode_none
    - TiledMode (axis=both) - @.view_tiled_mode_both
    - TiledMode (axis=x) - @.view_tiled_mode_x
    - TiledMode (axis=y) - @.view_tiled_mode_y
  - SymmetryMode - @.view_symmetry_options
  - SetLoopSection - @.view_set_loop_section
  - ShowOnionSkin - @.view_show_onion_skin
  - Timeline (switch=true) - @.view_timeline
  - **@.view_preview**
    - TogglePreview - @.view_preview
    - ToggleOtherLayersOpacity (preview=true, checkedIfZero=true) - @.view_preview_hide_other_layers
    - ShowBrushPreviewInPreview - @.view_preview_brush_preview
  - AdvancedMode - @.view_advanced_mode
  - FullscreenMode - @.view_full_screen_mode
  - FullscreenPreview - @.view_full_screen_preview
  - Home - @.view_home
  - Refresh - @.view_refresh
- **@.help**
  - OpenBrowser (filename=README.md) - @.help_readme
  - Launch (type=url, path=/quickref/) - @.help_quick_reference
  - Launch (type=url, path=/docs/) - @.help_documentation
  - Launch (type=url, path=/tutorial/) - @.help_tutorial
  - Launch (type=url, path=/release-notes/) - @.help_release_notes
  - Launch (type=url, path=http://twitter.com/aseprite) - @.help_twitter
  - EnterLicense - @.help_enter_license
  - About - @.help_about

### 3.2 Command table

Columns: id, English label (en.ini), menus where it appears, default shortcuts with parameters. `(none)` means a parameterised keybinding slot that ships unbound (can be assigned in Keyboard Shortcuts). Windows/Linux keys shown; macOS uses Cmd for Ctrl.

| Command id | Label | In menus | Shortcut(s) [params] |
|---|---|---|---|
| About | About | help_menu |  |
| AddColor | Add {0} Color to Palette |  | (none) [source=fg]; (none) [source=bg] |
| AdvancedMode | Advanced Mode | view_menu | Ctrl+F |
| Apply | Apply |  | Enter ctx=Transformation; Enter Pad ctx=Transformation |
| AutocropSprite | Trim Sprite | sprite_menu | (none); (none) [byGrid=true] |
| BrightnessContrast | Adjust Brightness/Contrast | edit_adjustments |  |
| Cancel | Cancel Current Operation | cel_movement_popup_menu | Esc [type=all] |
| CanvasSize | Canvas Size | sprite_menu | C |
| CelProperties | Cel Properties | cel_popup_menu, frame_menu |  |
| ChangeBrush | Change Brush: {0} |  | + [change=increment-size]; = [change=increment-size]; - [change=decrement-size]; Plus Pad [change=increment-size]; Minus Pad [change=decrement-size]; Space+H [change=flip-x]; Space+X [change=flip-x]; Space+V [change=flip-y]; Space+Y [change=flip-y]; Space+D [change=flip-d]; Space+R [change=rotate-90cw]; Alt+1 [change=custom, slot=1]; Alt+2 [change=custom, slot=2]; Alt+3 [change=custom, slot=3]; Alt+4 [change=custom, slot=4]; Alt+5 [change=custom, slot=5]; Alt+6 [change=custom, slot=6]; Alt+7 [change=custom, slot=7]; Alt+8 [change=custom, slot=8]; Alt+9 [change=custom, slot=9]; (none) [change=increment-angle]; (none) [change=decrement-angle] |
| ChangeColor | Change Color: {0} |  | 9 [target=foreground, change=decrement-index]; 0 [target=foreground, change=increment-index]; [ [target=foreground, change=decrement-index]; ] [target=foreground, change=increment-index]; (none) [target=background, change=decrement-index]; (none) [target=background, change=increment-index] |
| ChangePixelFormat | Change Color Mode: {0} | sprite_color_mode | (none) [format=rgb]; (none) [format=grayscale]; (none) [format=indexed, dithering=ordered]; (none) [format=indexed, dithering=old] |
| Clear | Clear | edit_menu | Del; Backspace |
| ClearCel | Clear Cel | cel_popup_menu | (none) |
| ClearRecentFiles | Clear Recent Files | file_open_recent |  |
| CloseAllFiles | Close All Files | file_menu | Ctrl+Shift+W |
| CloseFile | Close File | document_tab_popup_menu, file_menu, tab_popup_menu | Ctrl+W |
| ColorCurve | Color Curve | edit_adjustments | Ctrl+M; F10 |
| ColorQuantization | Create Palette from Current Sprite (Color Quantization) | palette_popup_menu | (none) |
| ContiguousFill | Switch Contiguous Fill |  | (none) |
| ConvertLayer | Convert Layer | @main_menu.layer_convert_to, layer_convert_to |  |
| ConvolutionMatrix | Convolution Matrix | fx_popup_menu | F9 |
| Copy | Copy | edit_menu | Ctrl+C; Ctrl+Ins |
| CopyCel | Copy Cel | cel_movement_popup_menu |  |
| CopyMerged | Copy Merged | edit_menu | Ctrl+Shift+C |
| CopyPath | Copy Path | document_tab_popup_menu |  |
| CropSprite | Crop Sprite | sprite_menu |  |
| Cut | Cut | edit_menu | Ctrl+X; Shift+Del |
| Debugger | Debugger | scripts_menu |  |
| DeselectMask | Deselect Mask | select_menu | Ctrl+D; Esc ctx=Transformation |
| Despeckle | Despeckle | fx_popup_menu |  |
| DuplicateLayer | Duplicate Layer | layer_menu, layer_popup_menu |  |
| DuplicateSlice | Duplicate Slice | slice_popup_menu |  |
| DuplicateSprite | Duplicate Sprite | sprite_menu |  |
| DuplicateView | Duplicate View | document_tab_popup_menu, view_menu |  |
| EnterLicense | Enter License | help_menu |  |
| Exit | Exit | file_menu | (none) mac=Cmd+Q; (none) |
| ExportSpriteSheet | Export Sprite Sheet | file_export | Ctrl+E |
| ExportTileset | Export Tileset | file_export |  |
| Fill | Fill Selection with Foreground Color | edit_menu | F |
| FitScreen | Fit on Screen |  | Ctrl+0 |
| FlattenLayers | Flatten Layers | layer_menu, layer_popup_menu |  |
| Flip | Flip {0} {1} | edit_menu, sprite_rotate_canvas | Shift+H [target=mask, orientation=horizontal]; Shift+V [target=mask, orientation=vertical] |
| FrameProperties | Frame Properties | frame_menu, frame_popup_menu | P [frame=current] |
| FrameTagProperties | Tag Properties | frame_tags, tag_popup_menu |  |
| FullscreenMode | Toggle Fullscreen Mode | view_menu | F11 mac=Cmd+Ctrl+F |
| FullscreenPreview | Fullscreen Preview | view_menu | F8 |
| GotoFirstFrame | Go to First Frame | frame_jump_to | Home |
| GotoFirstFrameInTag | Go to First Frame In Tag | frame_jump_to |  |
| GotoFrame | Go to Frame | frame_jump_to | Alt+G |
| GotoLastFrame | Go to Last Frame | frame_jump_to | End |
| GotoLastFrameInTag | Go to Last Frame In Tag | frame_jump_to |  |
| GotoNextFrame | Go to Next Frame | frame_jump_to | Right ctx=Normal |
| GotoNextFrameWithSameTag | Go to Next Frame with same tag |  | . |
| GotoNextLayer | Go to Next Layer |  | Up ctx=Normal |
| GotoNextTab | Go to Next Tab |  | Ctrl+Tab; Ctrl+PgDn |
| GotoPreviousFrame | Go to Previous Frame | frame_jump_to | Left ctx=Normal |
| GotoPreviousFrameWithSameTag | Go to Previous Frame with same tag |  | , |
| GotoPreviousLayer | Go to Previous Layer |  | Down ctx=Normal |
| GotoPreviousTab | Go to Previous tab |  | Ctrl+Shift+Tab; Ctrl+PgUp |
| GridSettings | Grid Settings | view_grid |  |
| Home | Home | view_menu |  |
| HueSaturation | Adjust Hue/Saturation | edit_adjustments | Ctrl+U |
| ImportSpriteSheet | Import Sprite Sheet | file_import | Ctrl+I |
| InvertColor | Invert Color | edit_menu |  |
| InvertMask | Invert Selection | select_menu | Ctrl+Shift+I |
| KeyboardShortcuts | Keyboard Shortcuts | edit_menu | Ctrl+Alt+Shift+K |
| Launch | Launch | help_menu |  |
| LayerLock | Lock Layers | layer_menu |  |
| LayerOpacity | Set Layer Opacity to {0} ({1}%) |  | (none) [opacity=0]; (none) [opacity=32]; Shift+1 [opacity=64]; (none) [opacity=96]; Shift+2 [opacity=128]; (none) [opacity=160]; Shift+3 [opacity=192]; (none) [opacity=224]; Shift+4 [opacity=255] |
| LayerProperties | Layer Properties | layer_menu, layer_popup_menu | Shift+P; F2 ctx=Normal |
| LayerVisibility | Layer Visibility | layer_menu | Shift+X |
| LinkCels | Link Cels | cel_popup_menu | (none) |
| LoadMask | Load Selection | select_menu |  |
| LoadPalette | Load Palette | palette_popup_menu | (none) |
| MaskAll | Mask All | select_menu | Ctrl+A |
| MaskByColor | Mask By Color | select_menu |  |
| MaskContent | Mask Content | edit_menu | Ctrl+T |
| MergeDownLayer | Merge Down Layer | layer_menu, layer_popup_menu |  |
| ModifySelection | {0} Selection {1} | select_modify |  |
| MoveCel | Move Cel | cel_movement_popup_menu |  |
| MoveMask | Move {0} {1} | edit_shift | Left [target=content, direction=left, units=pixel, quantity=1] ctx=Selection; Right [target=content, direction=right, units=pixel, quantity=1] ctx=Selection; Up [target=content, direction=up, units=pixel, quantity=1] ctx=Selection; Down [target=content, direction=down, units=pixel, quantity=1] ctx=Selection; Shift+Left [target=content, direction=left, units=tile-width, quantity=1] ctx=Selection; Shift+Right [target=content, direction=right, units=tile-width, quantity=1] ctx=Selection; Shift+Up [target=content, direction=up, units=tile-height, quantity=1] ctx=Selection; Shift+Down [target=content, direction=down, units=tile-height, quantity=1] ctx=Selection; Alt+Left [target=boundaries, direction=left, units=pixel, quantity=1] ctx=Selection; Alt+Right [target=boundaries, direction=right, units=pixel, quantity=1] ctx=Selection; Alt+Up [target=boundaries, direction=up, units=pixel, quantity=1] ctx=Selection; Alt+Down [target=boundaries, direction=down, units=pixel, quantity=1] ctx=Selection; Alt+Shift+Left [target=boundaries, direction=left, units=tile-width, quantity=1] ctx=Selection; Alt+Shift+Right [target=boundaries, direction=right, units=tile-width, quantity=1] ctx=Selection; Alt+Shift+Up [target=boundaries, direction=up, units=tile-height, quantity=1] ctx=Selection; Alt+Shift+Down [target=boundaries, direction=down, units=tile-height, quantity=1] ctx=Selection |
| NewBrush | New Brush | edit_menu | Ctrl+B |
| NewFile | New File | edit_paste_special, file_menu | Ctrl+N |
| NewFrame | New Frame | cel_popup_menu, frame_menu, frame_popup_menu, new_frame_popup_menu | Alt+N; Alt+B [content=empty]; Alt+M [content=cellinked]; Alt+D [content=celcopies]; Alt+Shift+D [content=cellinked]; (none) [content=cel] |
| NewFrameTag | New Tag | frame_popup_menu, frame_tags |  |
| NewLayer | New {} | @main_menu.layer_new, edit_paste_special, layer_new | Shift+N; Shift+Space+N [before=true]; Alt+Shift+N [group=true]; Ctrl+Shift+V [fromClipboard=true]; Ctrl+J [viaCopy=true]; Ctrl+Shift+J [viaCut=true]; Space+N [ask=true, tilemap=true]; (none) [tilemap=true] |
| NewSpriteFromSelection | New Sprite from Selection | edit_menu | Ctrl+Alt+N |
| OpenBrowser | Open Browser | help_menu |  |
| OpenFile | Open Sprite | file_menu | Ctrl+O |
| OpenGroup | Open/Close Group | layer_menu | Shift+E |
| OpenInFolder | Open In Folder | document_tab_popup_menu |  |
| OpenScriptFolder | Open Script Folder | scripts_menu |  |
| OpenWithApp | Open With Associated Application | document_tab_popup_menu |  |
| Options | Preferences | edit_menu | (none) mac=Cmd+,; Ctrl+K |
| Outline | Outline | fx_popup_menu | Shift+O |
| PaletteEditor | Switch {0} {1} {2} | palette_popup_menu | A; F4 [edit=switch, popup=foreground]; (none) [popup=foreground]; (none) [popup=background] |
| PaletteSize | Palette Size | palette_popup_menu |  |
| Paste | Paste | edit_menu | Ctrl+V; Shift+Ins |
| PasteText | Insert Text | edit_menu |  |
| PixelPerfectMode | Switch Pixel Perfect Mode |  | (none) |
| PlayAnimation | Play Animation | animation_menu | Enter; Enter Pad |
| PlayPreviewAnimation | Play Preview Animation | animation_menu | Shift+Enter |
| Redo | Redo | edit_menu | Ctrl+Y; Ctrl+R; Ctrl+Shift+Z |
| Refresh | Refresh | scripts_menu, view_menu | F5 |
| RemoveFrame | Remove Frame | frame_menu, frame_popup_menu | Alt+C |
| RemoveFrameTag | Remove Tag | frame_tags, tag_popup_menu |  |
| RemoveLayer | Remove Layer | layer_menu, layer_popup_menu |  |
| RemoveSlice | Remove Slice | slice_popup_menu |  |
| ReopenClosedFile | Reopen Closed File | file_open_recent | Ctrl+Shift+T |
| RepeatLastExport | Repeat Last Export | file_export | Ctrl+Shift+X |
| ReplaceColor | Replace Color | edit_menu | Shift+R |
| ReselectMask | Reselect Mask | select_menu | Ctrl+Shift+D |
| ReverseFrames | Reverse Frames | frame_menu, frame_popup_menu | Alt+I |
| Rotate | Rotate {0} {1} | edit_rotate, sprite_rotate_canvas |  |
| RunCommand | Run Command | view_menu | Ctrl+Space |
| SaveFile | Save File | file_menu | Ctrl+S |
| SaveFileAs | Save File As | file_menu | Ctrl+Shift+S |
| SaveFileCopyAs | Export | file_export | Ctrl+Alt+Shift+S |
| SaveMask | Save Selection | select_menu |  |
| SavePalette | Save Palette | palette_popup_menu | (none) |
| Screenshot | Screenshot |  | (none); (none) [srgb=false]; (none) [save=true]; (none) [save=true, srgb=false]; F12 [steam=true] |
| Scroll | Scroll {0} |  | Space+Left [direction=left, units=zoomed-pixel, quantity=1]; Space+Right [direction=right, units=zoomed-pixel, quantity=1]; Space+Up [direction=up, units=zoomed-pixel, quantity=1]; Space+Down [direction=down, units=zoomed-pixel, quantity=1]; Shift+Space+Left [direction=left, units=zoomed-tile-width, quantity=1]; Shift+Space+Right [direction=right, units=zoomed-tile-width, quantity=1]; Shift+Space+Up [direction=up, units=zoomed-tile-height, quantity=1]; Shift+Space+Down [direction=down, units=zoomed-tile-height, quantity=1] |
| ScrollCenter | Scroll to center of canvas |  | Shift+Z |
| SelectPaletteColors | Select Used Colors | select_palette_colors |  |
| SelectTile | Select Tile |  | (none); (none) [mode=add]; (none) [mode=subtract]; (none) [mode=intersect] |
| SelectionAsGrid | Selection as Grid | view_grid |  |
| SetColorSelector | Set Color Selector | palette_popup_menu | (none) [type=tint-shade-tone]; (none) [type=spectrum]; (none) [type=rgb-wheel]; (none) [type=ryb-wheel]; (none) [type=normal-map-wheel] |
| SetInkType | Set Ink Type: {0} | ink_popup_menu | (none) [type=simple]; (none) [type=alpha-compositing]; (none) [type=copy-color]; (none) [type=lock-alpha]; (none) [type=shading] |
| SetLoopSection | Set Loop Section | frame_popup_menu, view_menu | F2 ctx=FramesSelection |
| SetPaletteEntrySize | Set Palette Entry Size | palette_popup_menu |  |
| SetPlaybackSpeed | Playback Speed {0}x | animation_menu |  |
| SetSameInk | Same Ink in All Tools | ink_popup_menu | (none) |
| ShowAutoGuides | Show Auto Guides | view_show |  |
| ShowBrushPreview | Show Brush Preview | view_show |  |
| ShowBrushPreviewInPreview | Show Brush Preview in Preview | view_preview |  |
| ShowExtras | Show Extras | view_menu | Ctrl+H |
| ShowGrid | Show Grid | view_show | Ctrl+' |
| ShowLayerEdges | Show Layer Edges | view_show |  |
| ShowMenu | Show Menu: {0} |  | Alt+F [menu=file_menu]; Alt+E [menu=edit_menu]; Alt+S [menu=sprite_menu]; Alt+L [menu=layer_menu]; Alt+R [menu=frame_menu]; Alt+T [menu=select_menu]; Alt+V [menu=view_menu]; Alt+H [menu=help_menu] |
| ShowOnionSkin | Show Onion Skin | view_menu | F3 |
| ShowPixelGrid | Show Pixel Grid | view_show | Ctrl+Shift+' |
| ShowSelectionEdges | Show Selection Edges | view_show |  |
| ShowSlices | Show Slices | view_show |  |
| ShowTileNumbers | Show Tile Numbers | view_show |  |
| SliceProperties | Slice Properties | slice_popup_menu |  |
| SnapToGrid | Snap to Grid | view_grid | Shift+S |
| SoloLayer | Solo Layer |  | (none) |
| SpriteProperties | Sprite Properties | sprite_menu | Ctrl+P |
| SpriteSize | Sprite Size | sprite_menu | Ctrl+Alt+I |
| Stroke | Stroke Selection Borders with Foreground Color | edit_menu | S |
| SwitchColors | Switch Colors |  | X |
| SymmetryMode | Symmetry Mode | view_menu | (none) [orientation=vertical]; (none) [orientation=horizontal]; (none) [orientation=right_diagonal]; (none) [orientation=left_diagonal] |
| TiledMode | Tiled Mode: {} | view_tiled_mode |  |
| TilesetMode | Tileset Mode: {} |  | Space+1 [mode=manual]; Space+2 [mode=auto]; Space+3 [mode=stack] |
| Timeline | Switch Timeline | view_menu | Tab [switch=true] |
| ToggleOtherLayersOpacity | Toggle Other Layers Opacity | view_preview | (none); Shift+F7 [preview=true, checkedIfZero=true] |
| TogglePlayAll | Play All Frames (Ignore Tags) | animation_menu |  |
| TogglePlayOnce | Play Once | animation_menu |  |
| TogglePlaySubtags | Play Subtags & Repetitions | animation_menu |  |
| TogglePreview | Toggle Preview | view_preview | F7 |
| ToggleRewindOnStop | Rewind on Stop | animation_menu |  |
| ToggleTilesMode | Toggle Tiles Mode |  | Space+Tab |
| ToggleTimelineThumbnails | Toggle Timeline Thumbnails |  | F6 |
| ToggleWorkspaceLayout | Toggle Workspace Layout | view_menu | Shift+W |
| Undo | Undo | edit_menu | Ctrl+Z; Ctrl+Z ctx=Transformation |
| UndoHistory | Undo History | edit_menu |  |
| UnlinkCel | Unlink Cel | cel_popup_menu | (none) |
| Zoom | Zoom |  | ~ [percentage=50]; ` [percentage=50]; 1 [percentage=100]; 2 [percentage=200]; (none) [percentage=300]; 3 [percentage=400]; (none) [percentage=500]; (none) [percentage=600]; 4 [percentage=800]; 5 [percentage=1600]; (none) [percentage=2400]; 6 [percentage=3200]; Ctrl++ [action=in]; Ctrl+- [action=out]; Ctrl+= [action=in]; Ctrl+Shift+= [action=in] |


### 3.3 Command ids named in en.ini but absent from gui.xml (scriptable / Run Command / internal)
BackgroundFromLayer, CelOpacity, CopyColors, CopyTiles, DeveloperConsole, DiscardBrush, Eyedropper, LayerFromBackground, LoadDefaultPalette, MoveColors, MoveTiles, RunScript, SavePaletteAsDefault, SavePaletteAsPreset, SetPalette, ShowBrushes, ShowDynamics, ShowPaletteOptions, ShowPalettePresets, ShowPaletteSortOptions, SwapCheckerboardColors, SwitchNonactiveLayersOpacity, TilesetDelete, TilesetDuplicate, WidgetZoo. (Move_* strings are sub-labels of MoveMask, not commands.) Aseprite exposes all of these via the Run Command palette (Ctrl+Space), which also accepts `=` math expressions and `@` Lua snippets. **[data: run_command]**

### 3.4 Non-command keyboard actions (gui.xml `<actions>`, `<quicktools>`, `<drag>`)
Copy Selection (Ctrl while dragging), Snap to Grid (Alt), Lock Axis (Shift), Fine Control (Ctrl, while translating/scaling), Angle Snap (Shift), Maintain Aspect Ratio (Shift), Scale From Center (Alt), Add/Subtract/Intersect Selection (Shift / Alt+Shift / Ctrl+Shift), Auto Select Layer (Ctrl), Straight Line From Last Point (Shift), Angle Snap From Last Point (Ctrl), Square Aspect (Shift), Draw From Center (Ctrl), Corner Radius (C), Move Origin (Space), Rotate Shape (Alt), Left/Right Mouse Button triggers (unbound), Brush Size drag (Ctrl+Alt).
Key contexts: Normal, Selection, Translating Selection, Scaling Selection, Rotating Selection, Move Tool, Freehand Tool, Shape Tool, Frames Selection, Transformation.
Shortcut capability: keys can be Ctrl/Cmd/Alt/Shift/Space/Win + key; per-platform overrides (`win`, `linux`, `mac`); import/export of the whole shortcut set; mouse-wheel behaviour table (default/custom); drag-value table.

---

## 4. Animation features

- **Timeline** **[data]**: layer rows x frame columns; cel grid; layer panel width (default 100), position Bottom/Left/Right; frame header shows number (first-frame base configurable); tag bar above frames with coloured tag spans; thumbnails on/off with size/overlay size/scale-to-fit; auto-show timeline when frame/layer added; Tab toggles timeline; select on click / Shift+click / drag; keep selection; drag-and-drop from edges to move/copy ranges.
- **Range selection** of frames, layers, cels (multi-cel ops: new frame from range, delete, reverse, properties for range, set loop section).
- **Frame ops**: New Frame (blank copy based), New Empty Frame, Duplicate Cels (copies), Duplicate Linked Cels, "New Frame with cel" variant, Remove Frame, Reverse Frames, Frame Properties (current range / all = constant frame rate), Go to Frame (number or tag name), first/last/prev/next frame, first/last in tag, prev/next with same tag.
- **Playback** **[data]**: Play (Enter), Play Preview (Shift+Enter), speed multipliers 0.25x, 0.5x, 1x, 1.5x, 2x, 3x, Play Once, Play All Frames (ignore tags), Play Subtags & Repetitions, Rewind on Stop, loop section (Set Loop Section, F2 in frame selection). Separate play settings for editor vs preview window.
- **Tag directions**: Forward, Reverse, Ping-pong, Ping-pong Reverse; repeat count.
- **Onion skin** **[data: pref document.onionskin, timeline_conf]**: active toggle (F3); previous frames N (default 1) and next frames N; base opacity 68, opacity step 28; type Merge Frames or Red/Blue Tint; loop through tag frames; current layer only; position Behind sprite (transparent layers only) or In front; Reset and "Set as defaults".
- **Frame duration** in ms, editable per range; export-time constraints for GIF (10 ms granularity).
- **Cel animation helpers**: linked cels, cel opacity/z-index, "Toggle other layers opacity" for focused work, non-active layer opacity 0-255 for editor and preview (experimental).
- **Preview window** (F7): separate live view with own zoom/scroll/auto-scroll, play controls, brush preview option, hides other layers option; Fullscreen Preview (F8).
- Animated sprite import: open a numbered file sequence as one animation (prompt Ask/Yes/No, with duration).

---

## 5. Color and palette features

- **Foreground/background colour** with 3 forms: RGB value, palette index (`index{n}`), tile; colour popup also supports Transparent selection; hex entry, sliders for RGB, HSV, HSL, grayscale, alpha. Alpha and opacity ranges selectable 0-255 or percent. **[data]**
- **Colour selectors** (5) **[data]**: Tint/Shade/Tone, Spectrum, RGB Color Wheel, RYB Color Wheel, Normal Map Color Wheel. Options: Discrete wheel, harmonies (None, Complementary, Monochromatic, Analogous, Split-Complementary, Triadic, Tetradic, Square), wheel model (RGB vs RYB), shader-based rendering, "hue with sat/value" tweak.
- **Palette editor** (Edit Palette mode, A / F4): edit selected entries with the colour popup, range selection, entry sizes Small(7) / Medium(11) / Large(17) px, separator between entries toggle, drag to reorder/copy, Palette Size dialog, Add fg/bg colour (and a warning button when the fg/bg is not in the palette), Remap Palette (match old to new indexes) with automatic remap for >256 colours warning.
- **Sort & Gradients** **[data]**: Reverse Colors, Sort Colors by Hue, Saturation, Brightness, Luminance, Red, Green, Blue, Alpha (ascending/descending), Gradient (between range ends), Gradient by Hue.
- **Ramps/shading**: shading ink uses a shade built from palette ranges.
- **Presets**: palette presets list (bundled + user), Load/Save Palette, Load Default, Save as Default, Save as Preset, Open Presets Folder. Palette file types: .gpl, .ase, .act-like, image files. **[inferred]**
- **Create palette from sprite** (Color Quantization) **[data]**: new palette with colour-count limit, replace current palette, replace current range, with alpha component; advanced: RGB-to-palette-index mapping algorithm (Default Octree, Table RGB 5bit+Alpha 3bit, Octree), colour best-fit criteria (Default Euclidean, RGB, Linearized RGB, CIEXYZ, CIELAB).
- **Select palette colours** by used/unused colours and used/unused tiles.
- **Colour mode conversion** (Sprite > Color Mode, ChangePixelFormat) **[data]**:
  - RGB, Grayscale, Indexed targets. To-gray algorithm: Luminance (default), HSV, HSL. Option "Merge layers" (flatten) when converting.
  - Indexed conversion dithering: None, Ordered (matrix: Bayer 2x2/4x4/8x8 bundled, custom bitmap via extension), Old Dithering, Floyd-Steinberg error diffusion; dithering factor 0-100 slider; with-alpha toggle; same quantisation advanced options as above (RGB map algorithm and fit criteria).
  - CLI: `--color-mode`, `--dithering-algorithm none|ordered|old`, `--dithering-matrix`.
- **Colour management** **[data]**: manage colours on/off, working RGB space (default sRGB), behaviour for files with profile (Disable, Use embedded, Convert, Assign, Ask) and for missing profile (default Assign), window colour profile (monitor / sRGB / specific), Assign/Convert in Sprite Properties, screenshot in sRGB or display profile.
- **Bundled palettes** (extension packs): 64 .gpl palettes across 12 packs: adigunpolack (5), arne (9), dawnbringer (2), endesga (8), hardware (many: NES, Game Boy, C64, CGA, VGA 13h, ZX Spectrum, MSX1, Master System, Atari 2600, Apple II, VIC-20, Teletext, Virtual Boy, CPC), hyohnoo, javierguerrero, pico8, pinetreepizza, software (Win16, X11, Web-safe, Solarized, Monokai, Google UI, Minecraft...), zughy, davitmasia. Also `palettes/tags.gpl` (default tag-colour swatches: Transparent, Red, Orange, Yellow, Green, Blue, Purple, Gray).

---

## 6. Filters and effects

All filter dialogs (except where noted) share: Preview toggle, Tiled toggle (apply across tile boundaries), Apply-to cels target Selected vs All (`filters.cels_target`), OK/Apply/Cancel, work on the current selection or whole cel, operate on the selected range of cels/layers/frames. **[data: filters strings]**

| Filter | Command id | Key | Parameters |
|---|---|---|---|
| Replace Color | ReplaceColor | Shift+R | From colour, To colour, tolerance |
| Invert Color | InvertColor | none | none |
| Brightness/Contrast | BrightnessContrast | none | brightness, contrast sliders |
| Hue/Saturation | HueSaturation | Ctrl+U | hue, saturation, lightness, alpha; mode selectable (default HSL multiplicative; other modes exist) **[behaviour of other modes not in data files]** |
| Color Curve | ColorCurve | Ctrl+M or F10 | editable spline curve with draggable control points; Point Properties dialog (X, Y, delete); per-channel mode **[inferred]** |
| Outline | Outline | Shift+O | outline colour, background colour, place Outside/Inside, shape Circle/Square/Horizontal/Vertical, plus a 3x3 custom neighbour matrix toggle grid |
| Convolution Matrix | ConvolutionMatrix | F9 | choose a stock matrix (from `convmatr.def`) or custom; Reload Stock button |
| Despeckle | Despeckle | none | median filter, width and height |
| Fill / Stroke | Fill, Stroke | F / S | uses foreground colour on selection (fill) or selection borders (stroke) |
| Pixel transforms | Rotate, Flip, MoveMask(wrap) | Shift+H/V | rotate 90/180/-90, flip H/V on selection or canvas, shift selection content by 1px with wrap (Edit > Shift) |

Stock convolution matrices in `convmatr.def` (24; each: name, W, H, centre x/y, kernel, divisor, bias, applied channels) **[data]**: brightness, contrast, negative, blur-3x3, blur-3x3-hard, blur-5x5, blur-7x7, blur-9x9, blur-17x17, blur-5x3-left, blur-17x3-left, blur-3x17-top, sharpen-3x3, sharpen-5x5, sharpen-7x7, edges-find, edges-find-horizontal, edges-find-vertical, misc-contour, misc-texturize, misc-emboss, misc-marmolize, misc-rock, misc-rock-edges. File grammar: `name W H cx cy { kernel numbers } divisor|auto bias|auto channels(rgb|rgba)`.

Sprite-level image operations **[data]**: Sprite Size (pixels or percent, lock ratio, interpolation Nearest-neighbour / Bilinear / RotSprite), Canvas Size (width/height or left/top/right/bottom border deltas, 9-anchor direction picker, trim content outside), Rotate canvas 90/180, Flip canvas, Crop Sprite (to selection), Trim Sprite (autocrop, optionally by grid), Duplicate Sprite (optionally merged layers only).
Effect inks/tools: Blur tool, Jumble tool (section 2). Experimental "new blend" and "compose groups separately" rendering switches.

---

## 7. Import / export

### 7.1 File formats referenced in data files
- Native: .aseprite / .ase. **[data]**
- Read/write images: PNG, GIF (animated), JPEG (quality), BMP, PCX, TGA (bits-per-pixel, RLE compress), WebP (animated; lossless simple/lossless/lossy), SVG (export, pixel scale), CSS (box-shadow export, pixel scale, CSS3 variables, sample HTML), QOI (library credit), FLC/FLI (README), ICO (icon assets exist; **[general]**), APNG (**[general]**, not in data), Aseprite palette .ase, GIMP .gpl, .msk selections. Image-sequence open/save with `{name}` formatting. **[data/general mix]**
- Format-capability warnings listed in alerts: alpha channel, tags, frames, grayscale mode, indexed mode, layers, palette changes between frames, <20 ms duration, non-10ms durations, RGB mode. Saving to a lossy format warns, offering .aseprite to keep everything.

### 7.2 File > Export (single image or animation) **[data: export_file]**
Output file; Resize % (editable); Area (whole sprite / selection / slice); Layers (visible layers, selected layers, layer by name); Frames (all, selected, tag); Animation direction; Play subtags & repetitions; Apply pixel ratio; Export for Twitter (adjust last frame duration to 1/4); Adjust resize to N%; Ignore empty frames. Animations to non-animated formats offer export as numbered sequence. Cannot overwrite the original file via Export.
Format option dialogs: GIF (Interlaced, Animation loop, Preserve palette order), JPEG (quality), WebP (type simple/lossless/lossy, compression, quality, image hint, preset, loop), TGA, SVG, CSS.

### 7.3 Export Sprite Sheet (sections: Sprite, Layout, Borders, Output) **[data]**
- Sheet type: Horizontal Strip, Vertical Strip, By Rows, By Columns, Packed.
- Constraints: None, Fixed # Columns, Fixed # Rows, Fixed Width, Fixed Height, Fixed Size.
- Border Padding, Spacing (shape padding), Inner Padding.
- Trim Sprite, Trim Cels (per frame), Trim by Grid, Extrude, Merge Duplicates, Ignore Empty, Size = power of two.
- Source: Layers (visible/selected/specific), Frames (all/selected/tag), Split Layers, Split Tags (and CLI Split Slices / Split Grid).
- Output: sheet image file, JSON data file with format Hash or Array, Meta: Layers / Tags / Slices listing, Item Filename format (marks {layer}, {frame}, {tag}, {tagframe}, {title}, {path}, {filename} ...), Item Tagname format, Preview checkbox, "Open Sprite Sheet" after export, overwrite alerts.
- Repeat Last Export (Ctrl+Shift+X).

### 7.4 Import Sprite Sheet **[data]**
Type (Horizontal Strip, Vertical Strip, By Rows, By Columns); tile bounds picked on canvas or typed (x, y, w, h, columns, rows); padding horizontal/vertical; include partial tiles at bottom/right edges; result placed as new layer "Sprite Sheet" with one frame per tile.

### 7.5 Other I/O
Open Sequence of files as an animation; Export Tileset; Reopen Closed File; screenshot commands (including Steam screenshot); Save Selection/Load Selection (.msk); Save/Load Palette; Copy Merged, Paste Special (new sprite / new layer / reference layer), clipboard image; Recover Files (crashed sessions and previous sessions, as raw images as frames or layers); data recovery auto-save period (default 2 min), keep edited sprite data N days (default 7), keep closed sprite in memory N minutes (default 15).

### 7.6 CLI (`aseprite -b --help`)
Modes: `--batch`/`-b` (no UI), `--shell` (interactive Lua console), `--preview`/`-p` (dry run), `--verbose`, `--debug`, `--version`, `--noinapp`.
Operations: `--save-as`, `--palette`, `--scale`, `--color-mode` (rgb|grayscale|indexed), `--dithering-algorithm` (none|ordered|old), `--dithering-matrix` (bayer2x2|bayer4x4|bayer8x8|file.png), `--shrink-to w,h`, `--crop x,y,w,h`, `--slice name`, `--oneframe`.
Sprite sheet: `--sheet`, `--sheet-type` (horizontal|vertical|rows|columns|packed), `--sheet-pack`, `--sheet-width`, `--sheet-height`, `--sheet-columns`, `--sheet-rows`, `--power-of-two-size`, `--data`, `--format` (json-hash|json-array), `--filename-format`, `--tagname-format`, `--border-padding`, `--shape-padding`, `--inner-padding`, `--trim`, `--trim-sprite`, `--trim-by-grid`, `--extrude`, `--merge-duplicates`, `--ignore-empty`.
Selection: `--layer`/`--import-layer`, `--all-layers`, `--ignore-layer`, `--tag`/`--frame-tag`, `--play-subtags`, `--frame-range from,to`, `--split-layers`, `--split-tags`, `--split-slices`, `--split-grid`, `--export-tileset`.
Listing: `--list-layers`, `--list-layer-hierarchy`, `--list-tags`, `--list-slices`.
Scripting: `--script file.lua`, `--script-param name=value` (read via `app.params`).

---

## 8. Preferences (summary; key names in the appendix)

344 option keys in pref.xml: 223 global (51 sections), 29 per-tool, 92 per-document, plus 22 enum types. Preferences window sections **[data: en.ini options]**: General, Files, Alerts, Color, Editor, Selection, Timeline, Cursors, Background, Grid, Guides & Slices, Undo, Theme, Extensions, Aseprite Format, Windows-specific, File Explorer, Tablet, Experimental; plus Reset Preferences.

Grouped highlights:
- **General/UI**: language (30 language files ship, including English), screen scale, UI scale, theme + font + mini font, show home tab, menu-bar visibility/mouseover expand, single-window vs multi-window UI, recent items (16), show full path, data recovery, native cursors/scale/colour, GPU acceleration (devmode), workspace layout selection.
- **Editor**: wheel zoom, slide zoom, zoom from centre, invert brush-size wheel, scrollbars, auto-scroll, auto-fit on open, straight-line preview, right-click mode (Paint BG colour, Pick FG, Erase, Scroll, Rect Marquee, Lasso, Select layer and move), downsampling mode (Nearest, Bilinear, Bilinear mipmap, Trilinear mipmap), tooltip delay, auto-select layer.
- **Selection**: edge move, handles disabling by modifier, move-on-add, pivot visibility/position, opaque / auto-opaque / transparent colour, rotation algorithm, modify-selection defaults, double-click tile select, force RotSprite, multi-cel transforms, snap to grid.
- **Timeline**: select on click/shift/drag, keep selection, drag from edges, position, panel width, auto-show, rewind on stop.
- **Cursors**: crosshair type (3), brush preview (5), tile preview, cursor colour (negative B/W or specific), snap to grid.
- **Background/Grid/Guides/Slices**: checkerboard type/size/colours/zoom-apply, grid and pixel-grid colour/opacity/auto-opacity, layer-edge and auto-guide colours, default slice colour. Each is configurable for new documents and for the active document.
- **Undo**: size limit MB, go to modified frame/layer, non-linear history, show tooltip; Undo History window.
- **Files/Alerts**: default extensions per command, per-format "show options" alerts, open-sequence behaviour, overwrite and unsupported-feature warnings, run-script alert, invalid fg/bg alert.
- **Color**: colour management set (section 5), alpha/opacity ranges, quantisation defaults.
- **Experimental**: new render engine, new blend, compose groups, native clipboard/dialog, shaders for colour selectors, flash layer, selection tool loop, non-active layer opacity, multiple windows.
- **Tablet**: API choice (Windows Pointer/Wintab), X11 stylus id, pen cursor fix, one-finger-as-mouse.

---

## 9. Scripting and extension surface

- **Lua scripting**: Lua is bundled (license list). Scripts run from File > Scripts folder, Run Script, `--script` in CLI, the interactive `--shell`, or Run Command `@` snippets. Scripts can request permissions via a Security dialog: read file, write file, execute command, open WebSocket, access clipboard, load external library; options "don't ask again for this access" or "full trust for this script"; Debugger window (continue, step into/over/out, breakpoints, stacktrace, locals, console). Script folder rescan; DeveloperConsole command. **[data]**
- **Extension package** (`package.json` + files, installed as a zipped `.aseprite-extension` **[general]**) **[data: bundled examples + a user extension]**:
  ```
  { "name", "displayName", "description", "version", "author": {name,email,url},
    "publisher", "license", "contributors": [...], "categories": ["Themes"|"Palettes"|"Scripts"|...],
    "contributes": { "themes": [{id,path,variant}], "palettes": [{id,path}],
                     "ditheringMatrices": [{id,name,path}], "scripts": [{path}],
                     "languages": [...], "keys": [...] } }
  ```
  Categories shown in the Extensions pane: Languages, Themes, Keyboard Shortcuts, Scripts, Palettes, Dithering Matrices, Multiple. Enable/disable/uninstall, open folder, update/downgrade prompt, install prompt. A theme cannot replace the id `default`.
- **Theme** (theme.xml + sheet.png + sheet.aseprite-data): `<theme name screenscaling uiscaling>` with `<authors>`, `<fonts>` (default, mini), `<dimensions>` (~20 named sizes: scrollbar_size, tabs_height, timeline_base_size, color_slider_height, context_bar_height ...), `<colors>`, `<cursors>`, `<parts>` (345 slices in the sheet), `<styles>` (175) built from parts/backgrounds/icons/text/border; light and dark variants. The sheet is itself a .aseprite sprite sliced with slices (9-slice borders). **[data: counted via tag frequencies]**
- **Keys file**: `<keyboard version="1">` with `<commands><key command shortcut mac win linux context><param name value/></key>`, `<tools><key tool shortcut/>`, `<quicktools>`, `<actions>`, `<drag>`, and wheel sections; exported/imported as `.aseprite-keys` **[general]**. **[data]**
- **Languages**: strings/*.ini, 30 files, section/key format `[section] key = text` with `{0}` placeholders and `&` mnemonics; `@.key` references in XML. **[data]**
- **Fonts**: fonts.xml declares preset fonts with per-platform file names and fallback chains.
- **Widgets**: dialogs described in declarative XML (61 files) with ids, buttonsets, exprs (numeric fields with math), combos, sliders, colour pickers. Implication for us: UI described as data is an architecture hint, not a requirement.
- **User files**: `user.aseprite-brushes`, `user.aseprite-layouts`, palettes folder, extensions folder, sessions folder (recovery). **[data]**

---

## 10. UI and workflow notes

- **Main window**: menu bar, tab bar of open documents (drag to reorder or into a second editor; tab menu: close, duplicate view, open with OS, open in folder, copy path), Home tab (new/open/recover, recent files and folders, news, update notice, crash-data sharing option), context bar (per-tool options), toolbar, colour bar (fg/bg, palette, tiles), editor, timeline, status bar (frame field, zoom, tips), preview window, mini-editor/navigator. **[data]**
- **Workspace layouts** (dock system): Default and Mirrored Default, user layouts (New Layout, delete/restore), dock left/right/top/bottom, toggled with Shift+W; splitters for timeline and colour bar persisted. **[data]**
- **Multiple editors/views**: Duplicate View; drag tab to split; Show Menu via Alt+letter; Advanced Mode (Ctrl+F) hides all UI except the canvas; Fullscreen (F11); Run Command palette (Ctrl+Space).
- **Navigation**: pan (Space+drag / hand / middle), zoom presets and Fit Screen, scroll with Space+arrows (zoomed pixel) and Shift+Space+arrows (tile steps), Scroll to centre, mini editor (navigator).
- **Tiled mode**: None, Both axes, X, Y; draw in a repeating pattern view (View > Tiled Mode). **[data]**
- **Grid/pixel grid/rulers**: Show Grid (Ctrl+'), Show Pixel Grid (Ctrl+Shift+'), Grid Settings dialog (x, y, w, h), Selection as Grid, Snap to Grid (Shift+S). No rulers are listed in the data files; guides appear only as "layer edges" and "auto guides" overlays. **[data: no ruler option exists]**
- **Show overlays** (View > Show): Layer Edges, Selection Edges, Grid, Auto Guides, Slices, Pixel Grid, Tile Numbers, Brush Preview.
- **Checkerboard** background types: 16x16, 8x8, 4x4, 2x2, 1x1, custom; two colours; apply zoom. Swap checkerboard colours command.
- **Undo**: full history, Undo History window with Initial State entry, optional non-linear history (branches), each operation recorded as a named transaction (examples in `[tx]`: Sprite Crop, Remap Colors, Set Transparent Color, Clear Tiles, Cel Movement, Move Selection Edges, Paste Image/Cels/Frames/Layers/Slices, Set Frame Duration).
- **Safety/robustness**: auto data recovery, crash sessions, reopen closed files, read-only marking of newer-version files, undo memory limit.
- **Status feedback strings** show rules the editor enforces: locked layers, hidden layers, reference layers cannot be modified/transformed, background layer cannot move, read-only sprite, sprite locked by another editor, memory limits for transform/RotSprite.
- **Misc**: Steam integration (screenshots, in-game flag), update checker, license entry, news feed.

---

## 11. Count summary

| Item | Count | Source |
|---|---|---|
| Tools | 26 (in 12 toolbar groups); 3 quick-tool modifiers | gui.xml `<tools>` |
| User-selectable ink types | 5 (plus 14 internal inks) | gui.xml, strings |
| Command ids (gui.xml keys or menus) | 169 | gui.xml |
| Command ids named in en.ini | 194 (25 not in gui.xml) | en.ini [commands] |
| Keyboard binding entries for commands | 245 | gui.xml `<commands>` |
| Tool key bindings | 26 | gui.xml `<tools>` |
| Menus | 8 top-level + 11 context popups | gui.xml |
| Preference option keys | 344 (223 global in 51 sections, 29 per-tool, 92 per-document) | pref.xml |
| Preference enum types | 22 | pref.xml |
| Dialog widget files | 61 (several further dialogs have strings but no widget file, so they are built in code: Hue/Saturation, Brightness/Contrast, Color Curve, Mask By Color, Undo History, Palette editor) | widgets/ |
| Blend modes | 19 | en.ini |
| Convolution stock matrices | 24 | convmatr.def |
| Bundled palettes | 64 across 12 packs | extensions/ |
| UI languages | 30 | strings/ |
| CLI options | 57 (plus 2 aliases) | `--help` |

---

## Appendix A. Preference key names

Key names only (`section.key`). Types and default values were removed on 2026-10-12: the spec allows names and options from the shipped data files, not copies of them. `tool.*` keys repeat per tool id; `doc.*` keys are stored per document. Defaults, where parity needs them, are re-observed by running Aseprite.

```
general.language
general.screen_scale
general.ui_scale
general.gpu_acceleration
general.show_home
general.visible_timeline
general.autoshow_timeline
general.rewind_on_stop
general.expand_menubar_on_mouseover
general.data_recovery
general.data_recovery_period
general.keep_edited_sprite_data
general.keep_edited_sprite_data_for
general.keep_closed_sprite_on_memory
general.keep_closed_sprite_on_memory_for
general.show_full_path
general.edit_full_path
general.workspace_layout
general.timeline_position
general.timeline_layer_panel_width
general.show_menu_bar
general.recent_items
general.osx_async_view
general.x11_stylus_id
undo.size_limit
undo.goto_modified
undo.allow_nonlinear_history
undo.show_tooltip
editor.zoom_with_wheel
editor.zoom_with_slide
editor.zoom_from_center_with_wheel
editor.zoom_from_center_with_keys
editor.invert_brush_size_wheel
editor.show_scrollbars
editor.auto_scroll
editor.right_click_mode
editor.auto_select_layer
editor.auto_select_layer_quick
editor.straight_line_preview
editor.play_once
editor.play_all
editor.play_subtags
editor.auto_fit
editor.downsampling
cels.user_data_visibility
layers.user_data_visibility
sprite.user_data_visibility
tags.user_data_visibility
timeline.keep_selection
timeline.select_on_click
timeline.select_on_click_with_key
timeline.select_on_drag
timeline.drag_and_drop_from_edges
cursor.use_native_cursor
cursor.cursor_scale
cursor.cursor_color
cursor.painting_cursor_type
cursor.brush_preview
cursor.brush_preview_in_preview
cursor.tile_preview
cursor.snap_to_grid
preview.play_once
preview.play_all
preview.play_subtags
theme.selected
theme.font
theme.mini_font
tablet.api
tablet.set_cursor_fix
experimental.multiple_windows
experimental.new_render_engine
experimental.new_blend
experimental.compose_groups
experimental.use_native_clipboard
experimental.use_native_file_dialog
experimental.use_shaders_for_color_selectors
experimental.hue_with_sat_value_for_color_selector
experimental.one_finger_as_mouse_movement
experimental.load_wintab_driver
experimental.flash_layer
experimental.use_selection_tool_loop
experimental.nonactive_layers_opacity
experimental.nonactive_layers_opacity_preview
experimental.tooltip_delay
news.cache_file
context_bar.show_corner_radius
status_bar.focus_frame_field_on_mouseover
color_bar.box_size
color_bar.tiles_box_size
color_bar.fg_color
color_bar.bg_color
color_bar.fg_tile
color_bar.bg_tile
color_bar.selector
color_bar.discrete_wheel
color_bar.wheel_model
color_bar.harmony
color_bar.show_invalid_fg_bg_color_alert
color_bar.entries_separator
color_bar.show_color_and_tiles
color_bar.default_tileset_mode
updater.inits
updater.exits
updater.is_developer
updater.wait_days
updater.last_check
updater.uuid
updater.current_version
updater.new_version
updater.new_url
brush.pattern
save_brush.brush_type
save_brush.brush_size
save_brush.brush_angle
save_brush.fg_color
save_brush.bg_color
save_brush.image_color
save_brush.ink_type
save_brush.ink_opacity
save_brush.shade
save_brush.pixel_perfect
selection.mode
selection.move_edges
selection.modifiers_disable_handles
selection.move_on_add_mode
selection.pivot_visibility
selection.pivot_position
selection.opaque
selection.auto_opaque
selection.keep_selection_after_clear
selection.auto_show_selection_edges
selection.transparent_color
selection.rotation_algorithm
selection.modify_selection_quantity
selection.modify_selection_brush
selection.doubleclick_select_tile
selection.force_rotsprite
selection.multicel_when_layers_or_frames
selection.snap_to_grid
quantization.with_alpha
quantization.dithering_algorithm
quantization.dithering_factor
quantization.to_gray
quantization.advanced
quantization.rgbmap_algorithm
quantization.fit_criteria
eyedropper.channel
eyedropper.sample
eyedropper.discard_brush
shared.share_dynamics
shared.share_ink
shared.ink
new_file.width
new_file.height
new_file.color_mode
new_file.background_color
new_file.advanced
new_file.pixel_ratio
file_selector.current_folder
file_selector.zoom
file_selector.show_hidden
text_tool.font_face
text_tool.font_size
text_tool.antialias
text_tool.font_info
symmetry_mode.enabled
perf.show_render_time
guides.layer_edges_color
guides.auto_guides_color
slices.default_color
slices.user_data_visibility
slices.use_keys
advanced_mode.show_alert
open_file.open_sequence
save_file.show_file_format_doesnt_support_alert
save_file.show_export_animation_in_sequence_alert
save_file.default_extension
export_file.show_overwrite_files_alert
export_file.image_default_extension
export_file.animation_default_extension
sprite_sheet.show_overwrite_files_alert
sprite_sheet.default_extension
sprite_sheet.preview
sprite_sheet.sections
gif.show_alert
gif.interlaced
gif.loop
gif.preserve_palette_order
jpeg.show_alert
jpeg.quality
svg.show_alert
svg.pixel_scale
tga.show_alert
tga.bits_per_pixel
tga.compress
css.show_alert
css.pixel_scale
css.with_vars
css.generate_html
webp.show_alert
webp.loop
webp.type
webp.quality
webp.compression
webp.image_hint
webp.image_preset
hue_saturation.mode
filters.cels_target
scripts.show_run_script_alert
color.manage
color.working_rgb_space
color.files_with_profile
color.missing_profile
color.window_profile
color.window_profile_name
range.alpha
range.opacity
canvas_size.trim_outside
tileset.base_index
tileset.cache_compressed_tilesets
tileset.advanced
tilemap.show_delete_unused_tileset_alert
aseprite_format.cel_format
tool.opacity
tool.tolerance
tool.contiguous
tool.filled
tool.filled_preview
tool.ink
tool.freehand_algorithm
tool.corner_radius
tool.brush.type
tool.brush.size
tool.brush.angle
tool.spray.width
tool.spray.speed
tool.floodfill.stop_at_grid
tool.floodfill.refer_to
tool.floodfill.pixel_connectivity
tool.dynamics.stabilizer
tool.dynamics.stabilizer_factor
tool.dynamics.size
tool.dynamics.angle
tool.dynamics.gradient
tool.dynamics.min_size
tool.dynamics.min_angle
tool.dynamics.color_from_to
tool.dynamics.matrix_name
tool.dynamics.min_pressure_threshold
tool.dynamics.max_pressure_threshold
tool.dynamics.min_velocity_threshold
tool.dynamics.max_velocity_threshold
doc.site.frame
doc.site.layer
doc.tiled.mode
doc.symmetry.mode
doc.symmetry.x_axis
doc.symmetry.y_axis
doc.grid.snap
doc.grid.bounds
doc.grid.color
doc.grid.opacity
doc.grid.auto_opacity
doc.pixel_grid.color
doc.pixel_grid.opacity
doc.pixel_grid.auto_opacity
doc.bg.type
doc.bg.size
doc.bg.zoom
doc.bg.color1
doc.bg.color2
doc.timeline.first_frame
doc.thumbnails.zoom
doc.thumbnails.enabled
doc.thumbnails.overlay_enabled
doc.thumbnails.overlay_size
doc.thumbnails.scale_up_to_fit
doc.onionskin.active
doc.onionskin.prev_frames
doc.onionskin.next_frames
doc.onionskin.opacity_base
doc.onionskin.opacity_step
doc.onionskin.type
doc.onionskin.loop_tag
doc.onionskin.current_layer
doc.onionskin.position
doc.save_copy.filename
doc.save_copy.resize_scale
doc.save_copy.area
doc.save_copy.layer
doc.save_copy.layer_index
doc.save_copy.frame_tag
doc.save_copy.ani_dir
doc.save_copy.apply_pixel_ratio
doc.save_copy.for_twitter
doc.save_copy.play_subtags
doc.save_copy.ignore_empty
doc.sprite_sheet.defined
doc.sprite_sheet.type
doc.sprite_sheet.columns
doc.sprite_sheet.rows
doc.sprite_sheet.width
doc.sprite_sheet.height
doc.sprite_sheet.texture_filename
doc.sprite_sheet.data_filename
doc.sprite_sheet.data_format
doc.sprite_sheet.filename_format
doc.sprite_sheet.tagname_format
doc.sprite_sheet.border_padding
doc.sprite_sheet.shape_padding
doc.sprite_sheet.inner_padding
doc.sprite_sheet.trim_sprite
doc.sprite_sheet.trim
doc.sprite_sheet.trim_by_grid
doc.sprite_sheet.extrude
doc.sprite_sheet.merge_duplicates
doc.sprite_sheet.ignore_empty
doc.sprite_sheet.power_of_two_size
doc.sprite_sheet.open_generated
doc.sprite_sheet.layer
doc.sprite_sheet.layer_index
doc.sprite_sheet.frame_tag
doc.sprite_sheet.split_layers
doc.sprite_sheet.split_tags
doc.sprite_sheet.split_grid
doc.sprite_sheet.list_layers
doc.sprite_sheet.list_frame_tags
doc.sprite_sheet.list_slices
doc.import_sprite_sheet.type
doc.import_sprite_sheet.bounds
doc.import_sprite_sheet.padding_enabled
doc.import_sprite_sheet.padding_bounds
doc.import_sprite_sheet.partial_tiles
doc.preview.zoom
doc.preview.scroll
doc.preview.auto_scroll
doc.show.layer_edges
doc.show.selection_edges
doc.show.grid
doc.show.pixel_grid
doc.show.brush_preview
doc.show.slices
doc.show.auto_guides
doc.show.tile_numbers
```
