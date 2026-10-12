-- Generates the rp-io test fixtures with the installed Aseprite.
-- Reference: research/vendor/aseprite-api/. Run with:
--   aseprite -b --script-param out=<dir> --script make_fixtures.lua

local outDir = app.params.out
if outDir == nil or outDir == "" then
  error("--script-param out=<directory> is required")
end

if not app.fs.isDirectory(outDir) then
  app.fs.makeAllDirectories(outDir)
end
if not app.fs.isDirectory(outDir) then
  error("cannot create output directory: " .. outDir)
end

local function save(sprite, name)
  local path = app.fs.joinPath(outDir, name)
  sprite:saveAs(path)
  if not app.fs.isFile(path) then
    error("file was not written: " .. path)
  end
  sprite:close()
  print("wrote " .. name)
end

local function rgbaBasic()
  local sprite = Sprite(8, 8)
  local layer = sprite.layers[1]
  layer.name = "ink"
  local cel = sprite:newCel(layer)
  cel.image:drawPixel(1, 1, app.pixelColor.rgba(255, 0, 0, 255))
  cel.image:drawPixel(6, 6, app.pixelColor.rgba(0, 255, 0, 128))
  save(sprite, "rgba_basic.aseprite")
end

local function grayscale()
  local sprite = Sprite(4, 4, ColorMode.GRAY)
  local cel = sprite:newCel(sprite.layers[1])
  cel.image:drawPixel(0, 0, app.pixelColor.graya(200, 255))
  save(sprite, "grayscale.aseprite")
end

local function indexed()
  local sprite = Sprite(4, 4, ColorMode.INDEXED)
  sprite.transparentColor = 0
  -- The API cannot name palette entries, so entry 2 is plain red.
  local pal = Palette(4)
  pal:setColor(0, Color{ r = 0, g = 0, b = 0 })
  pal:setColor(1, Color{ r = 255, g = 255, b = 255 })
  pal:setColor(2, Color{ r = 255, g = 0, b = 0 })
  pal:setColor(3, Color{ r = 0, g = 255, b = 0 })
  sprite:setPalette(pal)
  local cel = sprite:newCel(sprite.layers[1])
  cel.image:drawPixel(2, 2, 2)
  save(sprite, "indexed.aseprite")
end

local function layersTree()
  local sprite = Sprite(8, 8)
  local bg = sprite.layers[1]
  bg.name = "bg"

  local body = sprite:newGroup()
  body.name = "body"
  local arm = sprite:newLayer()
  arm.name = "arm"
  arm.parent = body
  arm.blendMode = BlendMode.MULTIPLY
  arm.opacity = 128

  local head = sprite:newGroup()
  head.name = "head"
  head.isCollapsed = true
  local eye = sprite:newLayer()
  eye.name = "eye"
  eye.parent = head
  eye.isVisible = false

  save(sprite, "layers_tree.aseprite")
end

local function animation()
  local sprite = Sprite(8, 8)
  local layer = sprite.layers[1]
  layer.name = "a"
  sprite:newEmptyFrame(2)
  sprite:newEmptyFrame(3)
  sprite:newEmptyFrame(4)

  local img = Image(8, 8, ColorMode.RGB)
  sprite:newCel(layer, 1, img, Point(0, 0))
  sprite:newCel(layer, 2, img:clone(), Point(0, 0))
  sprite:newCel(layer, 3, img:clone(), Point(0, 0))

  -- LinkCels replaces the merged cels, so re-fetch them afterwards.
  app.range.layers = { layer }
  app.range.frames = { 1, 2, 3 }
  app.command.LinkCels()

  local c1 = layer:cel(1)
  local c2 = layer:cel(2)
  local c3 = layer:cel(3)
  if c1 == nil or c2 == nil or c3 == nil then
    error("LinkCels left cels missing in frames 1-3")
  end
  if c1.image.id ~= c2.image.id or c1.image.id ~= c3.image.id then
    error("LinkCels did not link the cels in frames 1-3")
  end

  c1.opacity = 64
  c1.zIndex = 1

  sprite.frames[1].duration = 0.1
  sprite.frames[2].duration = 0.15
  sprite.frames[3].duration = 0.2
  sprite.frames[4].duration = 0.25

  local fwd = sprite:newTag(1, 2)
  fwd.name = "fwd"
  fwd.aniDir = AniDir.FORWARD
  fwd.color = Color{ r = 255, g = 0, b = 0 }

  local rev = sprite:newTag(1, 2)
  rev.name = "rev"
  rev.aniDir = AniDir.REVERSE
  rev.repeats = 3
  rev.color = Color{ r = 0, g = 255, b = 0 }

  local pp = sprite:newTag(1, 4)
  pp.name = "pp"
  pp.aniDir = AniDir.PING_PONG
  pp.color = Color{ r = 0, g = 0, b = 255 }

  local ppr = sprite:newTag(3, 4)
  ppr.name = "ppr"
  ppr.aniDir = AniDir.PING_PONG_REVERSE
  ppr.color = Color{ r = 255, g = 255, b = 0 }

  save(sprite, "animation.aseprite")
