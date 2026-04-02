use git2::{Oid, Repository};

use crate::analyzers::config::RepositoryRef;

pub(super) fn apply_git_ref(
    repository: &Repository,
    git_ref: Option<&RepositoryRef>,
) -> Result<(), git2::Error> {
    let Some(git_ref) = git_ref else {
        return Ok(());
    };

    let target = match git_ref {
        RepositoryRef::Branch(branch) => resolve_object_id(
            repository,
            &[
                format!("refs/remotes/origin/{branch}"),
                format!("refs/heads/{branch}"),
            ],
        )?,
        RepositoryRef::Tag(tag) => resolve_object_id(repository, &[format!("refs/tags/{tag}")])?,
        RepositoryRef::Commit(sha) => Oid::from_str(sha)?,
    };

    let object = repository.find_object(target, None)?;
    repository.checkout_tree(&object, None)?;
    repository.set_head_detached(target)?;
    Ok(())
}

pub(super) fn sync_checkout_head(
    repository: &Repository,
    git_ref: Option<&RepositoryRef>,
) -> Result<(), git2::Error> {
    if git_ref.is_some() {
        return apply_git_ref(repository, git_ref);
    }

    let target = resolve_object_id(
        repository,
        &[
            "refs/remotes/origin/HEAD".to_string(),
            repository
                .head()
                .ok()
                .and_then(|head| head.shorthand().map(str::to_string))
                .filter(|name| !name.is_empty() && name != "HEAD")
                .map_or_else(
                    || "refs/remotes/origin/main".to_string(),
                    |name| format!("refs/remotes/origin/{name}"),
                ),
            "refs/remotes/origin/main".to_string(),
            "refs/remotes/origin/master".to_string(),
            "refs/heads/main".to_string(),
            "refs/heads/master".to_string(),
        ],
    )?;
    let object = repository.find_object(target, None)?;
    repository.checkout_tree(&object, None)?;
    repository.set_head_detached(target)?;
    Ok(())
}

fn resolve_object_id(repository: &Repository, candidates: &[String]) -> Result<Oid, git2::Error> {
    for candidate in candidates {
        if let Ok(reference) = repository.revparse_single(candidate) {
            return Ok(reference.id());
        }
    }

    Err(git2::Error::from_str("git reference not found"))
}
