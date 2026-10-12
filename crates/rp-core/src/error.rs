use crate::color::ColorMode;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    #[error("pixel data is {actual} bytes, expected {expected}")]
    DataLength { expected: usize, actual: usize },
    #[error("a {pixel:?} pixel cannot be written to a {image:?} image")]
    ModeMismatch { image: ColorMode, pixel: ColorMode },
    #[error("({x}, {y}) is outside the {width}x{height} image")]
    OutOfBounds {
        x: i64,
        y: i64,
        width: u32,
        height: u32,
    },
    #[error("a {width}x{height} image is too large")]
    TooLarge { width: u32, height: u32 },
}
