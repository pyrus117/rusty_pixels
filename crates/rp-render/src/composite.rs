//! Layer tree flattening: buffers, cels, tilemaps, clipping.

use rp_core::{
    BlendMode, Cel, CelContent, ColorMode, HeaderFlags, Image, Layer, LayerFlags, LayerId,
    LayerKind, RgbaImage, Sprite, Tilemap,
};

use crate::blend::{m_round, over};
use crate::{LayerFilter, RenderError, RenderOptions};

struct Ctx<'a> {
    sprite: &'a Sprite,
    frame: u32,
    opts: &'a RenderOptions,
    children: &'a [Vec<usize>],
}

pub(crate) fn render(
    sprite: &Sprite,
    frame: u32,
    opts: &RenderOptions,
) -> Result<RgbaImage, RenderError> {
    if sprite.frames.get(frame as usize).is_none() {
        return Err(RenderError::UnknownFrame(frame));
    }
    if let LayerFilter::Only(ids) = &opts.layers {
        for id in ids {
            if sprite.layer(*id).is_none() {
                return Err(RenderError::UnknownLayer(*id));
            }
        }
    }
    let children = children_of(sprite);
    let mut out = RgbaImage::new(u32::from(sprite.width), u32::from(sprite.height))?;
    for (i, layer) in sprite.layers.iter().enumerate() {
        if layer.parent.is_none() {
            let ctx = Ctx {
                sprite,
                frame,
                opts,
                children: &children,
            };
            node(&ctx, &mut out, i, true, false)?;
        }
    }
    Ok(out)
}

/// Draws one layer into `out`. `vis` is true when every ancestor is visible;
/// `only` is true when an ancestor is in an `Only` selection.
fn node(
    ctx: &Ctx,
    out: &mut RgbaImage,
    idx: usize,
    vis: bool,
    only: bool,
) -> Result<(), RenderError> {
    let sprite = ctx.sprite;
    let Some(layer) = sprite.layers.get(idx) else {
        return Ok(());
    };
    match layer.kind {
        LayerKind::Group => {
            let vis = vis && layer.flags.contains(LayerFlags::VISIBLE);
            let only = only || selected_id(ctx.opts, layer.id);
            // Without GROUP_BLEND_VALID a group does not isolate: its children
            // blend onto the backdrop below as if the group were not there.
            let (mode, opacity) = if sprite.flags.contains(HeaderFlags::GROUP_BLEND_VALID) {
                (layer.blend_mode, layer.opacity)
            } else {
                if let Some(kids) = ctx.children.get(idx) {
                    for &c in kids {
                        node(ctx, out, c, vis, only)?;
                    }
                }
                return Ok(());
            };
            let mut sub = RgbaImage::new(u32::from(sprite.width), u32::from(sprite.height))?;
            if let Some(kids) = ctx.children.get(idx) {
                for &c in kids {
                    node(ctx, &mut sub, c, vis, only)?;
                }
            }
            blit(
                out,
                sub.data(),
                sub.width(),
                sub.height(),
                0,
                0,
                mode,
                i32::from(opacity),
            )
        }
        LayerKind::Image | LayerKind::Tilemap { .. } => {
            let draw = match &ctx.opts.layers {
                LayerFilter::All => true,
                LayerFilter::Visible => vis && layer.flags.contains(LayerFlags::VISIBLE),
                LayerFilter::Only(ids) => only || ids.contains(&layer.id),
            };
            if draw {
                cel(ctx, out, layer)?;
            }
            Ok(())
        }
    }
}

fn selected_id(opts: &RenderOptions, id: LayerId) -> bool {
    match &opts.layers {
        LayerFilter::Only(ids) => ids.contains(&id),
        _ => false,
    }
}

