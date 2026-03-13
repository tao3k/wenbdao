#![allow(clippy::expect_used)]

use std::io::Write as IoWrite;
use std::path::PathBuf;

use super::{ExternalSymbol, SymbolIndex, SymbolKind, extract_symbols};

#[test]
fn test_extract_rust_symbols() {
    let temp_file = tempfile::NamedTempFile::new().expect("create temp file");
    {
        let mut f = std::io::BufWriter::new(&temp_file);
        writeln!(f, "pub struct MyStruct {{").expect("write");
        writeln!(f, "    field: String,").expect("write");
        writeln!(f, "}}").expect("write");
        writeln!(f).expect("write");
        writeln!(f, "pub enum MyEnum {{").expect("write");
        writeln!(f, "    Variant,").expect("write");
        writeln!(f, "}}").expect("write");
        writeln!(f).expect("write");
        writeln!(f, "pub fn my_function() {{").expect("write");
        writeln!(f, "}}").expect("write");
    }

    let symbols = extract_symbols(temp_file.path(), "rust").expect("extract symbols");

    assert!(
        symbols
            .iter()
            .any(|s| s.name == "MyStruct" && s.kind == SymbolKind::Struct)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "MyEnum" && s.kind == SymbolKind::Enum)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "my_function" && s.kind == SymbolKind::Function)
    );
}

#[test]
fn test_extract_python_symbols() {
    let temp_file = tempfile::NamedTempFile::new().expect("create temp file");
    {
        let mut f = std::io::BufWriter::new(&temp_file);
        writeln!(f, "class MyClass:").expect("write");
        writeln!(f, "    pass").expect("write");
        writeln!(f).expect("write");
        writeln!(f, "def my_function():").expect("write");
        writeln!(f, "    pass").expect("write");
    }

    let symbols = extract_symbols(temp_file.path(), "python").expect("extract symbols");

    assert!(
        symbols
            .iter()
            .any(|s| s.name == "MyClass" && s.kind == SymbolKind::Struct)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "my_function" && s.kind == SymbolKind::Function)
    );
}

#[test]
fn test_symbol_index_search() {
    let mut index = SymbolIndex::new();

    // Add test symbols
    index.add_symbols(
        "serde",
        &[
            ExternalSymbol {
                name: "Serializer".to_string(),
                kind: SymbolKind::Struct,
                file: PathBuf::from("lib.rs"),
                line: 10,
                crate_name: "serde".to_string(),
            },
            ExternalSymbol {
                name: "serialize".to_string(),
                kind: SymbolKind::Function,
                file: PathBuf::from("lib.rs"),
                line: 20,
                crate_name: "serde".to_string(),
            },
        ],
    );

    index.add_symbols(
        "tokio",
        &[ExternalSymbol {
            name: "spawn".to_string(),
            kind: SymbolKind::Function,
            file: PathBuf::from("lib.rs"),
            line: 5,
            crate_name: "tokio".to_string(),
        }],
    );

    let results = index.search("serialize", 10);
    // Both "serialize" and "Serializer" match (case-insensitive contains)
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|s| s.name == "Serializer"));
    assert!(results.iter().any(|s| s.name == "serialize"));

    let results = index.search("spawn", 10);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "spawn");

    let results = index.search_crate("serde", "serialize", 10);
    // Both "serialize" and "Serializer" match within serde crate
    assert_eq!(results.len(), 2);
}

#[test]
fn test_serialize_deserialize() {
    let mut index = SymbolIndex::new();

    index.add_symbols(
        "test",
        &[ExternalSymbol {
            name: "MyStruct".to_string(),
            kind: SymbolKind::Struct,
            file: PathBuf::from("lib.rs"),
            line: 10,
            crate_name: "test".to_string(),
        }],
    );

    let data = index.serialize();

    let mut index2 = SymbolIndex::new();
    let _ = index2.deserialize(&data);

    let results = index2.search("MyStruct", 10);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "MyStruct");
}
