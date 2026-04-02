use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use git2::{IndexAddOption, Repository, Signature, Time};

pub(crate) fn write_default_repo_config(base: &Path, repo_dir: &Path, repo_id: &str) -> Result<()> {
    fs::write(
        base.join("wendao.toml"),
        format!(
            r#"[link_graph.projects.{repo_id}]
root = "{}"
plugins = ["julia"]
"#,
            repo_dir.display()
        ),
    )?;
    Ok(())
}

pub(crate) fn create_local_git_repo(base: &Path, package_name: &str) -> Result<PathBuf> {
    let repo_dir = base.join(package_name.to_ascii_lowercase());
    fs::create_dir_all(repo_dir.join("src"))?;
    fs::write(repo_dir.join("README.md"), "# Gateway Repo\n")?;
    fs::write(
        repo_dir.join("Project.toml"),
        format!(
            r#"name = "{package_name}"
uuid = "12345678-1234-1234-1234-123456789abc"
version = "0.1.0"
"#
        ),
    )?;
    fs::write(
        repo_dir.join("src").join(format!("{package_name}.jl")),
        format!(
            "module {package_name}\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n"
        ),
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        format!("using {package_name}\nsolve()\n"),
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;

    let repository = Repository::init(&repo_dir)?;
    repository.remote(
        "origin",
        &format!(
            "https://example.invalid/xiuxian-wendao/{}.git",
            package_name.to_ascii_lowercase()
        ),
    )?;
    commit_all(&repository, "initial import")?;
    Ok(repo_dir)
}

fn commit_all(repository: &Repository, message: &str) -> Result<()> {
    let mut index = repository.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repository.find_tree(tree_id)?;
    let signature = Signature::new("Gateway Perf", "gateway-perf@example.invalid", &git_time())?;
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|target| repository.find_commit(target).ok());

    match parent {
        Some(ref commit) => {
            repository.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[commit],
            )?;
        }
        None => {
            repository.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?;
        }
    }

    Ok(())
}

fn git_time() -> Time {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("system time before unix epoch: {error}"))
        .as_secs();
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    Time::new(seconds, 0)
}
