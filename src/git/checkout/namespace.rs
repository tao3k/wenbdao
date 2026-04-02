use std::path::PathBuf;

use url::Url;
use xiuxian_io::PrjDirs;

use crate::analyzers::config::RegisteredRepository;

pub(super) fn managed_checkout_root_for(repository: &RegisteredRepository) -> PathBuf {
    let mut root = PrjDirs::data_home()
        .join("xiuxian-wendao")
        .join("repo-intelligence")
        .join("repos");
    root.push(managed_repo_namespace(repository));
    root
}

pub(super) fn managed_mirror_root_for(repository: &RegisteredRepository) -> PathBuf {
    let mut root = PrjDirs::data_home()
        .join("xiuxian-wendao")
        .join("repo-intelligence")
        .join("mirrors");
    root.push(managed_repo_namespace(repository));

    let leaf = root
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| sanitize_repo_id(repository.id.as_str()));
    root.set_file_name(format!("{leaf}.git"));
    root
}

fn managed_repo_namespace(repository: &RegisteredRepository) -> PathBuf {
    repository
        .url
        .as_deref()
        .and_then(repo_namespace_from_remote_url)
        .unwrap_or_else(|| PathBuf::from(sanitize_repo_id(repository.id.as_str())))
}

fn repo_namespace_from_remote_url(remote_url: &str) -> Option<PathBuf> {
    remote_namespace_segments(remote_url).map(|segments| {
        let mut namespace = PathBuf::new();
        for segment in segments {
            namespace.push(segment);
        }
        namespace
    })
}

fn remote_namespace_segments(remote_url: &str) -> Option<Vec<String>> {
    if let Ok(parsed) = Url::parse(remote_url) {
        let host = parsed.host_str()?.trim();
        if host.is_empty() {
            return None;
        }

        let mut segments = vec![sanitize_namespace_segment(host)];
        segments.extend(
            parsed
                .path_segments()?
                .filter(|segment| !segment.trim().is_empty())
                .map(sanitize_namespace_segment),
        );
        trim_git_suffix(&mut segments);
        return (!segments.is_empty()).then_some(segments);
    }

    let (remote, path) = remote_url.rsplit_once(':')?;
    if remote.contains('/') {
        return None;
    }

    let host = remote
        .rsplit_once('@')
        .map_or(remote, |(_, host)| host)
        .trim();
    if host.is_empty() {
        return None;
    }

    let mut segments = vec![sanitize_namespace_segment(host)];
    segments.extend(
        path.split('/')
            .filter(|segment| !segment.trim().is_empty())
            .map(sanitize_namespace_segment),
    );
    trim_git_suffix(&mut segments);
    (!segments.is_empty()).then_some(segments)
}

fn sanitize_namespace_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn trim_git_suffix(segments: &mut [String]) {
    if let Some(last) = segments.last_mut()
        && let Some(stripped) = last.strip_suffix(".git")
        && !stripped.is_empty()
    {
        *last = stripped.to_string();
    }
}

pub(super) fn sanitize_repo_id(repo_id: &str) -> String {
    repo_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}
