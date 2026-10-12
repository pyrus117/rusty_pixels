use crate::color::{ColorMode, PixelValue};
use crate::error::CoreError;

/// Row-major pixels, top row first, in the file's byte layout for `mode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    mode: ColorMode,
    data: Vec<u8>,
}

// Saturation to usize::MAX keeps an overflowing size reportable in DataLength
// instead of wrapping to a short buffer in release builds.
fn data_len(width: u32, height: u32, mode: ColorMode) -> usize {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(mode.bytes_per_pixel()))
        .unwrap_or(usize::MAX)
}

impl Image {
    /// All bytes zero (transparent for RGBA/greyscale, index 0 for indexed).
    pub fn new(width: u32, height: u32, mode: ColorMode) -> Image {
        Image {
            width,
            height,
            mode,
            data: vec![0u8; data_len(width, height, mode)],
        }
    }

    /// Errors with DataLength unless data.len() == width*height*bpp (computed without overflow;
    /// an overflowing size is a DataLength error with expected = usize::MAX).
    pub fn from_raw(
        width: u32,
        height: u32,
        mode: ColorMode,
        data: Vec<u8>,
    ) -> Result<Image, CoreError> {
        let expected = data_len(width, height, mode);
        if data.len() != expected {
            return Err(CoreError::DataLength {
                expected,
                actual: data.len(),
            });
        }
        Ok(Image {
            width,
            height,
            mode,
            data,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn mode(&self) -> ColorMode {
        self.mode
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_raw(self) -> Vec<u8> {
        self.data
    }

    /// The pixel's bytes, or None if out of bounds.
    pub fn get(&self, x: u32, y: u32) -> Option<&[u8]> {
        let off = self.offset(x, y)?;
        self.data.get(off..off + self.mode.bytes_per_pixel())
    }

    /// ModeMismatch if value.mode() != self.mode; OutOfBounds if x/y outside.
    pub fn put(&mut self, x: u32, y: u32, value: PixelValue) -> Result<(), CoreError> {
        if value.mode() != self.mode {
            return Err(CoreError::ModeMismatch {
                image: self.mode,
                pixel: value.mode(),
            });
        }
        let off = self.offset(x, y).ok_or(CoreError::OutOfBounds {
            x: i64::from(x),
            y: i64::from(y),
            width: self.width,
            height: self.height,
        })?;
        let end = off + self.mode.bytes_per_pixel();
        let src = value.to_bytes();
        let len = self.data.len();
        let dst = self.data.get_mut(off..end).ok_or(CoreError::DataLength {
            expected: end,
            actual: len,
        })?;
        dst.copy_from_slice(&src);
        Ok(())
    }

    fn offset(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let bpp = self.mode.bytes_per_pixel();
        Some(y as usize * (self.width as usize * bpp) + x as usize * bpp)
    }
}
