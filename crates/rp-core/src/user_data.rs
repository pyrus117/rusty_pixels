#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserData {
    pub text: Option<String>,
    pub color: Option<[u8; 4]>,
    /// Normalised: no "has properties" flag is the same as zero maps.
    pub properties: Vec<PropertiesMap>,
}

impl UserData {
    /// No text, no colour, no property maps.
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.color.is_none() && self.properties.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertiesMap {
    /// 0 = user properties; otherwise an ExternalFile id naming the extension.
    pub key: u32,
    /// Order preserved as in the file.
    pub properties: Vec<(String, PropValue)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    /// Raw 16.16.
    Fixed(i32),
    F32(f32),
    F64(f64),
    String(String),
    Point {
        x: i32,
        y: i32,
    },
    Size {
        width: i32,
        height: i32,
    },
    Rect {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    /// element_type 0 = mixed (each element carries its own type in the file).
    Vector {
        element_type: u16,
        items: Vec<PropValue>,
    },
    Map(Vec<(String, PropValue)>),
    Uuid([u8; 16]),
}

impl PropValue {
    /// The file-format type code: Bool = 0x01 … Uuid = 0x13.
    pub fn type_code(&self) -> u16 {
        match self {
            PropValue::Bool(_) => 0x01,
            PropValue::I8(_) => 0x02,
            PropValue::U8(_) => 0x03,
            PropValue::I16(_) => 0x04,
            PropValue::U16(_) => 0x05,
            PropValue::I32(_) => 0x06,
            PropValue::U32(_) => 0x07,
            PropValue::I64(_) => 0x08,
            PropValue::U64(_) => 0x09,
            PropValue::Fixed(_) => 0x0A,
            PropValue::F32(_) => 0x0B,
            PropValue::F64(_) => 0x0C,
            PropValue::String(_) => 0x0D,
            PropValue::Point { .. } => 0x0E,
            PropValue::Size { .. } => 0x0F,
            PropValue::Rect { .. } => 0x10,
            PropValue::Vector { .. } => 0x11,
            PropValue::Map(_) => 0x12,
            PropValue::Uuid(_) => 0x13,
        }
    }
}
