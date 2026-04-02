use crate::zhenfa_router::native::semantic_check::docs_governance::tests::support::*;

#[test]
fn workspace_doc_identity_scan_respects_explicit_doc_scope() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    let core_dir = crate_dir.join("docs/01_core");
    fs::create_dir_all(&core_dir).or_panic("create core docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let intro_path = core_dir.join("101_intro.md");
    let intro_path_str = intro_path.to_string_lossy().to_string();
    fs::write(
        &intro_path,
        "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write intro");

    let contracts_path = core_dir.join("102_contracts.md");
    fs::write(
        &contracts_path,
        "# Contracts\n\n:PROPERTIES:\n:ID: readable-contracts\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write contracts");

    let issues = collect_workspace_doc_governance_issues(temp.path(), Some(&intro_path_str));
    let identity_issues = issues
        .iter()
        .filter(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
        .collect::<Vec<_>>();

    assert_eq!(identity_issues.len(), 1);
    assert_eq!(
        Path::new(&identity_issues[0].doc)
            .canonicalize()
            .or_panic("canonical issue path"),
        intro_path.canonicalize().or_panic("canonical intro path")
    );
}

#[test]
fn workspace_scope_does_not_match_prefix_sibling_crates() {
    let temp = TempDir::new().or_panic("tempdir");
    let wendao_dir = temp.path().join("packages/rust/crates/xiuxian-wendao");
    let modelica_dir = temp
        .path()
        .join("packages/rust/crates/xiuxian-wendao-modelica");

    for crate_dir in [&wendao_dir, &modelica_dir] {
        fs::create_dir_all(crate_dir.join("docs/01_core")).or_panic("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{}\"\nversion = \"0.1.0\"\n",
                crate_dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .or_panic("crate name")
            ),
        )
        .or_panic("write cargo");
    }

    let wendao_doc = wendao_dir.join("docs/01_core/101_core.md");
    fs::write(
        &wendao_doc,
        "# Wendao\n\n:PROPERTIES:\n:ID: readable-wendao\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write wendao doc");

    let modelica_doc = modelica_dir.join("docs/01_core/101_core.md");
    fs::write(
        &modelica_doc,
        "# Modelica\n\n:PROPERTIES:\n:ID: readable-modelica\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write modelica doc");

    let issues = collect_workspace_doc_governance_issues(
        temp.path(),
        Some(&modelica_dir.join("docs").to_string_lossy()),
    );
    let identity_issues = issues
        .iter()
        .filter(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
        .collect::<Vec<_>>();

    assert_eq!(identity_issues.len(), 1);
    assert_eq!(
        Path::new(&identity_issues[0].doc)
            .canonicalize()
            .or_panic("canonical issue path"),
        modelica_doc
            .canonicalize()
            .or_panic("canonical modelica doc")
    );
}
