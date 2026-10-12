use rp_core::{
    BlendMode, Cel, CelContent, Frame, HeaderFlags, Image, Layer, LayerFlags, LayerId, LayerKind,
    Sprite, UserData,
};
use uuid::Uuid;

use crate::change::Change;
use crate::command::{
    AddFrame, AddLayer, Command, PixelWrite, RemoveLayer, RenameLayer, SetFrameDuration,
    SetPaletteColor, SetPixels,
};
use crate::error::OpError;
use crate::registry::spec;

/// Memory cap for one grown cel: width * height * bytes_per_pixel must not
/// exceed it. Bounds a hostile or accidental allocation independently of the
/// canvas; 256 MiB leaves room for large but legitimate cels.
pub const MAX_CEL_BYTES: usize = 256 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Human,
    Agent { session: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub revision: u64,
    pub created_layer: Option<LayerId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub label: String,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub revision: u64,
    pub origin: Origin,
    pub command: Command,
}

struct UndoEntry {
    label: String,
    origin: Origin,
    changes: Vec<Change>,
}

struct Transaction {
    label: String,
    origin: Origin,
    changes: Vec<Change>,
}

pub struct Engine {
    sprite: Sprite,
    revision: u64,
    /// None once an existing layer holds the last id; ids are never reused.
    next_layer_id: Option<u32>,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    log: Vec<LogEntry>,
    txn: Option<Transaction>,
}

fn history_label(title: &str, origin: &Origin) -> String {
    match origin {
        Origin::Human => title.to_string(),
        Origin::Agent { .. } => format!("Agent: {title}"),
    }
}

impl Engine {
    /// next layer id = max existing id + 1 (0 if none), None when u32::MAX is taken;
    /// revision 0; empty history.
    pub fn new(sprite: Sprite) -> Engine {
        let next_layer_id = match sprite.layers.iter().map(|l| l.id.0).max() {
            Some(max) => max.checked_add(1),
            None => Some(0),
        };
        Engine {
            sprite,
            revision: 0,
            next_layer_id,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            log: Vec::new(),
            txn: None,
        }
    }

    pub fn sprite(&self) -> &Sprite {
        &self.sprite
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn apply(&mut self, cmd: Command, origin: Origin) -> Result<Outcome, OpError> {
        self.apply_inner(cmd, origin)
    }

    /// Like apply, but fails with StaleRevision unless revision() == base_revision.
    pub fn apply_at(
        &mut self,
        cmd: Command,
        origin: Origin,
        base_revision: u64,
    ) -> Result<Outcome, OpError> {
        if self.revision != base_revision {
            return Err(OpError::StaleRevision {
                expected: base_revision,
                actual: self.revision,
            });
        }
        self.apply_inner(cmd, origin)
    }

    pub fn begin(&mut self, label: &str, origin: Origin) -> Result<(), OpError> {
        if self.txn.is_some() {
            return Err(OpError::TransactionOpen);
        }
        self.txn = Some(Transaction {
            label: label.to_string(),
            origin,
            changes: Vec::new(),
        });
        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), OpError> {
        let txn = self.txn.take().ok_or(OpError::NoTransaction)?;
        if !txn.changes.is_empty() {
            let label = history_label(&txn.label, &txn.origin);
            self.push_history(UndoEntry {
                label,
                origin: txn.origin,
                changes: txn.changes,
            });
        }
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), OpError> {
        let txn = self.txn.take().ok_or(OpError::NoTransaction)?;
        if txn.changes.is_empty() {
            return Ok(());
        }
        for change in txn.changes.iter().rev() {
            change.revert(&mut self.sprite)?;
        }
        self.revision += 1;
        Ok(())
    }

    /// Ok(None) when there is nothing to undo. TransactionOpen while a transaction is open.
    pub fn undo(&mut self) -> Result<Option<String>, OpError> {
        if self.txn.is_some() {
            return Err(OpError::TransactionOpen);
        }
        let Some(entry) = self.undo_stack.pop() else {
            return Ok(None);
        };
        for change in entry.changes.iter().rev() {
            change.revert(&mut self.sprite)?;
        }
        self.revision += 1;
        let label = entry.label.clone();
        self.redo_stack.push(entry);
        Ok(Some(label))
    }

    pub fn redo(&mut self) -> Result<Option<String>, OpError> {
        if self.txn.is_some() {
            return Err(OpError::TransactionOpen);
        }
        let Some(entry) = self.redo_stack.pop() else {
            return Ok(None);
        };
        for change in &entry.changes {
            change.apply(&mut self.sprite)?;
        }
        self.revision += 1;
        let label = entry.label.clone();
        self.undo_stack.push(entry);
        Ok(Some(label))
    }

    /// Undo stack, oldest first.
    pub fn history(&self) -> Vec<HistoryEntry> {
        self.undo_stack
            .iter()
            .map(|e| HistoryEntry {
                label: e.label.clone(),
                origin: e.origin.clone(),
            })
            .collect()
    }

    /// Every successfully applied command, in order (audit/replay record; not used for undo).
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }

