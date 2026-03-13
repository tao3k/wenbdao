//! Skill VFS resolver core implementation and URI resolution.

/// Skill VFS resolver core implementation.
pub mod core;
/// Embedded mount helpers for semantic resources.
mod mount;
/// Cached UTF-8 read helpers.
mod read;
/// URI resolution logic for skill VFS.
pub mod resolve_uri;
/// Runtime discovery helpers for skill roots.
mod runtime;
