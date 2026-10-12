//! Document operations for Rusty Pixels.

mod change;
mod command;
mod engine;
mod error;
mod registry;

pub use change::Change;
pub use command::{
    AddFrame, AddLayer, Command, CommandKind, PixelWrite, RemoveLayer, RenameLayer,
    SetFrameDuration, SetPaletteColor, SetPixels,
};
pub use engine::{Engine, HistoryEntry, LogEntry, MAX_CEL_BYTES, Origin, Outcome};
pub use error::OpError;
pub use registry::{OpSpec, params_schema, registry, spec};
