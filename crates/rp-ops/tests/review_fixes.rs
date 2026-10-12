#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Regression tests for the M0 independent review (findings M4, L5, L8, L12).
use rp_core::*;
use rp_ops::*;

const RED: PixelValue = PixelValue::Rgba {
    r: 255,
    g: 0,
    b: 0,
    a: 255,
};

fn image_layer(id: u32) -> Layer {
    Layer {
        id: LayerId(id),
        parent: None,
        name: format!("l{id}"),
        kind: LayerKind::Image,
        flags: LayerFlags(3),
        blend_mode: BlendMode::Normal,
        opacity: 255,
        uuid: None,
        user_data: UserData::default(),
    }
}

/// 8x8 RGBA sprite, one image layer (id 0) whose frame-0 cel is a blank w x h image at (x, y).
fn with_cel(x: i32, y: i32, w: u32, h: u32) -> Sprite {
    let mut s = Sprite::new(8, 8, ColorMode::Rgba);
    s.layers = vec![image_layer(0)];
    s.cels.insert(
        (LayerId(0), 0),
        Cel {
            x,
            y,
            opacity: 255,
            z_index: 0,
            content: CelContent::Image(Image::new(w, h, ColorMode::Rgba)),
            extra: None,
            user_data: UserData::default(),
        },
    );
    s
}

fn paint(e: &mut Engine, x: i32, y: i32) -> Result<Outcome, OpError> {
    e.apply(
        Command::SetPixels(SetPixels {
            layer: LayerId(0),
            frame: 0,
            pixels: vec![PixelWrite { x, y, value: RED }],
        }),
        Origin::Human,
    )
}

fn cel_bounds(e: &Engine) -> (i32, i32, u32, u32) {
    let cel = e.sprite().cel(LayerId(0), 0).unwrap();
    match &cel.content {
        CelContent::Image(i) => (cel.x, cel.y, i.width(), i.height()),
        other => panic!("{other:?}"),
    }
}

// M4, revised after the second review (O1): growth is bounded by memory (MAX_CEL_BYTES) and
// the file format, not by area, so painting next to an overhanging cel keeps working.
#[test]
fn growth_toward_a_far_offscreen_cel_is_refused() {
    let mut e = Engine::new(with_cel(-32000, -32000, 1, 1));
    let before = e.sprite().clone();
    assert!(matches!(
        paint(&mut e, 7, 7),
        Err(OpError::InvalidArgument(_))
    ));
    assert_eq!(e.sprite(), &before);
    assert_eq!(e.revision(), 0);
}

#[test]
fn growth_within_canvas_plus_old_cel_is_allowed() {
    let mut e = Engine::new(with_cel(-8, -8, 16, 16));
    paint(&mut e, 7, 7).unwrap();
    assert_eq!(cel_bounds(&e), (-8, -8, 16, 16));
    let mut e = Engine::new(with_cel(0, 0, 8, 1));
    paint(&mut e, 0, 7).unwrap();
    assert_eq!(cel_bounds(&e), (0, 0, 8, 8));
}

#[test]
fn painting_the_far_corner_of_an_overhanging_cel_is_allowed() {
    let mut s = with_cel(-16, -16, 32, 32);
    s.width = 64;
    s.height = 64;
    let mut e = Engine::new(s);
    paint(&mut e, 63, 63).unwrap();
    assert_eq!(cel_bounds(&e), (-16, -16, 80, 80));
    let mut e = Engine::new(with_cel(-1, -1, 2, 2));
    paint(&mut e, 7, 7).unwrap();
    assert_eq!(cel_bounds(&e), (-1, -1, 9, 9));
    // 4008 x 4008 RGBA is 64 MiB: large but legitimate, so it is allowed.
    let mut e = Engine::new(with_cel(-4000, -4000, 1, 1));
    paint(&mut e, 7, 7).unwrap();
    assert_eq!(cel_bounds(&e), (-4000, -4000, 4008, 4008));
    const { assert!(rp_ops::MAX_CEL_BYTES >= 4008 * 4008 * 4) };
}

// L8
#[test]
fn zero_size_old_cel_does_not_widen_the_bounds() {
    let mut e = Engine::new(with_cel(90, 90, 0, 0));
    paint(&mut e, 0, 0).unwrap();
    assert_eq!(cel_bounds(&e), (0, 0, 1, 1));
}

// L5
#[test]
fn exhausted_layer_ids_are_an_error_not_a_duplicate() {
    let mut s = Sprite::new(4, 4, ColorMode::Rgba);
    s.layers = vec![image_layer(u32::MAX)];
    let mut e = Engine::new(s);
    let before = e.sprite().clone();
    let r = e.apply(
        Command::AddLayer(AddLayer {
            name: "x".into(),
            above: None,
        }),
        Origin::Human,
    );
    assert!(matches!(r, Err(OpError::InvalidArgument(_))));
    assert_eq!(e.sprite(), &before);
}

// L12: failing structural commands change nothing.
#[test]
fn failing_add_and_remove_change_nothing() {
    let mut e = Engine::new(with_cel(0, 0, 1, 1));
    let before = e.sprite().clone();
    let r = e.apply(
        Command::AddLayer(AddLayer {
            name: "x".into(),
            above: Some(LayerId(42)),
        }),
        Origin::Human,
    );
    assert_eq!(r, Err(OpError::UnknownLayer(LayerId(42))));
    let r = e.apply(
        Command::AddLayer(AddLayer {
            name: String::new(),
            above: None,
        }),
        Origin::Human,
    );
    assert!(matches!(r, Err(OpError::InvalidArgument(_))));
    let r = e.apply(
        Command::RemoveLayer(RemoveLayer { layer: LayerId(42) }),
        Origin::Human,
    );
    assert_eq!(r, Err(OpError::UnknownLayer(LayerId(42))));
    assert_eq!(e.sprite(), &before);
    assert_eq!(e.revision(), 0);
    assert!(e.history().is_empty() && e.log().is_empty());
}
