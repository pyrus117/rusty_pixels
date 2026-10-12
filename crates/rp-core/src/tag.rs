use crate::user_data::UserData;

/// Discriminants are the file-format values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AniDir {
    Forward = 0,
    Reverse = 1,
    PingPong = 2,
    PingPongReverse = 3,
}

impl AniDir {
    pub fn from_u8(v: u8) -> Option<AniDir> {
        match v {
            0 => Some(AniDir::Forward),
            1 => Some(AniDir::Reverse),
            2 => Some(AniDir::PingPong),
            3 => Some(AniDir::PingPongReverse),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub from: u32,
    pub to: u32,
    pub direction: AniDir,
    pub repeat: u16,
    /// Deprecated colour kept for files written by Aseprite 1.2; the real colour is user_data.color.
    pub legacy_rgb: [u8; 3],
    pub name: String,
    pub user_data: UserData,
}
