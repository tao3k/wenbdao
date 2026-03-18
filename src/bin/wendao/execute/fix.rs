use crate::types::{Cli, FixArgs};
use anyhow::{Context, Result};
use std::collections::HashMap;
use xiuxian_wendao::link_graph::LinkGraphIndex;
use xiuxian_wendao::zhenfa_router::native::audit::generate_batch_fixes;
use xiuxian_wendao::zhenfa_router::native::semantic_check::{
    WendaoSemanticCheckArgs, run_audit_core,
};
use xiuxian_zhenfa::ZhenfaContext;

pub(super) fn handle(_cli: &Cli, args: &FixArgs, index: Option<&LinkGraphIndex>) -> Result<()> {
    let mut ctx = ZhenfaContext::default();

    // Inject the index into context extensions
    if let Some(idx) = index {
        ctx.insert_extension(idx.clone());
    } else {
        anyhow::bail!("LinkGraphIndex must be provided for fix (check your environment or --root)");
    }

    println!("🔍 Phase 1: Auditing targets for remediable issues...");

    // 1. Run the official semantic check core
    let check_args = WendaoSemanticCheckArgs {
        doc: Some(args.path.clone()),
        checks: None,
        include_warnings: Some(true),
        source_paths: Some(vec!["src".to_string()]),
        fuzzy_confidence_threshold: Some(args.confidence_threshold),
    };

    let (issues, file_contents) = run_audit_core(&ctx, &check_args)
        .map_err(|e| anyhow::anyhow!("Audit core failed: {e:?}"))?;

    if issues.is_empty() {
        println!("✅ No issues found. Your knowledge base is healthy.");
        return Ok(());
    }

    // 2. Generate surgical fixes
    println!("🛠️  Phase 2: Generating surgical remediation plans...");
    let fixes = generate_batch_fixes(&issues);

    if fixes.is_empty() {
        println!("⚠️  Found issues, but none have automatic fix suggestions.");
        return Ok(());
    }

    // 3. Apply fixes
    if args.dry_run {
        println!("🧪 Dry Run: Previewing {} suggested fixes:", fixes.len());
        for fix in &fixes {
            println!(
                "  - [{}] at {}:{} (Confidence: {:.0}%)",
                fix.issue_type,
                fix.doc_path,
                fix.line_number,
                fix.confidence * 100.0
            );
        }
        println!("\nTotal: {} fixes (NOT APPLIED)", fixes.len());
    } else {
        println!("🚀 Applying {} surgical fixes...", fixes.len());

        // Group fixes by file for transactional application
        let mut grouped_fixes: HashMap<String, Vec<_>> = HashMap::new();
        for fix in fixes {
            grouped_fixes
                .entry(fix.doc_path.clone())
                .or_default()
                .push(fix);
        }

        for (doc_id, file_fixes) in grouped_fixes {
            // Resolve doc_id to physical path using the index
            let path = if let Some(idx) = index {
                idx.doc_path(&doc_id)
                    .map_or_else(|| doc_id.clone(), std::string::ToString::to_string)
            } else {
                doc_id.clone()
            };

            let content_opt = file_contents.get(&doc_id).cloned();
            let mut modified_content = match content_opt {
                Some(c) => c,
                None => std::fs::read_to_string(&path).with_context(|| {
                    format!("Failed to read file for fixing: {path} (resolved from {doc_id})")
                })?,
            };

            let mut success_count = 0;

            // Apply fixes in reverse order to maintain byte offset integrity
            let mut sorted_fixes = file_fixes;
            sorted_fixes.sort_by(|a, b| {
                let a_start = a.byte_range.as_ref().map_or(usize::MAX, |r| r.start);
                let b_start = b.byte_range.as_ref().map_or(usize::MAX, |r| r.start);
                b_start.cmp(&a_start)
            });

            for fix in sorted_fixes {
                let result = fix.apply_surgical(&mut modified_content);
                if matches!(
                    result,
                    xiuxian_wendao::zhenfa_router::native::audit::FixResult::Success
                ) {
                    success_count += 1;
                } else {
                    println!("  ❌ Failed to apply fix to {doc_id}: {result}");
                }
            }

            if success_count > 0 {
                std::fs::write(&path, modified_content)
                    .with_context(|| format!("Failed to write fixed content to: {path}"))?;
                println!("  ✓ Applied {success_count} fixes to {doc_id}");
            }
        }
        println!("\n✅ Remediation complete.");
    }

    Ok(())
}
