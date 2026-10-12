/// Errors returned by rp-ops.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpError {
    #[error("no layer {0:?}")]
    UnknownLayer(rp_core::LayerId),
    #[error("no frame {0}")]
    UnknownFrame(u32),
    #[error("layer {0:?} is not an image layer")]
    NotAnImageLayer(rp_core::LayerId),
    #[error("({x}, {y}) is outside the {width}x{height} canvas")]
    OutOfCanvas {
        x: i32,
        y: i32,
        width: u16,
        height: u16,
    },
    #[error("a {pixel:?} pixel cannot go in a {sprite:?} sprite")]
    ModeMismatch {
        sprite: rp_core::ColorMode,
        pixel: rp_core::ColorMode,
    },
    #[error("palette has {len} colours; index {index} is out of range")]
    PaletteIndex { index: u32, len: usize },
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("canvas changed: you saw revision {expected}, it is now {actual}")]
    StaleRevision { expected: u64, actual: u64 },
    #[error("a transaction is already open")]
    TransactionOpen,
    #[error("no transaction is open")]
    NoTransaction,
}
