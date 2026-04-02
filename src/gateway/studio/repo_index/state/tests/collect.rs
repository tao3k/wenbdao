use crate::gateway::studio::repo_index::state::collect::collect_code_documents;

#[test]
fn collect_code_documents_returns_none_when_cancelled() {
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    std::fs::write(tempdir.path().join("module.jl"), "module Demo\nend\n")
        .unwrap_or_else(|error| panic!("write file: {error}"));

    let documents = collect_code_documents(tempdir.path(), || true);

    assert!(documents.is_none());
}
