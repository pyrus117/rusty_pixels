#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use std::io::Write;

#[derive(Default)]
pub struct B(pub Vec<u8>);

impl B {
    pub fn new() -> Self {
        B(Vec::new())
    }
    pub fn u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }
    pub fn u16(mut self, v: u16) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn i16(mut self, v: i16) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn i32(mut self, v: i32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u64(mut self, v: u64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn i64(mut self, v: i64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn f32(mut self, v: f32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn f64(mut self, v: f64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn zeros(mut self, n: usize) -> Self {
        self.0.extend(std::iter::repeat_n(0u8, n));
        self
    }
    pub fn bytes(mut self, b: &[u8]) -> Self {
        self.0.extend_from_slice(b);
        self
    }
    pub fn string(self, s: &str) -> Self {
        self.u16(s.len() as u16).bytes(s.as_bytes())
    }
    pub fn done(self) -> Vec<u8> {
        self.0
    }
}

pub fn chunk(kind: u16, data: &[u8]) -> Vec<u8> {
    B::new()
        .u32(data.len() as u32 + 6)
        .u16(kind)
        .bytes(data)
        .done()
}

pub fn frame_counts(duration: u16, old: u16, new: u32, chunks: &[Vec<u8>]) -> Vec<u8> {
    let body = chunks.concat();
    B::new()
        .u32(body.len() as u32 + 16)
        .u16(0xF1FA)
        .u16(old)
        .u16(duration)
        .zeros(2)
        .u32(new)
        .bytes(&body)
        .done()
}

pub fn frame(duration: u16, chunks: &[Vec<u8>]) -> Vec<u8> {
    frame_counts(duration, chunks.len() as u16, chunks.len() as u32, chunks)
}

pub struct Header {
    pub frames: u16,
    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub flags: u32,
    pub speed: u16,
    pub transparent_index: u8,
    pub ncolors: u16,
    pub pixel_w: u8,
    pub pixel_h: u8,
    pub grid: (i16, i16, u16, u16),
}

impl Default for Header {
    fn default() -> Self {
        Header {
            frames: 1,
            width: 8,
            height: 8,
            depth: 32,
            flags: 1,
            speed: 100,
            transparent_index: 0,
            ncolors: 0,
            pixel_w: 1,
            pixel_h: 1,
            grid: (0, 0, 16, 16),
        }
    }
}

pub fn file(h: &Header, frames: &[Vec<u8>]) -> Vec<u8> {
    let body = frames.concat();
    B::new()
        .u32(128 + body.len() as u32)
        .u16(0xA5E0)
        .u16(h.frames)
        .u16(h.width)
        .u16(h.height)
        .u16(h.depth)
        .u32(h.flags)
        .u16(h.speed)
        .u32(0)
        .u32(0)
        .u8(h.transparent_index)
        .zeros(3)
        .u16(h.ncolors)
        .u8(h.pixel_w)
        .u8(h.pixel_h)
        .i16(h.grid.0)
        .i16(h.grid.1)
        .u16(h.grid.2)
        .u16(h.grid.3)
        .zeros(84)
        .bytes(&body)
        .done()
}

pub fn zlib(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

pub fn layer_chunk(
    flags: u16,
    kind: u16,
    child_level: u16,
    blend: u16,
    opacity: u8,
    name: &str,
) -> Vec<u8> {
    chunk(
        0x2004,
        &B::new()
            .u16(flags)
            .u16(kind)
            .u16(child_level)
            .u16(0)
            .u16(0)
            .u16(blend)
            .u8(opacity)
            .zeros(3)
            .string(name)
            .done(),
    )
}

/// Cel chunk header (everything before the type-specific part).
pub fn cel_head(layer: u16, x: i16, y: i16, opacity: u8, cel_type: u16, z: i16) -> B {
    B::new()
        .u16(layer)
        .i16(x)
        .i16(y)
        .u8(opacity)
        .u16(cel_type)
        .i16(z)
        .zeros(5)
}

pub fn ud_chunk(text: Option<&str>, color: Option<[u8; 4]>) -> Vec<u8> {
    let flags = text.map_or(0, |_| 1) | color.map_or(0, |_| 2);
    let mut b = B::new().u32(flags);
    if let Some(t) = text {
        b = b.string(t);
    }
    if let Some(c) = color {
        b = b.bytes(&c);
    }
    chunk(0x2020, &b.done())
}

/// Frame-0 chunk kinds, in order, as dump_chunks reports them.
pub fn kinds(bytes: &[u8], frame: u32) -> Vec<u16> {
    rp_io::dump_chunks(bytes)
        .unwrap()
        .into_iter()
        .filter(|c| c.frame == frame)
        .map(|c| c.kind)
        .collect()
}