    fn push_history(&mut self, entry: UndoEntry) {
        self.redo_stack.clear();
        self.undo_stack.push(entry);
    }

    fn apply_inner(&mut self, cmd: Command, origin: Origin) -> Result<Outcome, OpError> {
        let (change, created_layer) = self.build(&cmd)?;
        change.apply(&mut self.sprite)?;
        self.revision += 1;
        if created_layer.is_some() {
            self.next_layer_id = self.next_layer_id.and_then(|id| id.checked_add(1));
        }
        let label = history_label(spec(cmd.kind()).title, &origin);
        self.log.push(LogEntry {
            revision: self.revision,
            origin: origin.clone(),
            command: cmd,
        });
        match &mut self.txn {
            Some(txn) => txn.changes.push(change),
            None => self.push_history(UndoEntry {
                label,
                origin,
                changes: vec![change],
            }),
        }
        Ok(Outcome {
            revision: self.revision,
            created_layer,
        })
    }

    /// Validate `cmd` and record its change without touching the sprite.
    fn build(&self, cmd: &Command) -> Result<(Change, Option<LayerId>), OpError> {
        match cmd {
            Command::SetPixels(c) => Ok((self.build_set_pixels(c)?, None)),
            Command::AddLayer(c) => {
                let (change, id) = self.build_add_layer(c)?;
                Ok((change, Some(id)))
            }
            Command::RemoveLayer(c) => Ok((self.build_remove_layer(c)?, None)),
            Command::RenameLayer(c) => Ok((self.build_rename_layer(c)?, None)),
            Command::AddFrame(c) => Ok((self.build_add_frame(c)?, None)),
            Command::SetFrameDuration(c) => Ok((self.build_set_frame_duration(c)?, None)),
            Command::SetPaletteColor(c) => Ok((self.build_set_palette_color(c)?, None)),
        }
    }

