-- Generates the drawing-tool oracle with the installed Aseprite: each case is a
-- blank 16x16 sprite with one tool stroke, saved as <tool>_<params>.aseprite.
-- The parameters live in the file name so the Rust test can replay them.
-- Reference: research/vendor/aseprite-api/. Run with:
--   aseprite -b --script-param out=<dir> --script make_tool_fixtures.lua

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

local RED = Color{ r = 255, g = 0, b = 0, a = 255 }

-- The low bits of this LCG repeat with a short period, so callers get the high bits.
local function lcg(seed)
  local state = seed
  return function()
    state = (state * 1103515245 + 12345) % 2147483648
    return state // 65536
  end
end

local function save(sprite, name)
  local path = app.fs.joinPath(outDir, name .. ".aseprite")
  sprite:saveAs(path)
  if not app.fs.isFile(path) then
    error("file was not written: " .. path)
  end
  sprite:close()
end

local function blank()
  local sprite = Sprite(16, 16)
  app.activeSprite = sprite
  sprite:newCel(sprite.layers[1], 1)
  return sprite
end

local function stroke(tool, x0, y0, x1, y1)
  local sprite = blank()
  app.useTool{
    tool = tool,
    color = RED,
    brush = Brush(1),
    points = { Point(x0, y0), Point(x1, y1) },
  }
  save(sprite, string.format("%s_%d_%d_%d_%d", tool, x0, y0, x1, y1))
end

-- Hand-picked edge cases first, then deterministic random ones.
local fixed = {
  { 3, 3, 3, 3 }, { 0, 0, 15, 15 }, { 15, 0, 0, 15 }, { 2, 7, 13, 7 },
  { 7, 1, 7, 14 }, { 1, 2, 14, 5 }, { 14, 5, 1, 2 }, { 2, 1, 5, 14 },
  { 4, 4, 5, 5 }, { 4, 4, 6, 4 }, { 4, 4, 4, 5 },
}
local rand = lcg(42)
for _ = 1, 12 do
  fixed[#fixed + 1] = { rand() % 16, rand() % 16, rand() % 16, rand() % 16 }
end

for _, p in ipairs(fixed) do
  for _, tool in ipairs({ "line", "rectangle", "filled_rectangle", "ellipse", "filled_ellipse" }) do
    stroke(tool, p[1], p[2], p[3], p[4])
  end
end

-- Paint bucket on a maze of blue walls (one diagonal gap tests connectivity),
-- default tool options, from several seed points.
local function maze(sprite)
  local img = sprite.cels[1].image
  local walls = lcg(7)
  for y = 0, 15 do
    for x = 0, 15 do
      if walls() % 4 == 0 then
        img:drawPixel(x, y, app.pixelColor.rgba(0, 0, 255, 255))
      end
    end
  end
  for i = 0, 15 do
    img:drawPixel(8, i, app.pixelColor.rgba(0, 0, 255, 255))
  end
  -- A diagonal-only gap through the wall: (7,7) -> (8,8) -> (9,9), with the
  -- four direct neighbours of (8,8) walled.
  local clear = app.pixelColor.rgba(0, 0, 0, 0)
  local wall = app.pixelColor.rgba(0, 0, 255, 255)
  img:drawPixel(8, 8, clear)
  img:drawPixel(7, 7, clear)
  img:drawPixel(9, 9, clear)
  img:drawPixel(7, 8, wall)
  img:drawPixel(9, 8, wall)
end

-- Open cells in different regions, the diagonal gap, and one wall pixel.
local seeds = { { 1, 1 }, { 12, 1 }, { 5, 14 }, { 14, 13 }, { 8, 8 }, { 8, 3 } }
local mazeBase = blank()
maze(mazeBase)
save(mazeBase, "maze")
for _, s in ipairs(seeds) do
  local sprite = blank()
  maze(sprite)
  app.useTool{ tool = "paint_bucket", color = RED, points = { Point(s[1], s[2]) } }
  save(sprite, string.format("paint_bucket_%d_%d", s[1], s[2]))
end
