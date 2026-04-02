use crate::analyzers::{ProjectedGapKind, ProjectionPageKind, RepoSyncMode};
use crate::gateway::studio::router::StudioApiError;

pub(crate) fn required_repo_id(repo: Option<&str>) -> Result<String, StudioApiError> {
    repo.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_REPO", "`repo` is required"))
}

pub(crate) fn required_search_query(query: Option<&str>) -> Result<String, StudioApiError> {
    query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`query` is required"))
}

pub(crate) fn optional_search_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn required_import_search_filters(
    package: Option<&str>,
    module: Option<&str>,
) -> Result<(Option<String>, Option<String>), StudioApiError> {
    let package = optional_search_filter(package);
    let module = optional_search_filter(module);
    if package.is_none() && module.is_none() {
        return Err(StudioApiError::bad_request(
            "MISSING_IMPORT_FILTER",
            "at least one of `package` or `module` is required",
        ));
    }
    Ok((package, module))
}

pub(crate) fn required_page_id(page_id: Option<&str>) -> Result<String, StudioApiError> {
    page_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PAGE_ID", "`page_id` is required"))
}

pub(crate) fn required_gap_id(gap_id: Option<&str>) -> Result<String, StudioApiError> {
    gap_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_GAP_ID", "`gap_id` is required"))
}

pub(super) fn required_node_id(node_id: Option<&str>) -> Result<String, StudioApiError> {
    node_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_NODE_ID", "`node_id` is required"))
}

pub(crate) fn parse_repo_sync_mode(mode: Option<&str>) -> Result<RepoSyncMode, StudioApiError> {
    match mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("ensure")
    {
        "ensure" => Ok(RepoSyncMode::Ensure),
        "refresh" => Ok(RepoSyncMode::Refresh),
        "status" => Ok(RepoSyncMode::Status),
        other => Err(StudioApiError::bad_request(
            "INVALID_MODE",
            format!("unsupported repo sync mode `{other}`"),
        )),
    }
}

pub(crate) fn parse_projection_page_kind(
    kind: Option<&str>,
) -> Result<Option<ProjectionPageKind>, StudioApiError> {
    match kind.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some("reference") => Ok(Some(ProjectionPageKind::Reference)),
        Some("how_to") => Ok(Some(ProjectionPageKind::HowTo)),
        Some("tutorial") => Ok(Some(ProjectionPageKind::Tutorial)),
        Some("explanation") => Ok(Some(ProjectionPageKind::Explanation)),
        Some(other) => Err(StudioApiError::bad_request(
            "INVALID_KIND",
            format!("unsupported projected page kind `{other}`"),
        )),
    }
}

pub(crate) fn required_projection_page_kind(
    kind: Option<&str>,
) -> Result<ProjectionPageKind, StudioApiError> {
    parse_projection_page_kind(kind)?
        .ok_or_else(|| StudioApiError::bad_request("MISSING_KIND", "`kind` is required"))
}

pub(crate) fn parse_projected_gap_kind(
    kind: Option<&str>,
) -> Result<Option<ProjectedGapKind>, StudioApiError> {
    match kind.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some("module_reference_without_documentation") => {
            Ok(Some(ProjectedGapKind::ModuleReferenceWithoutDocumentation))
        }
        Some("symbol_reference_without_documentation") => {
            Ok(Some(ProjectedGapKind::SymbolReferenceWithoutDocumentation))
        }
        Some("symbol_reference_unverified") => {
            Ok(Some(ProjectedGapKind::SymbolReferenceUnverified))
        }
        Some("example_how_to_without_anchor" | "example_howto_without_anchor") => {
            Ok(Some(ProjectedGapKind::ExampleHowToWithoutAnchor))
        }
        Some("documentation_page_without_anchor") => {
            Ok(Some(ProjectedGapKind::DocumentationPageWithoutAnchor))
        }
        Some(other) => Err(StudioApiError::bad_request(
            "INVALID_GAP_KIND",
            format!("unsupported projected gap kind `{other}`"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::required_import_search_filters;

    #[test]
    fn import_search_filters_require_package_or_module() {
        let error = required_import_search_filters(None, None)
            .expect_err("missing import filters should fail");
        assert_eq!(error.code(), "MISSING_IMPORT_FILTER");
    }
}
