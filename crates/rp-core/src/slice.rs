use crate::user_data::UserData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceCenter {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pivot {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceKey {
    pub frame: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Present iff the slice's flags have bit 1.
    pub center: Option<SliceCenter>,
    /// Present iff the slice's flags have bit 2.
    pub pivot: Option<Pivot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slice {
    pub name: String,
    /// Raw flags: 1 = nine-patch, 2 = pivot. Other bits preserved as read.
    pub flags: u32,
    pub keys: Vec<SliceKey>,
    pub user_data: UserData,
}

impl Slice {
    pub const NINE_PATCH: u32 = 1;
    pub const PIVOT: u32 = 2;
}
