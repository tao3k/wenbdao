//! Integration tests for bounded agentic expansion planning.

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use xiuxian_wendao::{LinkGraphAgenticExpansionConfig, LinkGraphIndex};

fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[test]
fn test_agentic_expansion_plan_respects_worker_and_pair_budgets()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("notes/a.md"),
        "---\ntags:\n  - alpha\n---\n# A\n\ncontent\n",
    )?;
    write_file(
        &tmp.path().join("notes/b.md"),
        "---\ntags:\n  - alpha\n---\n# B\n\ncontent\n",
    )?;
    write_file(
        &tmp.path().join("notes/c.md"),
        "---\ntags:\n  - beta\n---\n# C\n\ncontent\n",
    )?;
    write_file(
        &tmp.path().join("notes/d.md"),
        "---\ntags:\n  - gamma\n---\n# D\n\ncontent\n",
    )?;

    let index = LinkGraphIndex::build(tmp.path()).map_err(|err| err.to_string())?;
    let plan = index.agentic_expansion_plan_with_config(
        None,
        LinkGraphAgenticExpansionConfig {
            max_workers: 2,
            max_candidates: 4,
            max_pairs_per_worker: 2,
            time_budget_ms: 1_000.0,
        },
    );

    assert_eq!(plan.total_notes, 4);
    assert_eq!(plan.candidate_notes, 4);
    assert_eq!(plan.total_possible_pairs, 6);
    assert!(plan.workers.len() <= 2);
    assert!(plan.workers.iter().all(|worker| worker.pair_count <= 2));
    assert!(plan.selected_pairs <= 4);
    assert_eq!(
        plan.selected_pairs,
        plan.workers
            .iter()
            .map(|worker| worker.pair_count)
            .sum::<usize>()
    );

    let mut seen_pairs = std::collections::HashSet::<(String, String)>::new();
    for worker in &plan.workers {
        for pair in &worker.pairs {
            let key = if pair.left_id <= pair.right_id {
                (pair.left_id.clone(), pair.right_id.clone())
            } else {
                (pair.right_id.clone(), pair.left_id.clone())
            };
            assert!(seen_pairs.insert(key), "duplicate candidate pair in plan");
        }
    }

    Ok(())
}

#[test]
fn test_agentic_expansion_plan_query_narrows_candidates() -> Result<(), Box<dyn std::error::Error>>
{
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\nalpha momentum\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\nalpha breakout\n")?;
    write_file(
        &tmp.path().join("docs/c.md"),
        "# C\n\nbeta mean reversion\n",
    )?;
    write_file(&tmp.path().join("docs/d.md"), "# D\n\ngamma divergence\n")?;

    let index = LinkGraphIndex::build(tmp.path()).map_err(|err| err.to_string())?;
    let plan = index.agentic_expansion_plan_with_config(
        Some("alpha"),
        LinkGraphAgenticExpansionConfig {
            max_workers: 3,
            max_candidates: 10,
            max_pairs_per_worker: 3,
            time_budget_ms: 1_000.0,
        },
    );

    assert_eq!(plan.query.as_deref(), Some("alpha"));
    assert!(plan.candidate_notes <= 2);
    assert!(plan.selected_pairs <= 1);
    assert!(plan.workers.len() <= 1);

    Ok(())
}
