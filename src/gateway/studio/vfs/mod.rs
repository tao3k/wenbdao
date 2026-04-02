//! Virtual File System (VFS) orchestration for Studio API.

mod categories;
mod content;
mod filters;
mod flight;
mod navigation;
mod roots;
mod scan;

#[cfg(test)]
mod tests;

pub(crate) use content::{get_entry, read_content, resolve_vfs_file_path};
pub(crate) use flight::StudioVfsResolveFlightRouteProvider;
pub(crate) use navigation::resolve_navigation_target;
pub(crate) use roots::list_root_entries;
pub(crate) use scan::scan_roots;
