//! Route inventory shared by the Wendao gateway runtime and `OpenAPI` contract tests.

mod docs;
mod graph;
mod repo;
mod search;
mod shared;
mod ui;
mod vfs;

pub use self::{docs::*, graph::*, repo::*, search::*, shared::*, ui::*, vfs::*};
