#![forbid(unsafe_code)]

pub mod drop;
pub mod model;
pub mod ops;
pub mod persist;

pub use drop::{
    DockDrop, EDGE_FRACTION, drop_from_fraction, drop_from_fraction_or_float, preview_rect,
};
pub use model::{DockModel, DockNode, FloatingPanel, PanelId, SplitAxis};
pub use ops::DockOp;
pub use persist::{DecodeError, decode, encode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PanelPayload(pub PanelId);

impl PanelPayload {
    pub const fn panel(self) -> PanelId {
        self.0
    }
}
