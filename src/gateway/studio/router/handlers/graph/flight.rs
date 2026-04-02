use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;
use xiuxian_vector::{
    LanceBooleanArray, LanceDataType, LanceField, LanceInt32Array, LanceRecordBatch, LanceSchema,
    LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    GraphNeighborsFlightRouteProvider, GraphNeighborsFlightRouteResponse,
};

use super::service::run_graph_neighbors;
use super::shared::{normalize_hops, normalize_limit, parse_direction};
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{GraphLink, GraphNeighborsResponse, GraphNode};

/// Studio-backed Flight provider for the semantic `/graph/neighbors` route.
#[derive(Clone)]
pub(crate) struct StudioGraphNeighborsFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioGraphNeighborsFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioGraphNeighborsFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioGraphNeighborsFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl GraphNeighborsFlightRouteProvider for StudioGraphNeighborsFlightRouteProvider {
    async fn graph_neighbors_batch(
        &self,
        node_id: &str,
        direction: &str,
        hops: usize,
        limit: usize,
    ) -> Result<GraphNeighborsFlightRouteResponse, Status> {
        load_graph_neighbors_flight_response(
            Arc::clone(&self.state),
            node_id,
            direction,
            hops,
            limit,
        )
        .await
        .map_err(studio_api_error_to_tonic_status)
    }
}

pub(crate) async fn build_graph_neighbors_response(
    state: Arc<GatewayState>,
    node_id: &str,
    direction: &str,
    hops: usize,
    limit: usize,
) -> Result<GraphNeighborsResponse, StudioApiError> {
    let node_id = node_id.trim();
    if node_id.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_NODE_ID",
            "`nodeId` is required",
        ));
    }
    Ok(run_graph_neighbors(
        state,
        node_id,
        parse_direction(Some(direction)),
        normalize_hops(Some(hops)),
        normalize_limit(Some(limit)),
    )
    .await?)
}

pub(crate) async fn load_graph_neighbors_flight_response(
    state: Arc<GatewayState>,
    node_id: &str,
    direction: &str,
    hops: usize,
    limit: usize,
) -> Result<GraphNeighborsFlightRouteResponse, StudioApiError> {
    let response = build_graph_neighbors_response(state, node_id, direction, hops, limit).await?;
    let batch = graph_neighbors_response_batch(&response).map_err(|error| {
        StudioApiError::internal(
            "GRAPH_NEIGHBORS_FLIGHT_BATCH_FAILED",
            "Failed to materialize graph neighbors through the Flight-backed provider",
            Some(error),
        )
    })?;
    Ok(GraphNeighborsFlightRouteResponse::new(batch))
}