fn cel(ctx: &Ctx, out: &mut RgbaImage, layer: &Layer) -> Result<(), RenderError> {
    let sprite = ctx.sprite;
    let Some(link) = sprite.cel(layer.id, ctx.frame) else {
        return Ok(());
    };
    // A linked cel draws its target's content at its own position and opacity.
    // A link to a link has no content of its own, so it draws nothing.
    let content = match &link.content {
        CelContent::Linked { .. } => match sprite.resolve_cel(layer.id, ctx.frame) {
            Some((_, resolved)) => match &resolved.content {
                CelContent::Linked { .. } => return Ok(()),
                c => c,
            },
            None => return Ok(()),
        },
        c => c,
    };
    let layer_op = if sprite.flags.contains(HeaderFlags::LAYER_OPACITY_VALID) {
        layer.opacity
    } else {
        255
    };
    let outer = m_round(i32::from(link.opacity), i32::from(layer_op));
    match content {
        CelContent::Image(img) => {
            let rgba = rgba_of(sprite, img).map_err(|reason| bad(ctx, layer, reason))?;
            blit(
                out,
                &rgba,
                img.width(),
                img.height(),
                i64::from(link.x),
                i64::from(link.y),
                layer.blend_mode,
                outer,
            )
        }
        CelContent::Tilemap(tm) => {
            let LayerKind::Tilemap { tileset_index } = layer.kind else {
                return Err(bad(ctx, layer, "tilemap cel on a non-tilemap layer".into()));
            };
            tilemap(ctx, out, layer, link, tm, tileset_index, outer)
        }
        CelContent::Linked { .. } => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
fn tilemap(
    ctx: &Ctx,
    out: &mut RgbaImage,
    layer: &Layer,
    cel: &Cel,
    tm: &Tilemap,
    tileset_index: u32,
    outer: i32,
) -> Result<(), RenderError> {
    let sprite = ctx.sprite;
    let ts = sprite.tilesets.get(tileset_index as usize).ok_or_else(|| {
        bad(
            ctx,
            layer,
            format!("tileset {tileset_index} does not exist"),
        )
    })?;
    let cells = (tm.width as usize)
        .checked_mul(tm.height as usize)
        .filter(|&n| n <= tm.tiles.len())
        .ok_or_else(|| bad(ctx, layer, "tilemap grid does not fit its tiles".into()))?;
    for (k, &entry) in tm.tiles.iter().take(cells).enumerate() {
        let id = entry & tm.tile_id_mask;
        if id == 0 {
            continue;
        }
        let tile = ts
            .tiles
            .get(id as usize)
            .ok_or_else(|| bad(ctx, layer, format!("tile {id} is past the tileset")))?;
        let rgba = rgba_of(sprite, tile).map_err(|reason| bad(ctx, layer, reason))?;
        let rgba = flip(
            rgba,
            tile.width(),
            tile.height(),
            entry & tm.x_flip_mask != 0,
            entry & tm.y_flip_mask != 0,
            entry & tm.diagonal_flip_mask != 0,
        );
        let x0 = i64::from(cel.x) + (k % tm.width as usize) as i64 * i64::from(ts.tile_width);
        let y0 = i64::from(cel.y) + (k / tm.width as usize) as i64 * i64::from(ts.tile_height);
        blit(
            out,
            &rgba,
            tile.width(),
            tile.height(),
            x0,
            y0,
            layer.blend_mode,
            outer,
        )?;
    }
    Ok(())
}

/// Mirrors a tile inside its cell: vertical, then horizontal, then diagonal.
/// A diagonal flip on a non-square tile samples outside itself and gets clear.
fn flip(rgba: Vec<u8>, w: u32, h: u32, x: bool, y: bool, d: bool) -> Vec<u8> {
    if !x && !y && !d {
        return rgba;
    }
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; rgba.len()];
    for dv in 0..h {
        for du in 0..w {
            let (mut su, mut sv) = (du, dv);
            if y {
                sv = h - 1 - sv;
            }
            if x {
                su = w - 1 - su;
            }
            if d {
                std::mem::swap(&mut su, &mut sv);
            }
            if su >= w || sv >= h {
                continue;
            }
            let (s, o) = ((sv * w + su) * 4, (dv * w + du) * 4);
            if let (Some(s), Some(o)) = (rgba.get(s..s + 4), out.get_mut(o..o + 4)) {
                o.copy_from_slice(s);
            }
        }
    }
    out
}

/// Draws straight-RGBA pixels at (x0, y0), clipped to the canvas.
#[allow(clippy::too_many_arguments)]
fn blit(
    out: &mut RgbaImage,
    rgba: &[u8],
    w: u32,
    h: u32,
    x0: i64,
    y0: i64,
    mode: BlendMode,
    outer: i32,
) -> Result<(), RenderError> {
    let (ow, oh) = (i64::from(out.width()), i64::from(out.height()));
    for (i, px) in rgba
        .chunks_exact(4)
        .take(w as usize * h as usize)
        .enumerate()
    {
        let [r, g, b, a] = px else {
            continue;
        };
        let alpha = m_round(i32::from(*a), outer);
        if alpha == 0 {
            continue;
        }
        let cx = x0 + (i % w as usize) as i64;
        let cy = y0 + (i / w as usize) as i64;
        if cx < 0 || cy < 0 || cx >= ow || cy >= oh {
            continue;
        }
        let dst = match out.get(cx as u32, cy as u32) {
            Some(p) => p,
            None => continue,
        };
        let e = over(mode, [*r, *g, *b, *a], dst, alpha as u8);
        out.put(cx as u32, cy as u32, e)?;
    }
    Ok(())
}

/// Any pixel data, expanded to straight RGBA8. Wrong colour modes and short
/// palettes are transparent or an error, never a panic.
fn rgba_of(sprite: &Sprite, img: &Image) -> Result<Vec<u8>, String> {
    if img.mode() != sprite.color_mode {
        return Err(format!(
            "cel image is {:?}, sprite is {:?}",
            img.mode(),
            sprite.color_mode
        ));
    }
    let data = img.data();
    Ok(match sprite.color_mode {
        ColorMode::Rgba => data.to_vec(),
        ColorMode::Grayscale => {
            let mut out = Vec::with_capacity(data.len() * 2);
            for px in data.chunks_exact(2) {
                out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
            out
        }
        ColorMode::Indexed => {
            let mut out = Vec::with_capacity(data.len() * 4);
            for &index in data {
                out.extend_from_slice(&palette_rgba(sprite, index));
            }
            out
        }
    })
}

fn palette_rgba(sprite: &Sprite, index: u8) -> [u8; 4] {
    if index == sprite.transparent_index {
        return [0, 0, 0, 0];
    }
    match sprite.palette.entries.get(index as usize) {
        Some(e) => e.rgba,
        None => [0, 0, 0, 0],
    }
}

fn bad(ctx: &Ctx, layer: &Layer, reason: String) -> RenderError {
    RenderError::BadCel {
        layer: layer.id,
        frame: ctx.frame,
        reason,
    }
}

/// Parent links only matter through groups; a corrupt parent elsewhere just
/// orphans the child.
fn children_of(sprite: &Sprite) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); sprite.layers.len()];
    for (i, layer) in sprite.layers.iter().enumerate() {
        if let Some(parent) = layer.parent
            && let Some(p) = sprite
                .layer_index(parent)
                .filter(|&p| matches!(sprite.layers[p].kind, LayerKind::Group))
        {
            children[p].push(i);
        }
    }
    children
}
