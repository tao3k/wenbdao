//! Export TypeScript bindings from Rust Specta types.
//!
//! This binary generates `bindings.ts` for the Qianji Studio frontend,
//! ensuring type safety between the Rust backend and TypeScript frontend.
//!
//! Usage:
//!   cargo run --bin `export_types` --features zhenfa-router

use specta_typescript::Typescript;
use xiuxian_wendao::gateway::studio::types::studio_type_collection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let types = studio_type_collection();
    let ts = Typescript::new()
        .header("// Auto-generated from xiuxian-wendao\n// Run: cargo run --bin export_types --features zhenfa-router\n\n")
        .export(&types)?;

    let output_path = std::path::PathBuf::from(".data/qianji-studio/src/api/bindings.ts");

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&output_path, ts)?;
    println!("TypeScript bindings written to: {}", output_path.display());
    Ok(())
}