pub(crate) fn graph_neighbors_response_batch(
    response: &GraphNeighborsResponse,
) -> Result<LanceRecordBatch, String> {
    let node_rows = response.nodes.iter().map(FlightGraphRow::from_node);
    let link_rows = response.links.iter().map(FlightGraphRow::from_link);
    let rows = node_rows.chain(link_rows).collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("rowType", LanceDataType::Utf8, false),
            LanceField::new("nodeId", LanceDataType::Utf8, true),
            LanceField::new("nodeLabel", LanceDataType::Utf8, true),
            LanceField::new("nodePath", LanceDataType::Utf8, true),
            LanceField::new("nodeType", LanceDataType::Utf8, true),
            LanceField::new("nodeIsCenter", LanceDataType::Boolean, true),
            LanceField::new("nodeDistance", LanceDataType::Int32, true),
            LanceField::new("navigationPath", LanceDataType::Utf8, true),
            LanceField::new("navigationCategory", LanceDataType::Utf8, true),
            LanceField::new("navigationProjectName", LanceDataType::Utf8, true),
            LanceField::new("navigationRootLabel", LanceDataType::Utf8, true),
            LanceField::new("navigationLine", LanceDataType::Int32, true),
            LanceField::new("navigationLineEnd", LanceDataType::Int32, true),
            LanceField::new("navigationColumn", LanceDataType::Int32, true),
            LanceField::new("linkSource", LanceDataType::Utf8, true),
            LanceField::new("linkTarget", LanceDataType::Utf8, true),
            LanceField::new("linkDirection", LanceDataType::Utf8, true),
            LanceField::new("linkDistance", LanceDataType::Int32, true),
        ])),
        vec![
            Arc::new(LanceStringArray::from(
                rows.iter().map(|row| row.row_type).collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.node_id.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.node_label.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.node_path.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.node_type.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceBooleanArray::from(
                rows.iter()
                    .map(|row| row.node_is_center)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceInt32Array::from(
                rows.iter()
                    .map(|row| row.node_distance.map(usize_to_i32).transpose())
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.navigation_path.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.navigation_category.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.navigation_project_name.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.navigation_root_label.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceInt32Array::from(
                rows.iter()
                    .map(|row| row.navigation_line.map(usize_to_i32).transpose())
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(LanceInt32Array::from(
                rows.iter()
                    .map(|row| row.navigation_line_end.map(usize_to_i32).transpose())
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(LanceInt32Array::from(
                rows.iter()
                    .map(|row| row.navigation_column.map(usize_to_i32).transpose())
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.link_source.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.link_target.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                rows.iter()
                    .map(|row| row.link_direction.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceInt32Array::from(
                rows.iter()
                    .map(|row| row.link_distance.map(usize_to_i32).transpose())
                    .collect::<Result<Vec<_>, _>>()?,
            )),
        ],
    )
    .map_err(|error| format!("failed to build graph-neighbors Flight batch: {error}"))
}

#[derive(Debug, Clone)]
struct FlightGraphRow {
    row_type: &'static str,
    node_id: Option<String>,
    node_label: Option<String>,
    node_path: Option<String>,
    node_type: Option<String>,
    node_is_center: Option<bool>,
    node_distance: Option<usize>,
    navigation_path: Option<String>,
    navigation_category: Option<String>,
    navigation_project_name: Option<String>,
    navigation_root_label: Option<String>,
    navigation_line: Option<usize>,
    navigation_line_end: Option<usize>,
    navigation_column: Option<usize>,
    link_source: Option<String>,
    link_target: Option<String>,
    link_direction: Option<String>,
    link_distance: Option<usize>,
}

impl FlightGraphRow {
    fn from_node(node: &GraphNode) -> Self {
        Self {
            row_type: "node",
            node_id: Some(node.id.clone()),
            node_label: Some(node.label.clone()),
            node_path: Some(node.path.clone()),
            node_type: Some(node.node_type.clone()),
            node_is_center: Some(node.is_center),
            node_distance: Some(node.distance),
            navigation_path: node
                .navigation_target
                .as_ref()
                .map(|target| target.path.clone()),
            navigation_category: node
                .navigation_target
                .as_ref()
                .map(|target| target.category.clone()),
            navigation_project_name: node
                .navigation_target
                .as_ref()
                .and_then(|target| target.project_name.clone()),
            navigation_root_label: node
                .navigation_target
                .as_ref()
                .and_then(|target| target.root_label.clone()),
            navigation_line: node
                .navigation_target
                .as_ref()
                .and_then(|target| target.line),
            navigation_line_end: node
                .navigation_target
                .as_ref()
                .and_then(|target| target.line_end),
            navigation_column: node
                .navigation_target
                .as_ref()
                .and_then(|target| target.column),
            link_source: None,
            link_target: None,
            link_direction: None,
            link_distance: None,
        }
    }

    fn from_link(link: &GraphLink) -> Self {
        Self {
            row_type: "link",
            node_id: None,
            node_label: None,
            node_path: None,
            node_type: None,
            node_is_center: None,
            node_distance: None,
            navigation_path: None,
            navigation_category: None,
            navigation_project_name: None,
            navigation_root_label: None,
            navigation_line: None,
            navigation_line_end: None,
            navigation_column: None,
            link_source: Some(link.source.clone()),
            link_target: Some(link.target.clone()),
            link_direction: Some(link.direction.clone()),
            link_distance: Some(link.distance),
        }
    }
}

fn usize_to_i32(value: usize) -> Result<i32, String> {
    i32::try_from(value)
        .map_err(|error| format!("failed to represent graph-neighbors position: {error}"))
}

fn studio_api_error_to_tonic_status(error: StudioApiError) -> Status {
    match error.status() {
        axum::http::StatusCode::BAD_REQUEST => Status::invalid_argument(error.error.message),
        axum::http::StatusCode::NOT_FOUND => Status::not_found(error.error.message),
        axum::http::StatusCode::CONFLICT => Status::failed_precondition(error.error.message),
        _ => Status::internal(error.error.message),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use xiuxian_vector::LanceStringArray;

    use super::{graph_neighbors_response_batch, load_graph_neighbors_flight_response};
    use crate::gateway::studio::router::handlers::graph::tests::build_fixture;

    #[tokio::test]
    async fn load_graph_neighbors_flight_response_materializes_node_and_link_rows() {
        let fixture = build_fixture(&[
            ("docs/alpha.md", "# Alpha\n\nSee [[beta]].\n"),
            ("docs/beta.md", "# Beta\n\nBody.\n"),
        ]);

        let response = load_graph_neighbors_flight_response(
            Arc::clone(&fixture.state),
            "kernel/docs/alpha.md",
            "both",
            1,
            20,
        )
        .await
        .unwrap_or_else(|error| panic!("load graph-neighbors Flight response: {error:?}"));

        let row_types = response
            .batch
            .column_by_name("rowType")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .unwrap_or_else(|| panic!("rowType column should decode as Utf8"));
        let navigation_project_name = response
            .batch
            .column_by_name("navigationProjectName")
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
            .unwrap_or_else(|| panic!("navigationProjectName column should decode as Utf8"));

        assert!(response.app_metadata.is_empty());
        assert_eq!(row_types.value(0), "node");
        assert!(
            row_types.iter().flatten().any(|value| value == "link"),
            "expected one link row in graph-neighbors Flight batch",
        );
        assert_eq!(navigation_project_name.value(0), "kernel");
    }

    #[test]
    fn graph_neighbors_response_batch_preserves_navigation_target_fields() {
        let response = crate::gateway::studio::types::GraphNeighborsResponse {
            center: crate::gateway::studio::types::GraphNode {
                id: "kernel/docs/index.md".to_string(),
                label: "Index".to_string(),
                path: "kernel/docs/index.md".to_string(),
                navigation_target: Some(crate::gateway::studio::types::StudioNavigationTarget {
                    path: "kernel/docs/index.md".to_string(),
                    category: "doc".to_string(),
                    project_name: Some("kernel".to_string()),
                    root_label: Some("project".to_string()),
                    line: Some(7),
                    line_end: Some(9),
                    column: Some(3),
                }),
                node_type: "doc".to_string(),
                is_center: true,
                distance: 0,
            },
            nodes: vec![crate::gateway::studio::types::GraphNode {
                id: "kernel/docs/index.md".to_string(),
                label: "Index".to_string(),
                path: "kernel/docs/index.md".to_string(),
                navigation_target: Some(crate::gateway::studio::types::StudioNavigationTarget {
                    path: "kernel/docs/index.md".to_string(),
                    category: "doc".to_string(),
                    project_name: Some("kernel".to_string()),
                    root_label: Some("project".to_string()),
                    line: Some(7),
                    line_end: Some(9),
                    column: Some(3),
                }),
                node_type: "doc".to_string(),
                is_center: true,
                distance: 0,
            }],
            links: vec![crate::gateway::studio::types::GraphLink {
                source: "kernel/docs/index.md".to_string(),
                target: "kernel/docs/child.md".to_string(),
                direction: "outgoing".to_string(),
                distance: 1,
            }],
            total_nodes: 1,
            total_links: 1,
        };

        let batch = graph_neighbors_response_batch(&response)
            .unwrap_or_else(|error| panic!("graph-neighbors Flight batch: {error}"));
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 18);
    }
}
