-- Generates the rp-render oracle sprites with the installed Aseprite.
-- tools/fixtures/render_oracle.sh runs this, then has the Aseprite CLI flatten
-- each sprite to PNG; those PNGs are the expected renders.
-- Reference: research/vendor/aseprite-api/. Run with:
--   aseprite -b --script-param out=<dir> --script make_render_fixtures.lua

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

-- Deterministic LCG so the oracle is reproducible without math.random.
local function lcg(seed)
  local state = seed
  return function()
    state = (state * 1103515245 + 12345) % 2147483648
    return state
  end
end

-- Alphas that stress rounding: the extremes, the midpoint pair, and odd values.
local ALPHAS = { 0, 1, 2, 64, 127, 128, 129, 200, 254, 255 }

local function noise(image, seed)
  local rand = lcg(seed)
  for y = 0, image.height - 1 do
    for x = 0, image.width - 1 do
      local r = rand() % 256
      local g = rand() % 256
      local b = rand() % 256
      local a = ALPHAS[(rand() % #ALPHAS) + 1]
      image:drawPixel(x, y, app.pixelColor.rgba(r, g, b, a))
    end
  end
end

local MODES = {
  { "normal", BlendMode.NORMAL },
  { "multiply", BlendMode.MULTIPLY },
  { "screen", BlendMode.SCREEN },
  { "overlay", BlendMode.OVERLAY },
  { "darken", BlendMode.DARKEN },
  { "lighten", BlendMode.LIGHTEN },
  { "color_dodge", BlendMode.COLOR_DODGE },
  { "color_burn", BlendMode.COLOR_BURN },
  { "hard_light", BlendMode.HARD_LIGHT },
  { "soft_light", BlendMode.SOFT_LIGHT },
  { "difference", BlendMode.DIFFERENCE },
  { "exclusion", BlendMode.EXCLUSION },
  { "hue", BlendMode.HSL_HUE },
  { "saturation", BlendMode.HSL_SATURATION },
  { "color", BlendMode.HSL_COLOR },
  { "luminosity", BlendMode.HSL_LUMINOSITY },
  { "addition", BlendMode.ADDITION },
  { "subtract", BlendMode.SUBTRACT },
  { "divide", BlendMode.DIVIDE },
}

-- One sprite per blend mode and opacity variant: noise over noise.
-- "full" isolates the blend formula; "op" adds layer and cel opacity.
local function blend(modeName, mode, variant, layerOpacity, celOpacity)
  local sprite = Sprite(16, 16)
  local bottom = sprite.layers[1]
  bottom.name = "bottom"
  local bottomCel = sprite:newCel(bottom, 1)
  noise(bottomCel.image, 1)

  local top = sprite:newLayer()
  top.name = "top"
  top.blendMode = mode
  top.opacity = layerOpacity
  local topCel = sprite:newCel(top, 1)
  noise(topCel.image, 2)
  topCel.opacity = celOpacity

  save(sprite, "blend_" .. modeName .. "_" .. variant .. ".aseprite")
end

-- Layer tree: groups with opacity and blend, a hidden layer, cel offsets that
-- overhang every edge.
local function groups()
  local sprite = Sprite(12, 12)
  local bg = sprite.layers[1]
  bg.name = "bg"
  local bgCel = sprite:newCel(bg, 1)
  noise(bgCel.image, 3)

  local group = sprite:newGroup()
  group.name = "group"
  group.opacity = 150
  group.blendMode = BlendMode.MULTIPLY

  local inner = sprite:newLayer()
  inner.name = "inner"
  inner.parent = group
  local innerImage = Image(8, 8)
  noise(innerImage, 4)
  sprite:newCel(inner, 1, innerImage, Point(-3, 6))

  local inner2 = sprite:newLayer()
  inner2.name = "inner2"
  inner2.parent = group
  inner2.blendMode = BlendMode.SCREEN
  inner2.opacity = 190
  local inner2Image = Image(9, 5)
  noise(inner2Image, 5)
  sprite:newCel(inner2, 1, inner2Image, Point(7, -2))

  local hidden = sprite:newLayer()
  hidden.name = "hidden"
  hidden.isVisible = false
  local hiddenCel = sprite:newCel(hidden, 1)
  noise(hiddenCel.image, 6)

  local over = sprite:newLayer()
  over.name = "over"
  over.opacity = 100
  local overImage = Image(4, 4)
  noise(overImage, 7)
  sprite:newCel(over, 1, overImage, Point(4, 4))

  save(sprite, "groups.aseprite")
end

-- A hidden group must hide its visible children too.
local function hiddenGroup()
  local sprite = Sprite(6, 6)
  local bg = sprite.layers[1]
  bg.name = "bg"
  noise(sprite:newCel(bg, 1).image, 8)
  local group = sprite:newGroup()
  group.name = "off"
  group.isVisible = false
  local child = sprite:newLayer()
  child.name = "child"
  child.parent = group
  noise(sprite:newCel(child, 1).image, 9)
  save(sprite, "hidden_group.aseprite")
end

-- Three frames with different cels and an empty frame in the middle.
local function frames()
  local sprite = Sprite(6, 6)
  local layer = sprite.layers[1]
  layer.name = "ink"
  noise(sprite:newCel(layer, 1).image, 10)
  sprite:newEmptyFrame(2)
  sprite:newEmptyFrame(3)
  local img = Image(3, 3)
  noise(img, 11)
  sprite:newCel(layer, 3, img, Point(2, 1))
  save(sprite, "frames.aseprite")
end

local function grayscale()
  local sprite = Sprite(8, 8, ColorMode.GRAY)
  local bottom = sprite.layers[1]
  bottom.name = "bottom"
  local rand = lcg(12)
  local bottomCel = sprite:newCel(bottom, 1)
  local top = sprite:newLayer()
  top.name = "top"
  top.opacity = 160
  local topCel = sprite:newCel(top, 1)
  for y = 0, 7 do
    for x = 0, 7 do
      bottomCel.image:drawPixel(x, y,
        app.pixelColor.graya(rand() % 256, ALPHAS[(rand() % #ALPHAS) + 1]))
      topCel.image:drawPixel(x, y,
        app.pixelColor.graya(rand() % 256, ALPHAS[(rand() % #ALPHAS) + 1]))
    end
  end
  save(sprite, "grayscale.aseprite")
end

-- Indexed at full opacity only: the PNG export of an indexed sprite is itself
-- indexed, so partial opacity has no exact expected value.
local function indexed()
  local sprite = Sprite(8, 8, ColorMode.INDEXED)
  sprite.transparentColor = 0
  local pal = Palette(8)
  local rand = lcg(13)
  for i = 0, 7 do
    pal:setColor(i, Color{ r = rand() % 256, g = rand() % 256, b = rand() % 256, a = 255 })
  end
  sprite:setPalette(pal)
  local bottom = sprite.layers[1]
  bottom.name = "bottom"
  local bottomCel = sprite:newCel(bottom, 1)
  local top = sprite:newLayer()
  top.name = "top"
  local topCel = sprite:newCel(top, 1)
  for y = 0, 7 do
    for x = 0, 7 do
      bottomCel.image:drawPixel(x, y, rand() % 8)
      -- Index 0 is the transparent colour, so about half the top layer shows through.
      local v = rand() % 16
      if v >= 8 then v = 0 end
      topCel.image:drawPixel(x, y, v)
    end
  end
  save(sprite, "indexed.aseprite")
end

for _, m in ipairs(MODES) do
  blend(m[1], m[2], "full", 255, 255)
  blend(m[1], m[2], "op", 173, 211)
end
groups()
hiddenGroup()
frames()
grayscale()
indexed()