end

local function slices()
  local sprite = Sprite(16, 16)
  local slice = sprite:newSlice(Rectangle(1, 1, 10, 8))
  slice.name = "btn"
  -- Centre and pivot are in local coordinates relative to the slice bounds.
  slice.center = Rectangle(2, 2, 6, 4)
  slice.pivot = Point(5, 4)
  save(sprite, "slices.aseprite")
end

local function userData()
  local sprite = Sprite(4, 4)
  sprite.data = "sprite note"

  local layer = sprite.layers[1]
  layer.color = Color{ r = 0, g = 0, b = 255 }
  layer.data = "layer note"

  local cel = sprite:newCel(layer)
  cel.data = "cel note"

  local tag = sprite:newTag(1, 1)
  tag.name = "tag"
  tag.color = Color{ r = 255, g = 255, b = 0 }

  -- A slice with no bounds is not saved, so give it the whole canvas.
  local slice = sprite:newSlice(Rectangle(0, 0, 4, 4))
  slice.name = "slice"
  slice.data = "slice note"

  layer.properties.flag = true
  layer.properties.count = 7
  layer.properties.ratio = 1.5
  layer.properties.label = "hello"
  layer.properties.pos = Point(3, 4)
  layer.properties.size = Size(5, 6)
  layer.properties.rect = Rectangle(1, 2, 3, 4)
  layer.properties.nums = { 1, 2, 3 }
  layer.properties.mixed = { 1, "two", 3.5 }
  layer.properties.nested = { x = 1, y = 2 }
  layer.properties.id = Uuid("74341b56-4f7f-4ab1-9495-58cf5bce0e1c")

  layer.properties("rusty/fixtures", { enabled = true, name = "fixtures" })

  save(sprite, "user_data.aseprite")
end

local function tilemap()
  local sprite = Sprite(16, 8)
  app.command.NewLayer{ name = "map", tilemap = true, gridBounds = Rectangle(0, 0, 8, 8) }
  local map = sprite.layers[#sprite.layers]
  if map == nil or not map.isTilemap then
    error("tilemap layer was not created")
  end
  sprite:deleteLayer(sprite.layers[1])

  -- The auto-created tileset holds only the empty tile; grow it to 3 tiles.
  local tileset = map.tileset
  sprite:newTile(tileset)
  sprite:newTile(tileset)
  tileset:tile(1).data = "grass"

  -- A tilemap cel image stores one raw tile reference per pixel.
  local img = Image(2, 1, ColorMode.TILEMAP)
  img:drawPixel(0, 0, 0x80000002)
  sprite:newCel(map, 1, img, Point(0, 0))

  save(sprite, "tilemap.aseprite")
end

local function uuidLayers()
  local sprite = Sprite(4, 4)
  sprite.useLayerUuids = true
  sprite:newLayer()
  save(sprite, "uuid_layers.aseprite")
end

local function ratioGrid()
  local sprite = Sprite(4, 4)
  sprite.pixelRatio = Size(2, 1)
  sprite.gridBounds = Rectangle(1, 2, 4, 4)
  save(sprite, "ratio_grid.aseprite")
end

rgbaBasic()
grayscale()
indexed()
layersTree()
animation()
slices()
userData()
tilemap()
uuidLayers()
ratioGrid()
