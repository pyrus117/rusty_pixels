#![allow(clippy::unwrap_used, clippy::expect_used)]
use rp_core::*;

#[test]
fn new_is_transparent_and_put_get_round_trip() {
    let mut img = RgbaImage::new(3, 2).unwrap();
    assert_eq!(img.data(), &[0u8; 24][..]);
    img.put(2, 1, [1, 2, 3, 4]).unwrap();
    assert_eq!(img.get(2, 1), Some([1, 2, 3, 4]));
    assert_eq!(img.data()[20..24], [1, 2, 3, 4]);
    assert_eq!(img.get(3, 0), None);
    assert!(matches!(
        img.put(0, 2, [0; 4]),
        Err(CoreError::OutOfBounds { .. })
    ));
}

#[test]
fn sizes_are_checked() {
    assert!(matches!(
        RgbaImage::new(u32::MAX, u32::MAX),
        Err(CoreError::TooLarge { .. })
    ));
    assert!(matches!(
        RgbaImage::new(65536, 65536),
        Err(CoreError::TooLarge { .. })
    ));
    assert!(matches!(
        RgbaImage::from_raw(2, 2, vec![0; 15]),
        Err(CoreError::DataLength {
            expected: 16,
            actual: 15
        })
    ));
    assert!(RgbaImage::new(0, 0).unwrap().data().is_empty());
}
