mod collect;
mod coordinator;
mod filters;
mod fingerprint;
mod language;
mod task;

pub(crate) use coordinator::RepoIndexCoordinator;

#[cfg(test)]
mod tests;
