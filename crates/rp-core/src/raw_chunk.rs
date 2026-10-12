/// A chunk kept verbatim: the model has no field for its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawChunk {
    /// Index of the frame holding the chunk.
    pub frame: u32,
    /// Chunk type from the file format, e.g. 0x2005 for a cel.
    pub kind: u16,
    /// The chunk body: everything after the 6-byte chunk header.
    pub data: Vec<u8>,
}
