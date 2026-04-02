use std::fs;
use std::path::{Path, PathBuf};

use git2::{BranchType, IndexAddOption, Repository, Signature, Time, build::CheckoutBuilder};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub type TestResultPath = Result<PathBuf, Box<dyn std::error::Error>>;

pub fn create_sample_julia_repo(
    base: &Path,
    package_name: &str,
    expected_root: bool,
) -> TestResultPath {
    let repo_dir = base.join(package_name.to_ascii_lowercase());
    fs::create_dir_all(repo_dir.join("src"))?;
    fs::create_dir_all(repo_dir.join("src").join("nested"))?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::create_dir_all(repo_dir.join("test"))?;
    fs::create_dir_all(repo_dir.join("docs"))?;

    fs::write(
        repo_dir.join("Project.toml"),
        format!(
            r#"name = "{package_name}"
uuid = "12345678-1234-1234-1234-123456789abc"
version = "0.1.0"

[deps]
SciMLBase = "0bca4576-84f4-4d90-8ffe-ffa030f20462"
LinearAlgebra = "37e2e46d-f89d-539d-b4ee-838fcccc9c8e"
"#
        ),
    )?;

    let module_name = if expected_root {
        package_name.to_string()
    } else {
        format!("{package_name}Alt")
    };
    let root_file_name = if expected_root {
        format!("{package_name}.jl")
    } else {
        "Other.jl".to_string()
    };
    fs::write(
        repo_dir.join("src").join(root_file_name),
        format!(
            r#"module {module_name}

export solve, Problem
using LinearAlgebra
@reexport using SciMLBase
include("solvers.jl")

"""
Problem docs.
"""
struct Problem
    x::Int
end

"""
Solve docs.
"""
function solve(problem::Problem)
    problem.x
end

"""
end
"#
        ),
    )?;
    fs::write(
        repo_dir.join("src").join("solvers.jl"),
        r#"""
Fast solve docs.
"""
fastsolve(problem::Problem) = problem.x

include("nested/extra.jl")
"#,
    )?;
    fs::write(
        repo_dir.join("src").join("nested").join("extra.jl"),
        r#"""
Extra problem docs.
"""
struct ExtraProblem
    y::Int
end
"#,
    )?;

    fs::write(
        repo_dir.join("examples").join("basic.jl"),
        "problem = Problem(1)\nsolve(problem)\nfastsolve(problem)\n",
    )?;
    fs::write(
        repo_dir.join("test").join("runtests.jl"),
        "extra = ExtraProblem(2)\nprintln(extra)\n",
    )?;
    fs::write(repo_dir.join("README.md"), "# Sample\n")?;
    fs::write(repo_dir.join("docs").join("guide.md"), "# Guide\n")?;
    initialize_git_repository(
        &repo_dir,
        &format!(
            "https://example.invalid/{}/{}.git",
            "xiuxian-wendao",
            package_name.to_ascii_lowercase()
        ),
    )?;
    Ok(repo_dir)
}

pub fn create_sample_modelica_repo(base: &Path, package_name: &str) -> TestResultPath {
    let repo_dir = base.join(package_name.to_ascii_lowercase());
    fs::create_dir_all(repo_dir.join("Controllers").join("Examples"))?;
    fs::create_dir_all(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Tutorial"),
    )?;

    fs::write(repo_dir.join("README.md"), format!("# {package_name}\n"))?;
    fs::write(repo_dir.join("package.order"), "Controllers\n")?;
    fs::write(
        repo_dir.join("package.mo"),
        format!(
            "within;\npackage {package_name}\n  annotation(Documentation(info = \"<html>{package_name} package docs.</html>\"));\nend {package_name};\n",
        ),
    )?;
    fs::write(
        repo_dir.join("Controllers").join("package.mo"),
        format!("within {package_name};\npackage Controllers\nend Controllers;\n"),
    )?;
    fs::write(
        repo_dir.join("Controllers").join("PI.mo"),
        format!(
            "within {package_name}.Controllers;\nmodel PI\n  annotation(Documentation(info = \"<html>PI controller docs.</html>\"));\nend PI;\n",
        ),
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("package.order"),
        "Step\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("Step.mo"),
        format!("within {package_name}.Controllers.Examples;\nmodel Step\nend Step;\n"),
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("package.order"),
        "Tutorial\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("package.mo"),
        format!("within {package_name}.Controllers;\npackage UsersGuide\nend UsersGuide;\n",),
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Tutorial")
            .join("FirstSteps.mo"),
        format!(
            "within {package_name}.Controllers.UsersGuide.Tutorial;\nmodel FirstSteps\n  annotation(Documentation(info = \"<html>First steps guide.</html>\"));\nend FirstSteps;\n",
        ),
    )?;

    initialize_git_repository(
        &repo_dir,
        &format!(
            "https://example.invalid/{}/{}.git",
            "xiuxian-wendao",
            package_name.to_ascii_lowercase()
        ),
    )?;
    Ok(repo_dir)
}

fn initialize_git_repository(repo_dir: &Path, remote_url: &str) -> TestResult {
    let repository = Repository::init(repo_dir)?;
    repository.remote("origin", remote_url)?;
    let commit = commit_all(&repository, "initial import")?;
    ensure_branch_main(&repository, commit)?;
    Ok(())
}

fn commit_all(repository: &Repository, message: &str) -> Result<git2::Oid, git2::Error> {
    let mut index = repository.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repository.find_tree(tree_id)?;
    let signature = Signature::new(
        "Xiuxian Test",
        "test@example.com",
        &Time::new(1_700_000_000, 0),
    )?;
    let parent_commit = repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repository.find_commit(oid).ok());
    let parent_refs = parent_commit.iter().collect::<Vec<_>>();

    repository.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parent_refs,
    )
}

fn ensure_branch_main(repository: &Repository, commit_id: git2::Oid) -> Result<(), git2::Error> {
    let commit = repository.find_commit(commit_id)?;
    match repository.find_branch("main", BranchType::Local) {
        Ok(local_branch) => {
            let mut reference = local_branch.into_reference();
            reference.set_target(commit.id(), "move main to latest test commit")?;
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            repository.branch("main", &commit, true)?;
        }
        Err(error) => return Err(error),
    }
    repository.set_head("refs/heads/main")?;
    let mut checkout = CheckoutBuilder::new();
    checkout.force();
    repository.checkout_head(Some(&mut checkout))?;
    Ok(())
}
