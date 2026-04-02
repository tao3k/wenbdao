//! Bundled `OpenAPI` artifact helpers and invariants for the Wendao gateway.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::Value;

const BUNDLED_WENDAO_GATEWAY_OPENAPI_RELATIVE_PATH: &str =
    "resources/openapi/wendao_gateway.openapi.json";
const BUNDLED_WENDAO_GATEWAY_OPENAPI_TEXT: &str =
    include_str!("../../../resources/openapi/wendao_gateway.openapi.json");

/// Return the checked-in `OpenAPI` document for the Wendao gateway.
#[must_use]
pub fn bundled_wendao_gateway_openapi_document() -> &'static str {
    BUNDLED_WENDAO_GATEWAY_OPENAPI_TEXT
}

/// Return the repository-local path for the checked-in Wendao gateway `OpenAPI` document.
#[must_use]
pub fn bundled_wendao_gateway_openapi_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BUNDLED_WENDAO_GATEWAY_OPENAPI_RELATIVE_PATH)
}

/// Parse the checked-in Wendao gateway `OpenAPI` document.
///
/// # Errors
///
/// Returns an error when the bundled file cannot be parsed as JSON.
pub fn load_bundled_wendao_gateway_openapi_document() -> Result<Value> {
    serde_json::from_str(BUNDLED_WENDAO_GATEWAY_OPENAPI_TEXT)
        .context("failed to parse bundled Wendao gateway OpenAPI document")
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{
        bundled_wendao_gateway_openapi_document, bundled_wendao_gateway_openapi_path,
        load_bundled_wendao_gateway_openapi_document,
    };
    use crate::gateway::openapi::paths::{
        API_UI_CONFIG_OPENAPI_PATH, WENDAO_GATEWAY_ROUTE_CONTRACTS,
    };

    fn operation_summary(operation: &Value) -> &str {
        operation
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    fn operation_description(operation: &Value) -> &str {
        operation
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    #[test]
    fn bundled_gateway_openapi_document_is_valid_json() {
        let document = load_bundled_wendao_gateway_openapi_document()
            .unwrap_or_else(|error| panic!("bundled gateway OpenAPI should parse: {error}"));

        assert_eq!(document["openapi"], Value::String("3.1.0".to_string()));
        assert_eq!(
            document["info"]["title"],
            Value::String("Wendao Gateway".to_string())
        );
        assert!(
            bundled_wendao_gateway_openapi_path().is_file(),
            "bundled gateway OpenAPI path should exist on disk"
        );
        assert!(
            bundled_wendao_gateway_openapi_document().contains("\"paths\""),
            "bundled gateway OpenAPI text should include paths"
        );
    }

    #[test]
    fn bundled_gateway_openapi_document_covers_declared_route_inventory() {
        let document = load_bundled_wendao_gateway_openapi_document()
            .unwrap_or_else(|error| panic!("bundled gateway OpenAPI should parse: {error}"));
        let Some(paths) = document.get("paths").and_then(Value::as_object) else {
            panic!("bundled gateway OpenAPI should contain a `paths` object");
        };

        for route in WENDAO_GATEWAY_ROUTE_CONTRACTS {
            let Some(path_item) = paths.get(route.openapi_path).and_then(Value::as_object) else {
                panic!(
                    "bundled gateway OpenAPI should document path {}",
                    route.openapi_path
                );
            };

            for method in route.methods {
                let Some(operation) = path_item.get(*method) else {
                    panic!(
                        "bundled gateway OpenAPI should document {} {}",
                        method, route.openapi_path
                    );
                };
                assert!(
                    !operation_summary(operation).trim().is_empty(),
                    "{} {} should include a non-empty summary",
                    method,
                    route.openapi_path
                );
                assert!(
                    !operation_description(operation).trim().is_empty(),
                    "{} {} should include a non-empty description",
                    method,
                    route.openapi_path
                );

                let Some(responses) = operation.get("responses").and_then(Value::as_object) else {
                    panic!(
                        "{} {} should include OpenAPI responses",
                        method, route.openapi_path
                    );
                };
                assert!(
                    !responses.is_empty(),
                    "{} {} should document at least one response",
                    method,
                    route.openapi_path
                );
                for (status, response) in responses {
                    let description = response
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    assert!(
                        !description.trim().is_empty(),
                        "{} {} response {} should include a non-empty description",
                        method,
                        route.openapi_path,
                        status
                    );
                }

                if !route.path_params.is_empty() {
                    let Some(parameters) = operation.get("parameters").and_then(Value::as_array)
                    else {
                        panic!(
                            "{} {} should include path parameter declarations",
                            method, route.openapi_path
                        );
                    };
                    for required_param in route.path_params {
                        let matches_param = parameters.iter().any(|parameter| {
                            parameter.get("name").and_then(Value::as_str) == Some(*required_param)
                                && parameter.get("in").and_then(Value::as_str) == Some("path")
                                && parameter.get("required").and_then(Value::as_bool) == Some(true)
                        });
                        assert!(
                            matches_param,
                            "{} {} should declare required path parameter `{}`",
                            method, route.openapi_path, required_param
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn bundled_gateway_openapi_document_keeps_ui_config_post_example() {
        let document = load_bundled_wendao_gateway_openapi_document()
            .unwrap_or_else(|error| panic!("bundled gateway OpenAPI should parse: {error}"));
        let post = &document["paths"][API_UI_CONFIG_OPENAPI_PATH]["post"];

        assert!(
            post["requestBody"]["content"]["application/json"]["example"].is_object(),
            "POST /api/ui/config should include an example request body"
        );
    }

    #[test]
    fn bundled_gateway_openapi_document_declares_rerank_plugin_artifact_examples() {
        let document = load_bundled_wendao_gateway_openapi_document()
            .unwrap_or_else(|error| panic!("bundled gateway OpenAPI should parse: {error}"));
        let get = &document["paths"]["/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}"]["get"];

        assert_eq!(
            get["responses"]["200"]["content"]["application/json"]["example"]["route"].as_str(),
            Some("/rerank")
        );
        assert_eq!(
            get["responses"]["200"]["content"]["text/plain"]["example"].as_str(),
            Some(
                "plugin_id = \"xiuxian-wendao-julia\"\nartifact_id = \"deployment\"\nschema_version = \"v1\"\nbase_url = \"http://127.0.0.1:18080\"\nroute = \"/rerank\"\n"
            )
        );
    }

    #[test]
    fn bundled_gateway_openapi_document_declares_rerank_julia_deployment_artifact_examples() {
        let document = load_bundled_wendao_gateway_openapi_document()
            .unwrap_or_else(|error| panic!("bundled gateway OpenAPI should parse: {error}"));
        let get = &document["paths"]["/api/ui/julia-deployment-artifact"]["get"];

        assert_eq!(
            get["responses"]["200"]["content"]["application/json"]["example"]["route"].as_str(),
            Some("/rerank")
        );
        assert_eq!(
            get["responses"]["200"]["content"]["application/json"]["example"]["healthRoute"]
                .as_str(),
            Some("/healthz")
        );
        assert_eq!(
            get["responses"]["200"]["content"]["text/plain"]["example"].as_str(),
            Some(
                "artifact_schema_version = \"v1\"\ngenerated_at = \"2026-03-27T16:00:00+00:00\"\nbase_url = \"http://127.0.0.1:18080\"\nroute = \"/rerank\"\nhealth_route = \"/healthz\"\nschema_version = \"v1\"\ntimeout_secs = 30\n\n[launch]\nlauncher_path = \".data/WendaoAnalyzer/scripts/run_analyzer_service.sh\"\nargs = [\"--service-mode\", \"stream\", \"--analyzer-strategy\", \"linear_blend\"]\n"
            )
        );
    }

    #[test]
    fn bundled_gateway_openapi_document_omits_flight_only_http_paths() {
        let document = load_bundled_wendao_gateway_openapi_document()
            .unwrap_or_else(|error| panic!("bundled gateway OpenAPI should parse: {error}"));
        let Some(paths) = document.get("paths").and_then(Value::as_object) else {
            panic!("bundled gateway OpenAPI should contain a `paths` object");
        };

        assert!(
            !paths.contains_key("/api/search"),
            "bundled gateway OpenAPI must not expose the retired knowledge HTTP path"
        );
        assert!(
            !paths.contains_key("/api/search/definition"),
            "bundled gateway OpenAPI must not expose the retired definition HTTP path"
        );
        assert!(
            !paths.contains_key("/api/search/autocomplete"),
            "bundled gateway OpenAPI must not expose the retired autocomplete HTTP path"
        );
        assert!(
            !paths.contains_key("/api/search/intent"),
            "bundled gateway OpenAPI must not expose the retired intent HTTP path"
        );
        assert!(
            !paths.contains_key("/api/search/attachments"),
            "bundled gateway OpenAPI must not expose the retired attachments HTTP path"
        );
        assert!(
            !paths.contains_key("/api/search/references"),
            "bundled gateway OpenAPI must not expose the retired references HTTP path"
        );
        assert!(
            !paths.contains_key("/api/search/symbols"),
            "bundled gateway OpenAPI must not expose the retired symbols HTTP path"
        );
        assert!(
            !paths.contains_key("/api/search/ast"),
            "bundled gateway OpenAPI must not expose the retired AST HTTP path"
        );
        assert!(
            !paths.contains_key("/api/graph/neighbors/{id}"),
            "bundled gateway OpenAPI must not expose the retired graph-neighbors HTTP path"
        );
        assert!(
            !paths.contains_key("/api/neighbors/{id}"),
            "bundled gateway OpenAPI must not expose the retired node-neighbors HTTP path"
        );
        assert!(
            !paths.contains_key("/api/analysis/markdown"),
            "bundled gateway OpenAPI must not expose the retired markdown HTTP path"
        );
        assert!(
            !paths.contains_key("/api/analysis/code-ast"),
            "bundled gateway OpenAPI must not expose the retired code-ast HTTP path"
        );
        assert!(
            !paths.contains_key("/api/analysis/markdown/retrieval-arrow"),
            "bundled gateway OpenAPI must not expose the retired markdown retrieval-arrow path"
        );
        assert!(
            !paths.contains_key("/api/analysis/code-ast/retrieval-arrow"),
            "bundled gateway OpenAPI must not expose the retired code-ast retrieval-arrow path"
        );
    }
}
