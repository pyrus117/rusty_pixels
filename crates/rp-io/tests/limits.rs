#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::*;
use rp_io::AseError;

#[test]
fn oversized_palette_is_rejected_before_allocating() {
    let huge = chunk(
        0x2019,
        &B::new()
            .u32(u32::MAX)
            .u32(0)
            .u32(0)
            .zeros(8)
            .u16(0)
            .bytes(&[0, 0, 0, 255])
            .done(),
    );
    let r = rp_io::read(&file(&Header::default(), &[frame(100, &[huge])]));
    assert!(matches!(
        r,
        Err(AseError::Invalid {
            what: "palette size",
            ..
        })
    ));
    let max = chunk(
        0x2019,
        &B::new()
            .u32(65535)
            .u32(0)
            .u32(0)
            .zeros(8)
            .u16(0)
            .bytes(&[0, 0, 0, 255])
            .done(),
    );
    let s = rp_io::read(&file(&Header::default(), &[frame(100, &[max])])).unwrap();
    assert_eq!(s.palette.entries.len(), 65535);
}
