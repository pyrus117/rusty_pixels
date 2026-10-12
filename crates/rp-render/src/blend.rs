//! Per-pixel blend maths. The integer forms and roundings are fitted to the
//! oracle fixtures; they are not a port of any editor's source.

use rp_core::BlendMode;

/// `(a * b) / 255` rounded to nearest, the scale Aseprite uses throughout.
pub(crate) fn m_round(a: i32, b: i32) -> i32 {
    (a * b + 127) / 255
}

/// Moves `x` toward `y` by `k/255`. Negative deltas floor instead of truncating
/// toward zero, so they need `div_euclid`.
fn mix(x: i32, y: i32, k: i32) -> i32 {
    let d = (y - x) * k;
    if d >= 0 {
        x + (d + 127) / 255
    } else {
        x + (d + 128).div_euclid(255)
    }
}

/// Composites one straight-RGBA source pixel over one backdrop pixel.
/// `alpha` is the source's effective alpha: pixel alpha x cel opacity x
/// layer opacity, each step rounded with `m_round`.
pub(crate) fn over(mode: BlendMode, s: [u8; 4], b: [u8; 4], alpha: u8) -> [u8; 4] {
    let a = i32::from(alpha);
    if a == 0 {
        return b;
    }
    let ab = i32::from(b[3]);
    let s = [i32::from(s[0]), i32::from(s[1]), i32::from(s[2])];
    let b = [i32::from(b[0]), i32::from(b[1]), i32::from(b[2])];
    let ao = a + m_round(ab, 255 - a); // in 1..=255, so divisions below stay nonzero
    let blended = blend_rgb(mode, s, b);
    let mut out = [0u8; 4];
    for i in 0..3 {
        // N fades a colour toward the backdrop by the source opacity alone;
        // the two mixes then fade the plain source toward N(blend), matching
        // the oracle's stepwise rounding.
        let n = |y: i32| b[i] + ((y - b[i]) * a) / ao;
        let nb = n(blended[i]);
        let x1 = mix(n(s[i]), nb, ab);
        let x2 = mix(x1, nb, m_round(a, ab));
        out[i] = x2.clamp(0, 255) as u8;
    }
    out[3] = ao as u8;
    out
}

fn blend_rgb(mode: BlendMode, s: [i32; 3], b: [i32; 3]) -> [i32; 3] {
    match mode {
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => {
            let sf = f3(s);
            let bf = f3(b);
            let out = match mode {
                BlendMode::Hue => setlum(setsat(sf, sat(bf)), lum(bf)),
                BlendMode::Saturation => setlum(setsat(bf, sat(sf)), lum(bf)),
                BlendMode::Color => setlum(sf, lum(bf)),
                // Luminosity; the arm above already caught the other modes.
                _ => setlum(bf, lum(sf)),
            };
            // The oracle truncates these float results toward zero.
            let mut q = [0; 3];
            for (i, v) in out.iter().enumerate() {
                q[i] = (*v as i32).clamp(0, 255);
            }
            q
        }
        _ => {
            let mut out = [0; 3];
            for (i, &cs) in s.iter().enumerate() {
                out[i] = separable(mode, cs, b[i]);
            }
            out
        }
    }
}

fn separable(mode: BlendMode, cs: i32, cb: i32) -> i32 {
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => m_round(cs, cb),
        BlendMode::Screen => 255 - m_round(255 - cs, 255 - cb),
        BlendMode::Overlay => {
            if cb <= 127 {
                m_round(cs, 2 * cb)
            } else {
                255 - m_round(255 - cs, 510 - 2 * cb)
            }
        }
        BlendMode::Darken => cs.min(cb),
        BlendMode::Lighten => cs.max(cb),
        BlendMode::ColorDodge => {
            if cs == 255 {
                255
            } else {
                ((cb * 255 + (255 - cs) / 2) / (255 - cs)).min(255)
            }
        }
        BlendMode::ColorBurn => {
            if cs == 0 {
                0
            } else {
                (255 - ((255 - cb) * 255 + cs / 2) / cs).max(0)
            }
        }
        BlendMode::HardLight => {
            if cs <= 127 {
                m_round(cb, 2 * cs)
            } else {
                255 - m_round(255 - cb, 510 - 2 * cs)
            }
        }
        BlendMode::SoftLight => soft_light(cs, cb),
        BlendMode::Difference => (cs - cb).abs(),
        BlendMode::Exclusion => cs + cb - 2 * m_round(cs, cb),
        BlendMode::Addition => (cs + cb).min(255),
        BlendMode::Subtract => (cb - cs).max(0),
        BlendMode::Divide => {
            if cs == 0 {
                255
            } else {
                ((cb * 255 + cs / 2) / cs).min(255)
            }
        }
        // The nonseparable modes never reach this function.
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => cs,
    }
}

/// W3C soft light, float, rounded to nearest.
fn soft_light(cs: i32, cb: i32) -> i32 {
    let cs = f64::from(cs) / 255.0;
    let cb = f64::from(cb) / 255.0;
    let v = if cs <= 0.5 {
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
    } else {
        // The piecewise stand-in for sqrt that the spec prescribes below 0.25.
        let d = if cb > 0.25 {
            cb.sqrt()
        } else {
            ((16.0 * cb - 12.0) * cb + 4.0) * cb
        };
        cb + (2.0 * cs - 1.0) * (d - cb)
    };
    (v.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as i32
}

fn f3(c: [i32; 3]) -> [f64; 3] {
    [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])]
}

fn lum(c: [f64; 3]) -> f64 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn sat(c: [f64; 3]) -> f64 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn setlum(c: [f64; 3], l: f64) -> [f64; 3] {
    let d = l - lum(c);
    clip([c[0] + d, c[1] + d, c[2] + d])
}

/// Sets the min channel to 0 and the max to `s`, scaling the middle one.
/// Equal channels keep their order; `sort_by` is stable.
fn setsat(c: [f64; 3], s: f64) -> [f64; 3] {
    let mut order = [0usize, 1, 2];
    order.sort_by(|&x, &y| c[x].total_cmp(&c[y]));
    let (mn, mid, mx) = (c[order[0]], c[order[1]], c[order[2]]);
    let mut out = [0.0; 3];
    if mx > mn {
        out[order[0]] = 0.0;
        out[order[2]] = s;
        out[order[1]] = (mid - mn) * s / (mx - mn);
    }
    out
}

/// Scales a colour back into 0..255 while keeping its luminosity.
fn clip(mut c: [f64; 3]) -> [f64; 3] {
    let mut l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let mut x = c[0].max(c[1]).max(c[2]);
    if n < 0.0 {
        let d = l - n;
        for v in &mut c {
            *v = if d == 0.0 { l } else { l + (*v - l) * l / d };
        }
        l = lum(c);
        x = c[0].max(c[1]).max(c[2]);
    }
    if x > 255.0 {
        let d = x - l;
        for v in &mut c {
            *v = if d == 0.0 {
                l
            } else {
                l + (*v - l) * (255.0 - l) / d
            };
        }
    }
    c
}
