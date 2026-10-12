use crate::image::Image;
use crate::user_data::UserData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalTilesetRef {
    pub file_id: u32,
    pub tileset_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tileset {
    pub id: u32,
    /// Raw flags as read (1 external, 2 tiles in file, 4 tile 0 is empty, 8/16/32 auto flips).
    pub flags: u32,
    pub tile_count: u32,
    pub tile_width: u16,
    pub tile_height: u16,
    pub base_index: i16,
    pub name: String,
    /// Present iff flags bit 1.
    pub external: Option<ExternalTilesetRef>,
    /// One image per tile (tile_width x tile_height) iff flags bit 2; empty otherwise.
    pub tiles: Vec<Image>,
    pub user_data: UserData,
    /// Indexed by tile; trailing empty entries are trimmed (normalised form).
    pub tile_user_data: Vec<UserData>,
}

impl Tileset {
    pub const EXTERNAL: u32 = 1;
    pub const TILES_IN_FILE: u32 = 2;
}
