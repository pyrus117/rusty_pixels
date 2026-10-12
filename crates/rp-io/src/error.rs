#[derive(Debug, thiserror::Error)]
pub enum AseError {
    #[error("not an .aseprite file (magic {found:#06x})")]
    BadMagic { found: u16 },
    #[error("bad frame magic {found:#06x} in frame {frame}")]
    BadFrameMagic { frame: u32, found: u16 },
    #[error("unsupported colour depth {0}")]
    UnsupportedDepth(u16),
    #[error("file ends early: needed {needed} bytes at offset {offset}")]
    Truncated { offset: usize, needed: usize },
    #[error("invalid {what}: {detail}")]
    Invalid { what: &'static str, detail: String },
    #[error("{what} does not fit the file format: {value}")]
    OutOfRange { what: &'static str, value: i64 },
    #[error("zlib: {0}")]
    Zlib(String),
    #[error(transparent)]
    Core(#[from] rp_core::CoreError),
}
