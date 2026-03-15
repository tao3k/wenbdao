use xiuxian_wendao::extract_extensions_from_glob_patterns;

#[test]
fn test_extract_extensions_from_glob_patterns() {
    let patterns = vec![
        "**/*.md".to_string(),
        "**/*.org".to_string(),
        "templates/*.j2".to_string(),
        "**/*.toml".to_string(),
        "**/*.{md,markdown}".to_string(),
    ];

    let extensions = extract_extensions_from_glob_patterns(&patterns);
    assert_eq!(
        extensions,
        vec![
            "j2".to_string(),
            "markdown".to_string(),
            "md".to_string(),
            "org".to_string(),
            "toml".to_string(),
        ]
    );
}

#[test]
fn test_extract_extensions_from_brace_glob_patterns() {
    let patterns = vec!["**/*.{md,org,j2,toml}".to_string()];
    let extensions = extract_extensions_from_glob_patterns(&patterns);
    assert_eq!(
        extensions,
        vec![
            "j2".to_string(),
            "md".to_string(),
            "org".to_string(),
            "toml".to_string(),
        ]
    );
}

#[test]
fn test_extract_extensions_from_compound_suffix_patterns() {
    let patterns = vec![
        "**/*.md.j2".to_string(),
        "**/*.agenda.toml".to_string(),
        "**/*.{org,template.md.j2}".to_string(),
    ];
    let extensions = extract_extensions_from_glob_patterns(&patterns);
    assert_eq!(
        extensions,
        vec!["j2".to_string(), "org".to_string(), "toml".to_string()]
    );
}