    fn build_set_pixels(&self, cmd: &SetPixels) -> Result<Change, OpError> {
        if cmd.pixels.is_empty() {
            return Err(OpError::InvalidArgument(
                "set_pixels needs at least one pixel".to_string(),
            ));
        }
        let layer = self
            .sprite
            .layer(cmd.layer)
            .ok_or(OpError::UnknownLayer(cmd.layer))?;
        if layer.kind != LayerKind::Image {
            return Err(OpError::NotAnImageLayer(cmd.layer));
        }
        if cmd.frame as usize >= self.sprite.frames.len() {
            return Err(OpError::UnknownFrame(cmd.frame));
        }
        for p in &cmd.pixels {
            if p.x < 0
                || p.y < 0
                || p.x >= i32::from(self.sprite.width)
                || p.y >= i32::from(self.sprite.height)
            {
                return Err(OpError::OutOfCanvas {
                    x: p.x,
                    y: p.y,
                    width: self.sprite.width,
                    height: self.sprite.height,
                });
            }
            if p.value.mode() != self.sprite.color_mode {
                return Err(OpError::ModeMismatch {
                    sprite: self.sprite.color_mode,
                    pixel: p.value.mode(),
                });
            }
        }

        // resolve_cel follows a link, so a write lands on the source frame's cel.
        let (target_frame, before) = match self.sprite.resolve_cel(cmd.layer, cmd.frame) {
            Some((frame, cel)) => (frame, Some(cel.clone())),
            None => (cmd.frame, self.sprite.cel(cmd.layer, cmd.frame).cloned()),
        };
        let old = match &before {
            None => None,
            Some(cel) => match &cel.content {
                CelContent::Image(img) => Some((i64::from(cel.x), i64::from(cel.y), img)),
                CelContent::Linked { .. } | CelContent::Tilemap(_) => {
                    return Err(OpError::InvalidArgument(
                        "resolved cel content is not an image".to_string(),
                    ));
                }
            },
        };

        // Bounding-box union of the old cel and the written points, in i64 so
        // hostile cel positions cannot overflow the corners.
        let mut min_x = i64::MAX;
        let mut min_y = i64::MAX;
        let mut max_x = i64::MIN;
        let mut max_y = i64::MIN;
        for p in &cmd.pixels {
            min_x = min_x.min(i64::from(p.x));
            min_y = min_y.min(i64::from(p.y));
            max_x = max_x.max(i64::from(p.x));
            max_y = max_y.max(i64::from(p.y));
        }
        if let Some((old_x, old_y, old_img)) = &old {
            // A zero-area image has no bounds to honour, so it contributes nothing.
            if old_img.width() > 0 && old_img.height() > 0 {
                min_x = min_x.min(*old_x);
                min_y = min_y.min(*old_y);
                max_x = max_x.max(*old_x + i64::from(old_img.width()) - 1);
                max_y = max_y.max(*old_y + i64::from(old_img.height()) - 1);
            }
        }
        let union_w = max_x - min_x + 1;
        let union_h = max_y - min_y + 1;

        // Growth is bounded by memory (MAX_CEL_BYTES) and the file format, not
        // by area: a far off-screen cel cannot balloon an allocation, while
        // painting next to an overhanging cel keeps working. Checked i64
        // because hostile cel positions make the corners extreme.
        let bpp = self.sprite.color_mode.bytes_per_pixel();
        let bytes = union_w
            .checked_mul(union_h)
            .and_then(|px| i64::try_from(bpp).ok().and_then(|b| px.checked_mul(b)));
        match bytes {
            Some(total) if total <= MAX_CEL_BYTES as i64 => {}
            _ => {
                return Err(OpError::InvalidArgument(format!(
                    "grown cel would exceed the MAX_CEL_BYTES memory limit ({MAX_CEL_BYTES})"
                )));
            }
        }
        if union_w > i64::from(u16::MAX) {
            return Err(OpError::InvalidArgument(
                "cel width would exceed the file format limit of 65535".to_string(),
            ));
        }
        if union_h > i64::from(u16::MAX) {
            return Err(OpError::InvalidArgument(
                "cel height would exceed the file format limit of 65535".to_string(),
            ));
        }
        if min_x < i64::from(i16::MIN) || min_x > i64::from(i16::MAX) {
            return Err(OpError::InvalidArgument(
                "cel x would exceed the file format i16 limit".to_string(),
            ));
        }
        if min_y < i64::from(i16::MIN) || min_y > i64::from(i16::MAX) {
            return Err(OpError::InvalidArgument(
                "cel y would exceed the file format i16 limit".to_string(),
            ));
        }
        let width = union_w as u32;
        let height = union_h as u32;

        let image = self.grown_image(old, min_x, min_y, width, height, &cmd.pixels)?;
        // The i16 checks above leave no room for a failed cast here.
        let (x, y) = (min_x as i32, min_y as i32);
        let after = match &before {
            Some(cel) => Cel {
                x,
                y,
                content: CelContent::Image(image),
                ..cel.clone()
            },
            None => Cel {
                x,
                y,
                opacity: 255,
                z_index: 0,
                content: CelContent::Image(image),
                extra: None,
                user_data: UserData::default(),
            },
        };
        Ok(Change::Cel {
            layer: cmd.layer,
            frame: target_frame,
            before,
            after: Some(after),
        })
    }

    /// The old image copied into a zeroed bbox-sized image, then the new points written.
    fn grown_image(
        &self,
        old: Option<(i64, i64, &Image)>,
        min_x: i64,
        min_y: i64,
        width: u32,
        height: u32,
        pixels: &[PixelWrite],
    ) -> Result<Image, OpError> {
        let mode = self.sprite.color_mode;
        let bpp = mode.bytes_per_pixel();
        let total = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(bpp))
            .ok_or_else(|| OpError::InvalidArgument("cel is too large".to_string()))?;
        let mut buf = vec![0u8; total];

        if let Some((old_x, old_y, old_img)) = &old {
            // A zero-area image has no pixels, and its (x, y) is meaningless.
            if old_img.width() > 0 && old_img.height() > 0 {
                let row_bytes = old_img.width() as usize * bpp;
                for row in 0..old_img.height() as usize {
                    let src_start = row * row_bytes;
                    let src = old_img
                        .data()
                        .get(src_start..src_start + row_bytes)
                        .ok_or_else(|| {
                            OpError::InvalidArgument("old image is truncated".to_string())
                        })?;
                    let dst_start = ((old_y - min_y) as usize + row) * (width as usize * bpp)
                        + (old_x - min_x) as usize * bpp;
                    let dst = buf
                        .get_mut(dst_start..dst_start + row_bytes)
                        .ok_or_else(|| {
                            OpError::InvalidArgument("cel copy out of range".to_string())
                        })?;
                    dst.copy_from_slice(src);
                }
            }
        }

