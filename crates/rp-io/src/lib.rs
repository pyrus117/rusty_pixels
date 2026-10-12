//! .aseprite file reading and writing for Rusty Pixels.

mod bytes;
mod chunks;
mod error;
mod read;
mod write;

pub use error::AseError;
pub use read::{dump_chunks, read};
pub use write::write;

/// Cap on the decoded bytes one [`read`] call produces. Every cel and tileset
/// charges its declared decoded size to this budget before it is inflated, so a
/// small compressed chunk cannot make the reader allocate without bound.
pub const DECODE_BUDGET: usize = 1 << 30;

/// Bytes charged per tile on top of its pixels: a tileset of tiny tiles still
/// allocates one image object per tile.
pub const OBJECT_OVERHEAD: usize = 64;

/// One chunk of a frame, as listed by [`dump_chunks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkInfo {
    /// Index of the frame holding the chunk.
    pub frame: u32,
    /// Chunk type from the file format, e.g. 0x2005 for a cel.
    pub kind: u16,
    /// The chunk's size field: the 6-byte chunk header plus the body.
    pub size: u32,
}
