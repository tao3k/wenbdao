mod resolution;
mod retry;

pub(crate) use resolution::resolve_managed_checkout;

#[cfg(test)]
pub(crate) use retry::{current_remote_url, retryable_git_open_error_message};
