#[test]
fn managed_checkout_git_open_retry_only_retries_descriptor_pressure_messages() {
    assert!(
        crate::git::checkout::managed::retryable_git_open_error_message(
            "could not open '/tmp/example.git/config': Too many open files; class=Os (2)"
        )
    );
    assert!(
        !crate::git::checkout::managed::retryable_git_open_error_message(
            "could not open '/tmp/example.git/config': No such file or directory"
        )
    );
}
