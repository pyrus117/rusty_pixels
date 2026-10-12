#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub rgba: [u8; 4],
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palette {
    pub entries: Vec<PaletteEntry>,
}
