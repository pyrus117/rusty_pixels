use crate::image::Image;
use crate::user_data::UserData;

#[derive(Debug, Clone, PartialEq)]
pub struct Cel {
    pub x: i32,
    pub y: i32,
    pub opacity: u8,
    pub z_index: i16,
    pub content: CelContent,
    pub extra: Option<CelExtra>,
    pub user_data: UserData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CelContent {
    Image(Image),
    /// Shares the image of the cel in the same layer at `frame`.
    Linked {
        frame: u32,
    },
    Tilemap(Tilemap),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub bits_per_tile: u16,
    pub tile_id_mask: u32,
    pub x_flip_mask: u32,
    pub y_flip_mask: u32,
    pub diagonal_flip_mask: u32,
    /// Row-major, width*height entries.
    pub tiles: Vec<u32>,
}

/// Values are raw 16.16 fixed point, as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CelExtra {
    pub flags: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