        let mut image = Image::from_raw(width, height, mode, buf)
            .map_err(|e| OpError::InvalidArgument(format!("built cel image: {e}")))?;
        for p in pixels {
            image
                .put(
                    (i64::from(p.x) - min_x) as u32,
                    (i64::from(p.y) - min_y) as u32,
                    p.value,
                )
                .map_err(|e| OpError::InvalidArgument(format!("pixel write: {e}")))?;
        }
        Ok(image)
    }

    fn build_add_layer(&self, cmd: &AddLayer) -> Result<(Change, LayerId), OpError> {
        if cmd.name.is_empty() {
            return Err(OpError::InvalidArgument("layer name is empty".to_string()));
        }
        let (index, parent) = match cmd.above {
            Some(above) => {
                let range = self
                    .sprite
                    .subtree_range(above)
                    .ok_or(OpError::UnknownLayer(above))?;
                let parent = self.sprite.layer(above).and_then(|l| l.parent);
                (range.end, parent)
            }
            None => (self.sprite.layers.len(), None),
        };
        let id = LayerId(
            self.next_layer_id
                .ok_or_else(|| OpError::InvalidArgument("no layer ids left".to_string()))?,
        );
        let layer = Layer {
            id,
            parent,
            name: cmd.name.clone(),
            kind: LayerKind::Image,
            flags: LayerFlags(LayerFlags::VISIBLE | LayerFlags::EDITABLE),
            blend_mode: BlendMode::Normal,
            opacity: 255,
            uuid: self
                .sprite
                .flags
                .contains(HeaderFlags::LAYER_UUIDS)
                .then(|| Uuid::new_v4().into_bytes()),
            user_data: UserData::default(),
        };
        Ok((Change::LayerInsert { index, layer }, id))
    }

    fn build_remove_layer(&self, cmd: &RemoveLayer) -> Result<Change, OpError> {
        let range = self
            .sprite
            .subtree_range(cmd.layer)
            .ok_or(OpError::UnknownLayer(cmd.layer))?;
        let layers = self
            .sprite
            .layers
            .get(range.clone())
            .ok_or(OpError::UnknownLayer(cmd.layer))?
            .to_vec();
        let cels = self
            .sprite
            .cels
            .iter()
            .filter(|((id, _), _)| layers.iter().any(|l| l.id == *id))
            .map(|(key, cel)| (*key, cel.clone()))
            .collect();
        Ok(Change::LayerRemove {
            index: range.start,
            layers,
            cels,
        })
    }

    fn build_rename_layer(&self, cmd: &RenameLayer) -> Result<Change, OpError> {
        if cmd.name.is_empty() {
            return Err(OpError::InvalidArgument("layer name is empty".to_string()));
        }
        let before = self
            .sprite
            .layer(cmd.layer)
            .ok_or(OpError::UnknownLayer(cmd.layer))?
            .name
            .clone();
        Ok(Change::LayerRename {
            id: cmd.layer,
            before,
            after: cmd.name.clone(),
        })
    }

    fn build_add_frame(&self, cmd: &AddFrame) -> Result<Change, OpError> {
        if cmd.duration_ms == 0 {
            return Err(OpError::InvalidArgument(
                "frame duration must be non-zero".to_string(),
            ));
        }
        Ok(Change::FrameAppend {
            frame: Frame {
                duration_ms: cmd.duration_ms,
            },
        })
    }

    fn build_set_frame_duration(&self, cmd: &SetFrameDuration) -> Result<Change, OpError> {
        if cmd.duration_ms == 0 {
            return Err(OpError::InvalidArgument(
                "frame duration must be non-zero".to_string(),
            ));
        }
        let before = self
            .sprite
            .frames
            .get(cmd.frame as usize)
            .ok_or(OpError::UnknownFrame(cmd.frame))?
            .duration_ms;
        Ok(Change::FrameDuration {
            frame: cmd.frame,
            before,
            after: cmd.duration_ms,
        })
    }

    fn build_set_palette_color(&self, cmd: &SetPaletteColor) -> Result<Change, OpError> {
        let len = self.sprite.palette.entries.len();
        let before = self
            .sprite
            .palette
            .entries
            .get(cmd.index as usize)
            .ok_or(OpError::PaletteIndex {
                index: cmd.index,
                len,
            })?
            .rgba;
        Ok(Change::PaletteColor {
            index: cmd.index,
            before,
            after: cmd.rgba,
        })
    }
}
