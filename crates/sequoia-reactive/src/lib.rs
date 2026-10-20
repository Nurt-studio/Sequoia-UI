#![forbid(unsafe_code)]

mod effect;
mod environment;
mod history;
mod i18n;
mod memo;
mod runtime;
mod signal;
mod store;
mod undo_signal;
mod watch;

pub use effect::{Effect, EffectHandle, EffectScope, Subscription, effect};
pub use environment::{Environment, Listener};
pub use history::History;
pub use i18n::I18n;
pub use memo::{Memo, memo};
pub use runtime::{batch, current_transaction, flush_effects, transaction, untracked};
pub use signal::Signal;
pub use store::Store;
pub use undo_signal::{DEFAULT_COALESCE_WINDOW, UndoSignal};
pub use watch::{ReadSet, Watcher, capture_reads};

pub mod prelude {
    pub use crate::effect::{Effect, EffectHandle, EffectScope, Subscription, effect};
    pub use crate::environment::{Environment, Listener};
    pub use crate::history::History;
    pub use crate::i18n::I18n;
    pub use crate::memo::{Memo, memo};
    pub use crate::runtime::{batch, current_transaction, flush_effects, transaction, untracked};
    pub use crate::signal::Signal;
    pub use crate::store::Store;
    pub use crate::undo_signal::{DEFAULT_COALESCE_WINDOW, UndoSignal};
    pub use crate::watch::{ReadSet, Watcher, capture_reads};
}
