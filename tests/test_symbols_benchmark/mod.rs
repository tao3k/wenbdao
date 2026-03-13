use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;

use serde_json::{Value, json};
use xiuxian_wendao::dependency_indexer::{ExternalSymbol, SymbolKind};

fn append_format(content: &mut String, args: std::fmt::Arguments<'_>) {
    if content.write_fmt(args).is_err() {
        unreachable!("formatting into String should not fail");
    }
}

pub(crate) fn generate_rust_test_file(line_count: usize) -> String {
    let mut content = String::with_capacity(line_count * 50);

    for i in 0..(line_count / 50) {
        append_format(
            &mut content,
            format_args!(
                "pub struct Struct{i} {{\n    field_{i}: String,\n    field_{i}: i32,\n}}\n"
            ),
        );
    }

    for i in 0..(line_count / 100) {
        append_format(
            &mut content,
            format_args!(
                "pub enum Enum{i} {{\n    VariantA,\n    VariantB(i32),\n    VariantC {{ x: i32, y: i32 }},\n}}\n"
            ),
        );
    }

    for i in 0..(line_count / 30) {
        append_format(
            &mut content,
            format_args!(
                "pub fn function_{i}(arg1: &str, arg2: i32) -> Result<(), Box<dyn std::error::Error>> {{\n    let _result = process_data(arg1, arg2);\n    Ok(())\n}}\n"
            ),
        );
    }

    for i in 0..(line_count / 80) {
        append_format(
            &mut content,
            format_args!(
                "pub trait Trait{i} {{\n    fn method_a(&self) -> i32;\n    fn method_b(&self, x: i32) -> bool;\n}}\n"
            ),
        );
    }

    content
}

pub(crate) fn generate_python_test_file(line_count: usize) -> String {
    let mut content = String::with_capacity(line_count * 40);

    for i in 0..(line_count / 50) {
        append_format(
            &mut content,
            format_args!(
                "class Class{i}:\n    def __init__(self, param_a: str, param_b: int):\n        self.param_a = param_a\n        self.param_b = param_b\n\n    def method_a(self) -> str:\n        return self.param_a.upper()\n\n    def method_b(self, value: int) -> bool:\n        return value > 0\n\n    async def async_method(self) -> dict:\n        return {{\"status\": \"ok\"}}\n"
            ),
        );
    }

    for i in 0..(line_count / 20) {
        append_format(
            &mut content,
            format_args!(
                "def function_{i}(arg1: str, arg2: int) -> bool:\n    \"\"\"Process data and return result.\"\"\"\n    result = process(arg1, arg2)\n    return result\n\nasync def async_function_{i}(data: dict) -> list:\n    \"\"\"Async data processing.\"\"\"\n    results = []\n    return results\n"
            ),
        );
    }

    content
}

pub(crate) fn symbol_kind_name(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::Trait => "trait",
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Field => "field",
        SymbolKind::Impl => "impl",
        SymbolKind::Mod => "mod",
        SymbolKind::Const => "const",
        SymbolKind::Static => "static",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Unknown => "unknown",
    }
}

pub(crate) fn symbol_kind_counts(symbols: &[ExternalSymbol]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for symbol in symbols {
        *counts
            .entry(symbol_kind_name(&symbol.kind).to_string())
            .or_insert(0) += 1;
    }
    counts
}

pub(crate) fn symbol_rows(symbols: &[ExternalSymbol], limit: usize) -> Vec<Value> {
    symbols
        .iter()
        .take(limit)
        .map(|symbol| {
            json!({
                "name": symbol.name,
                "kind": symbol_kind_name(&symbol.kind),
                "line": symbol.line,
            })
        })
        .collect()
}
