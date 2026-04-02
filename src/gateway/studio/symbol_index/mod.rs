//! Background local-project symbol index coordinator for Studio.

mod state;
mod types;

pub(crate) use state::{SymbolIndexCoordinator, timestamp_now};
pub(crate) use types::{SymbolIndexPhase, SymbolIndexStatus};
