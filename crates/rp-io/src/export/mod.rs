//! PNG, GIF and sprite-sheet export. Bytes in, bytes out: no filesystem, no threads.

mod gif;
mod png;
mod sheet;

pub use self::gif::{GifFrame, encode_gif};
pub use self::png::encode_png;
pub use self::sheet::{
    JsonFormat, Sheet, SheetFrame, SheetMeta, SheetRect, SheetType, layout_sheet, sheet_json,
};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("nothing to export")]
    Empty,
    #[error("frames have different sizes")]
    MixedSizes,
    #[error("bad sheet layout: {0}")]
    BadLayout(String),
    #[error("encode: {0}")]
    Encode(String),
    #[error(transparent)]
    Image(#[from] rp_core::CoreError),
}
