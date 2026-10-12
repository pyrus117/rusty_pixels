use rp_core::{Cel, Frame, Layer, LayerId, PaletteEntry, Sprite};

use crate::error::OpError;

/// A piece of sprite state a change reads or writes, for conflict detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    LayerOrder,
    LayerExists(LayerId),
    LayerProps(LayerId),
    Cel(LayerId, u32),
    FrameExists(u32),
    FrameProps(u32),
    PaletteLen,
    PaletteEntry(u32),
}

/// One undoable edit: the state before and after, plus how to push it either way.
/// Whole-cel snapshots are M0's granularity; a tile-level patch can join later behind this API.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Cel {
        layer: LayerId,
        frame: u32,
        before: Option<Cel>,
        after: Option<Cel>,
    },
    LayerInsert {
        index: usize,
        layer: Layer,
    },
    LayerRemove {
        index: usize,
        layers: Vec<Layer>,
        cels: Vec<((LayerId, u32), Cel)>,
    },
    LayerRename {
        id: LayerId,
        before: String,
        after: String,
    },
    FrameAppend {
        index: u32,
        frame: Frame,
    },
    FrameDuration {
        frame: u32,
        before: u16,
        after: u16,
    },
    PaletteColor {
        index: u32,
        before: [u8; 4],
        after: [u8; 4],
    },
    PaletteResize {
        before: Vec<PaletteEntry>,
        after: Vec<PaletteEntry>,
    },
}

fn set_cel(sprite: &mut Sprite, layer: LayerId, frame: u32, cel: &Option<Cel>) {
    match cel {
        Some(c) => {
            sprite.cels.insert((layer, frame), c.clone());
        }
        None => {
            sprite.cels.remove(&(layer, frame));
        }
    }
}

fn insert_layers(sprite: &mut Sprite, index: usize, layers: &[Layer]) -> Result<(), OpError> {
    if index > sprite.layers.len() {
        return Err(layer_index(index, sprite.layers.len()));
    }
    for (offset, layer) in layers.iter().enumerate() {
        sprite.layers.insert(index + offset, layer.clone());
    }
    Ok(())
}

fn layer_index(index: usize, len: usize) -> OpError {
    OpError::InvalidArgument(format!("no layer position {index} in {len} layers"))
}

fn set_duration(sprite: &mut Sprite, frame: u32, ms: u16) -> Result<(), OpError> {
    let entry = sprite
        .frames
        .get_mut(frame as usize)
        .ok_or(OpError::UnknownFrame(frame))?;
    entry.duration_ms = ms;
    Ok(())
}

fn set_rgba(sprite: &mut Sprite, index: u32, rgba: [u8; 4]) -> Result<(), OpError> {
    let len = sprite.palette.entries.len();
    let entry = sprite
        .palette
        .entries
        .get_mut(index as usize)
        .ok_or(OpError::PaletteIndex { index, len })?;
    entry.rgba = rgba;
    Ok(())
}

fn rename(sprite: &mut Sprite, id: LayerId, name: &str) -> Result<(), OpError> {
    let layer = sprite.layer_mut(id).ok_or(OpError::UnknownLayer(id))?;
    layer.name = name.to_string();
    Ok(())
}

impl Change {
    pub fn apply(&self, sprite: &mut Sprite) -> Result<(), OpError> {
        match self {
            Change::Cel {
                layer,
                frame,
                after,
                ..
            } => {
                set_cel(sprite, *layer, *frame, after);
                Ok(())
            }
            Change::LayerInsert { index, layer } => {
                insert_layers(sprite, *index, std::slice::from_ref(layer))
            }
            Change::LayerRemove {
                index,
                layers,
                cels,
            } => {
                let end = match index.checked_add(layers.len()) {
                    Some(end) if end <= sprite.layers.len() => end,
                    _ => return Err(layer_index(*index, sprite.layers.len())),
                };
                sprite.layers.drain(*index..end);
                for (key, _) in cels {
                    sprite.cels.remove(key);
                }
                Ok(())
            }
            Change::LayerRename { id, after, .. } => rename(sprite, *id, after),
            Change::FrameAppend { frame, .. } => {
                sprite.frames.push(*frame);
                Ok(())
            }
            Change::FrameDuration { frame, after, .. } => set_duration(sprite, *frame, *after),
            Change::PaletteColor { index, after, .. } => set_rgba(sprite, *index, *after),
            Change::PaletteResize { after, .. } => {
                sprite.palette.entries = after.clone();
                Ok(())
            }
        }
    }

    pub fn revert(&self, sprite: &mut Sprite) -> Result<(), OpError> {
        match self {
            Change::Cel {
                layer,
                frame,
                before,
                ..
            } => {
                set_cel(sprite, *layer, *frame, before);
                Ok(())
            }
            Change::LayerInsert { index, .. } => {
                if *index >= sprite.layers.len() {
                    return Err(layer_index(*index, sprite.layers.len()));
                }
                sprite.layers.remove(*index);
                Ok(())
            }
            Change::LayerRemove {
                index,
                layers,
                cels,
            } => {
                insert_layers(sprite, *index, layers)?;
                for (key, cel) in cels {
                    sprite.cels.insert(*key, cel.clone());
                }
                Ok(())
            }
            Change::LayerRename { id, before, .. } => rename(sprite, *id, before),
            Change::FrameAppend { .. } => match sprite.frames.pop() {
                Some(_) => Ok(()),
                None => Err(OpError::UnknownFrame(0)),
            },
            Change::FrameDuration { frame, before, .. } => set_duration(sprite, *frame, *before),
            Change::PaletteColor { index, before, .. } => set_rgba(sprite, *index, *before),
            Change::PaletteResize { before, .. } => {
                sprite.palette.entries = before.clone();
                Ok(())
            }
        }
    }

    /// What this change reads and what it writes (plan A1 rule 6).
    /// A conflict is any other applied change whose reads or writes hit a write here.
    pub fn keys(&self) -> (Vec<Key>, Vec<Key>) {
        match self {
            Change::Cel { layer, frame, .. } => (
                vec![Key::LayerExists(*layer), Key::FrameExists(*frame)],
                vec![Key::Cel(*layer, *frame)],
            ),
            Change::LayerInsert { layer, .. } => (
                vec![],
                vec![
                    Key::LayerOrder,
                    Key::LayerExists(layer.id),
                    Key::LayerProps(layer.id),
                ],
            ),
            Change::LayerRemove { layers, cels, .. } => {
                let mut writes = vec![Key::LayerOrder];
                for l in layers {
                    writes.push(Key::LayerExists(l.id));
                    writes.push(Key::LayerProps(l.id));
                }
                for ((layer, frame), _) in cels {
                    writes.push(Key::Cel(*layer, *frame));
                }
                (vec![], writes)
            }
            Change::LayerRename { id, .. } => {
                (vec![Key::LayerExists(*id)], vec![Key::LayerProps(*id)])
            }
            Change::FrameAppend { index, .. } => (vec![], vec![Key::FrameExists(*index)]),
            Change::FrameDuration { frame, .. } => (
                vec![Key::FrameExists(*frame)],
                vec![Key::FrameProps(*frame)],
            ),
            Change::PaletteColor { index, .. } => (vec![], vec![Key::PaletteEntry(*index)]),
            Change::PaletteResize { before, after } => {
                // Writes the length, and every entry the resize adds or drops.
                let lo = before.len().min(after.len()) as u32;
                let hi = before.len().max(after.len()) as u32;
                let mut writes = vec![Key::PaletteLen];
                writes.extend((lo..hi).map(Key::PaletteEntry));
                (vec![], writes)
            }
        }
    }
}
