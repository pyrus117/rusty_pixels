use crate::error::AseError;

/// Reads little-endian primitives from a fixed slice. Every read is
/// bounds-checked, so a parser handed a short slice gets `Truncated`, never a
/// panic.
pub(crate) struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

// The full primitive set mirrors ByteWriter; this task only needs part of it.
#[allow(dead_code)]
impl<'a> ByteReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        ByteReader { data, pos: 0 }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], AseError> {
        let end = self.pos.checked_add(n).ok_or(AseError::Truncated {
            offset: self.pos,
            needed: n,
        })?;
        let out = self.data.get(self.pos..end).ok_or(AseError::Truncated {
            offset: self.pos,
            needed: n,
        })?;
        self.pos = end;
        Ok(out)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], AseError> {
        let offset = self.pos;
        let out = self.take(N)?;
        out.try_into()
            .map_err(|_| AseError::Truncated { offset, needed: N })
    }

    pub(crate) fn u8(&mut self) -> Result<u8, AseError> {
        Ok(u8::from_le_bytes(self.array()?))
    }

    pub(crate) fn u16(&mut self) -> Result<u16, AseError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn i16(&mut self) -> Result<i16, AseError> {
        Ok(i16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, AseError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn i32(&mut self) -> Result<i32, AseError> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, AseError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub(crate) fn i64(&mut self) -> Result<i64, AseError> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    pub(crate) fn f32(&mut self) -> Result<f32, AseError> {
        Ok(f32::from_le_bytes(self.array()?))
    }

    pub(crate) fn f64(&mut self) -> Result<f64, AseError> {
        Ok(f64::from_le_bytes(self.array()?))
    }

    pub(crate) fn bytes(&mut self, n: usize) -> Result<&'a [u8], AseError> {
        self.take(n)
    }

    pub(crate) fn skip(&mut self, n: usize) -> Result<(), AseError> {
        self.take(n).map(|_| ())
    }

    /// STRING: a WORD byte length, then that many UTF-8 bytes.
    pub(crate) fn string(&mut self) -> Result<String, AseError> {
        let len = usize::from(self.u16()?);
        let raw = self.bytes(len)?;
        let s = std::str::from_utf8(raw).map_err(|e| AseError::Invalid {
            what: "string",
            detail: e.to_string(),
        })?;
        Ok(s.to_owned())
    }
}

/// Writes little-endian primitives into a growing buffer. This is the mirror
/// of `ByteReader`, so a value round-trips through the two.
#[derive(Default)]
pub(crate) struct ByteWriter {
    buf: Vec<u8>,
}

// Same primitive set as ByteReader; later tasks use the rest.
#[allow(dead_code)]
impl ByteWriter {
    pub(crate) fn new() -> Self {
        ByteWriter::default()
    }

    pub(crate) fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub(crate) fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn i16(&mut self, v: i16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }

    pub(crate) fn string(&mut self, s: &str) -> Result<(), AseError> {
        let len = u16::try_from(s.len()).map_err(|_| AseError::OutOfRange {
            what: "string length",
            value: i64::try_from(s.len()).unwrap_or(i64::MAX),
        })?;
        self.u16(len);
        self.bytes(s.as_bytes());
        Ok(())
    }

    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}
