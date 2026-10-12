use crate::error::CoreError;

/// Largest flattened image `RgbaImage::new` will allocate. Renders and sheets
/// come from caller-chosen sizes, so the cap turns a hostile size into an error.
pub const MAX_RGBA_BYTES: usize = 1 << 30;

/// A flattened RGBA8 image: straight (not premultiplied) alpha, row-major,
/// top row first. The shared currency of rendering and export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

fn byte_len(width: u32, height: u32) -> Result<usize, CoreError> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))
        .filter(|&len| len <= MAX_RGBA_BYTES)
        .ok_or(CoreError::TooLarge { width, height })
}

impl RgbaImage {
    /// Fully transparent. TooLarge past MAX_RGBA_BYTES.
    pub fn new(width: u32, height: u32) -> Result<RgbaImage, CoreError> {
        Ok(RgbaImage {
            width,
            height,
            data: vec![0u8; byte_len(width, height)?],
        })
    }

    /// TooLarge past MAX_RGBA_BYTES; DataLength unless data.len() == width*height*4.
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Result<RgbaImage, CoreError> {
        let expected = byte_len(width, height)?;
        if data.len() != expected {
            return Err(CoreError::DataLength {
                expected,
                actual: data.len(),
            });
        }
        Ok(RgbaImage {
            width,
            height,
            data,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_raw(self) -> Vec<u8> {
        self.data
    }

    /// None if out of bounds.
    pub fn get(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let off = self.offset(x, y)?;
        let px = self.data.get(off..off + 4)?;
        Some([px[0], px[1], px[2], px[3]])
    }

    /// OutOfBounds if x/y outside.
    pub fn put(&mut self, x: u32, y: u32, rgba: [u8; 4]) -> Result<(), CoreError> {
        let oob = CoreError::OutOfBounds {
            x: i64::from(x),
            y: i64::from(y),
            width: self.width,
            height: self.height,
        };
        let off = self.offset(x, y).ok_or(oob.clone())?;
        let dst = self.data.get_mut(off..off + 4).ok_or(oob)?;
        dst.copy_from_slice(&rgba);
        Ok(())
    }

    fn offset(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y as usize * self.width as usize + x as usize) * 4)
    }
}
