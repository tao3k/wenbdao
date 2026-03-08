use super::SymbolIndex;
use crate::dependency_indexer::symbols::{ExternalSymbol, SymbolKind};
use std::io::Write;

impl SymbolIndex {
    /// Serialize to JSON string.
    #[must_use]
    pub fn serialize(&self) -> String {
        let mut output = Vec::new();

        for crate_sym in &self.by_crate {
            for symbol in &crate_sym.symbols {
                let kind_str = match symbol.kind {
                    SymbolKind::Struct => "struct",
                    SymbolKind::Enum => "enum",
                    SymbolKind::Trait => "trait",
                    SymbolKind::Function => "fn",
                    SymbolKind::Method => "method",
                    SymbolKind::Field => "field",
                    SymbolKind::Impl => "impl",
                    SymbolKind::Mod => "mod",
                    SymbolKind::Const => "const",
                    SymbolKind::Static => "static",
                    SymbolKind::TypeAlias => "type",
                    SymbolKind::Unknown => "unknown",
                };

                let line = symbol.line;
                let file = symbol.file.to_string_lossy();

                // Format: crate_name|symbol_name|kind|file:line
                if writeln!(
                    output,
                    "{}|{}|{}|{}:{}",
                    crate_sym.name, symbol.name, kind_str, file, line
                )
                .is_err()
                {
                    return String::new();
                }
            }
        }

        String::from_utf8(output).unwrap_or_default()
    }

    /// Deserialize from JSON string.
    #[must_use]
    pub fn deserialize(&mut self, data: &str) -> bool {
        self.clear();

        for line in data.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 4 {
                continue;
            }

            let crate_name = parts[0];
            let name = parts[1];
            let kind_str = parts[2];
            let loc = parts[3];

            let kind = match kind_str {
                "struct" => SymbolKind::Struct,
                "enum" => SymbolKind::Enum,
                "trait" => SymbolKind::Trait,
                "fn" => SymbolKind::Function,
                "method" => SymbolKind::Method,
                "field" => SymbolKind::Field,
                "impl" => SymbolKind::Impl,
                "mod" => SymbolKind::Mod,
                "const" => SymbolKind::Const,
                "static" => SymbolKind::Static,
                "type" => SymbolKind::TypeAlias,
                _ => SymbolKind::Unknown,
            };

            // Parse file:line
            let mut file_parts = loc.rsplitn(2, ':');
            let file = file_parts.nth(1).unwrap_or(loc);
            let line = file_parts
                .nth(0)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(1);

            let symbol = ExternalSymbol {
                name: name.to_string(),
                kind,
                file: std::path::PathBuf::from(file),
                line,
                crate_name: crate_name.to_string(),
            };

            self.add_symbols(crate_name, &[symbol]);
        }

        true
    }
}
