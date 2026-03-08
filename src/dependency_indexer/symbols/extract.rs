use std::path::Path;
use std::sync::LazyLock;

use super::{ExternalSymbol, SymbolKind};

/// Extract symbols from a source file (synchronous).
///
/// # Errors
///
/// Returns I/O errors when reading `path`.
pub fn extract_symbols(path: &Path, lang: &str) -> Result<Vec<ExternalSymbol>, std::io::Error> {
    use std::fs::read_to_string;
    let content = read_to_string(path)?;

    let mut symbols = Vec::new();

    match lang {
        "rust" => extract_rust_symbols(&content, path, &mut symbols),
        "python" => extract_python_symbols(&content, path, &mut symbols),
        _ => {}
    }

    Ok(symbols)
}

fn compile_regex(pattern: &str) -> regex::Regex {
    match regex::Regex::new(pattern) {
        Ok(regex) => regex,
        Err(_pattern_err) => match regex::Regex::new(r"$^") {
            Ok(fallback) => fallback,
            Err(fallback_err) => panic!("hardcoded fallback regex must compile: {fallback_err}"),
        },
    }
}

// Pre-compiled Rust regex patterns for extraction performance.
static RE_STRUCT: LazyLock<regex::Regex> =
    LazyLock::new(|| compile_regex(r"(?:pub\s+)?struct\s+(\w+)"));
static RE_ENUM: LazyLock<regex::Regex> =
    LazyLock::new(|| compile_regex(r"(?:pub\s+)?enum\s+(\w+)"));
static RE_TRAIT: LazyLock<regex::Regex> =
    LazyLock::new(|| compile_regex(r"(?:pub\s+)?trait\s+(\w+)"));
static RE_FN: LazyLock<regex::Regex> = LazyLock::new(|| compile_regex(r"(?:pub\s+)?fn\s+(\w+)"));
static RE_IMPL: LazyLock<regex::Regex> = LazyLock::new(|| compile_regex(r"impl\s+(\w+)"));
static RE_MOD: LazyLock<regex::Regex> = LazyLock::new(|| compile_regex(r"(?:pub\s+)?mod\s+(\w+)"));
static RE_TYPE: LazyLock<regex::Regex> =
    LazyLock::new(|| compile_regex(r"(?:pub\s+)?type\s+(\w+)"));
static RE_CONST: LazyLock<regex::Regex> =
    LazyLock::new(|| compile_regex(r"(?:pub\s+)?const\s+(\w+)"));
static RE_STATIC: LazyLock<regex::Regex> =
    LazyLock::new(|| compile_regex(r"(?:pub\s+)?static\s+(\w+)"));

fn extract_rust_symbols(content: &str, path: &Path, symbols: &mut Vec<ExternalSymbol>) {
    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;

        // pub struct Name
        if let Some(cap) = RE_STRUCT.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Struct,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // pub enum Name
        else if let Some(cap) = RE_ENUM.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Enum,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // pub trait Name
        else if let Some(cap) = RE_TRAIT.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Trait,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // pub fn name
        else if let Some(cap) = RE_FN.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Function,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // impl Name
        else if let Some(cap) = RE_IMPL.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Impl,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // mod name
        else if let Some(cap) = RE_MOD.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Mod,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // type Name
        else if let Some(cap) = RE_TYPE.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::TypeAlias,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // const NAME
        else if let Some(cap) = RE_CONST.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Const,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // static NAME
        else if let Some(cap) = RE_STATIC.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Static,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
    }
}

// Pre-compiled Python regex patterns for extraction performance.
static RE_PY_CLASS: LazyLock<regex::Regex> = LazyLock::new(|| compile_regex(r"class\s+(\w+)"));
static RE_PY_DEF: LazyLock<regex::Regex> = LazyLock::new(|| compile_regex(r"def\s+(\w+)"));
static RE_PY_ASYNC_DEF: LazyLock<regex::Regex> =
    LazyLock::new(|| compile_regex(r"async\s+def\s+(\w+)"));

fn extract_python_symbols(content: &str, path: &Path, symbols: &mut Vec<ExternalSymbol>) {
    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;

        // class Name
        if let Some(cap) = RE_PY_CLASS.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Struct, // Map class to struct
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // def name
        else if let Some(cap) = RE_PY_DEF.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Function,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
        // async def name
        else if let Some(cap) = RE_PY_ASYNC_DEF.captures(line) {
            symbols.push(ExternalSymbol {
                name: cap[1].to_string(),
                kind: SymbolKind::Function,
                file: path.to_path_buf(),
                line: line_num,
                crate_name: String::new(),
            });
        }
    }
}
