use std::io::Write as IoWrite;
use std::path::PathBuf;

use xiuxian_wendao::dependency_indexer::{
    ExternalSymbol, SymbolIndex, SymbolKind, extract_symbols,
};

pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(crate) fn extract_fixture_symbols(
    content: &str,
    language: &str,
) -> Result<Vec<ExternalSymbol>, Box<dyn std::error::Error>> {
    let temp_file = tempfile::NamedTempFile::new()?;
    {
        let mut writer = std::io::BufWriter::new(&temp_file);
        writer.write_all(content.as_bytes())?;
        writer.flush()?;
    }
    extract_symbols(temp_file.path(), language).map_err(Into::into)
}

pub(crate) fn build_symbol_index() -> SymbolIndex {
    let mut index = SymbolIndex::new();
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
    index
}
