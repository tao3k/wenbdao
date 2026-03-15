use schemars::JsonSchema;
use serde::Deserialize;
use xiuxian_zhenfa::{ZhenfaContext, ZhenfaError, zhenfa_tool};

use crate::link_graph::{
    Address, LinkGraphPlannedSearchPayload, LinkGraphRelatedFilter, MatchType, PageIndexNode,
    ResolveMode, resolve_node, resolve_with_indices,
};
use crate::{
    AssetRequest, LinkGraphIndex, LinkGraphSearchOptions, SkillVfsResolver, WendaoAssetHandle,
};

mod audit;
mod section_create;
mod semantic_check;
mod xml_lite;

pub use audit::{audit_search_payload, evaluate_alignment};

const DEFAULT_SEARCH_LIMIT: usize = 20;
const MAX_SEARCH_LIMIT: usize = 200;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WendaoSearchArgs {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    root_dir: Option<String>,
    #[serde(default)]
    options: Option<LinkGraphSearchOptions>,
    #[serde(default)]
    include_provisional: Option<bool>,
    #[serde(default)]
    provisional_limit: Option<usize>,
    /// Optional style anchors for CCS (Context Completeness Score) audit.
    #[serde(default)]
    anchors: Option<Vec<String>>,
}

/// Typed extension accessors for Wendao native tools.
pub trait WendaoContextExt {
    /// Resolve the injected immutable `LinkGraph` index from zhenfa context.
    ///
    /// # Errors
    /// Returns execution error when the index is not present in context.
    fn link_graph_index(&self) -> Result<std::sync::Arc<LinkGraphIndex>, ZhenfaError>;

    /// Resolve the injected semantic skill VFS resolver from zhenfa context.
    ///
    /// # Errors
    /// Returns execution error when resolver is not present in context.
    fn vfs(&self) -> Result<std::sync::Arc<SkillVfsResolver>, ZhenfaError>;

    /// Builds one skill-scoped asset request.
    ///
    /// # Errors
    /// Returns execution error when semantic URI mapping arguments are invalid.
    fn skill_asset(
        &self,
        semantic_name: &str,
        relative_path: &str,
    ) -> Result<AssetRequest, ZhenfaError>;
}

impl WendaoContextExt for ZhenfaContext {
    fn link_graph_index(&self) -> Result<std::sync::Arc<LinkGraphIndex>, ZhenfaError> {
        self.get_extension::<LinkGraphIndex>().ok_or_else(|| {
            ZhenfaError::execution("missing LinkGraphIndex in zhenfa context extensions")
        })
    }

    fn vfs(&self) -> Result<std::sync::Arc<SkillVfsResolver>, ZhenfaError> {
        self.get_extension::<SkillVfsResolver>().ok_or_else(|| {
            ZhenfaError::execution("missing SkillVfsResolver in zhenfa context extensions")
        })
    }

    fn skill_asset(
        &self,
        semantic_name: &str,
        relative_path: &str,
    ) -> Result<AssetRequest, ZhenfaError> {
        WendaoAssetHandle::skill_reference_asset(semantic_name, relative_path).map_err(|error| {
            ZhenfaError::invalid_arguments(format!(
                "invalid skill asset mapping (`{semantic_name}`, `{relative_path}`): {error}"
            ))
        })
    }
}

/// Search the Wendao graph index and return stripped XML-Lite `<hit>` records.
/// Native tool for searching the wendao graph index.
#[allow(missing_docs)]
#[zhenfa_tool(
    name = "wendao.search",
    description = "Search the Wendao graph index and return stripped XML-Lite <hit> records.",
    tool_struct = "WendaoSearchTool",
    mutation_scope = "wendao.search"
)]
pub async fn wendao_search(
    ctx: &ZhenfaContext,
    args: WendaoSearchArgs,
) -> Result<String, ZhenfaError> {
    let query = args.query.trim();
    if query.is_empty() {
        return Err(ZhenfaError::invalid_arguments(
            "`query` must be a non-empty string",
        ));
    }

    validate_root_dir_argument(args.root_dir.as_deref())?;
    let options = args.options.unwrap_or_default();
    let index = ctx.link_graph_index()?;
    let limit = normalize_limit(args.limit);

    // First-pass search
    let payload = index.search_planned_payload_with_agentic(
        query,
        limit,
        options.clone(),
        args.include_provisional,
        args.provisional_limit,
    );

    // Apply CCS audit and compensation loop if anchors provided
    if let Some(anchors) = args.anchors {
        if !anchors.is_empty() {
            let evidence: Vec<String> = payload
                .results
                .iter()
                .flat_map(|hit| vec![hit.stem.clone(), hit.title.clone()])
                .collect();

            let audit_result = audit::audit_search_payload(&evidence, &anchors);

            // Apply compensation if CCS < threshold
            let (mut final_payload, compensated) = if let Some(comp) = &audit_result.compensation {
                let mut compensated_options = options.clone();
                // Expand max_distance for broader retrieval
                if let Some(ref mut related) = compensated_options.filters.related {
                    related.max_distance =
                        Some(related.max_distance.unwrap_or(2) + comp.max_distance_delta);
                } else {
                    compensated_options.filters.related = Some(LinkGraphRelatedFilter {
                        max_distance: Some(comp.max_distance_delta + 2),
                        ..Default::default()
                    });
                }

                // Re-search with compensated parameters
                let compensated_payload = index.search_planned_payload_with_agentic(
                    query,
                    limit,
                    compensated_options,
                    args.include_provisional,
                    args.provisional_limit,
                );
                (compensated_payload, true)
            } else {
                (payload, false)
            };

            use crate::link_graph::LinkGraphCcsAudit;
            final_payload.ccs_audit = Some(LinkGraphCcsAudit {
                ccs_score: audit_result.ccs_score,
                passed: audit_result.passed,
                compensated,
                missing_anchors: audit_result.missing_anchors,
            });

            return Ok(xml_lite::render_xml_lite(&final_payload));
        }
    }

    Ok(xml_lite::render_xml_lite(&payload))
}

/// Render one planned payload into XML-Lite hit rows.
///
/// This is a thin public adapter over native XML-Lite rendering logic, used by
/// integration tests and tool-facing formatting call sites.
#[must_use]
pub fn render_xml_lite_hits(payload: &LinkGraphPlannedSearchPayload) -> String {
    xml_lite::render_xml_lite(payload)
}

fn normalize_limit(raw: Option<usize>) -> usize {
    raw.unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT)
}

fn validate_root_dir_argument(root_dir: Option<&str>) -> Result<(), ZhenfaError> {
    if let Some(value) = root_dir {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ZhenfaError::invalid_arguments(
                "`root_dir` must be non-empty when provided",
            ));
        }
    }
    Ok(())
}

// ============================================================================
// Semantic Addressing Tools (Triple-A Protocol)
// ============================================================================

/// Arguments for semantic section reading via Triple-A addressing.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WendaoSemanticReadArgs {
    /// Document stem or ID (e.g., "README" or "docs/architecture").
    doc: String,
    /// Semantic address using Triple-A protocol:
    /// - `#id` - Explicit anchor ID (e.g., `#arch-v1`)
    /// - `/path/to/heading` - Structural path through heading hierarchy
    /// - `@hash` - Content hash for self-healing lookup
    address: String,
    /// Include surrounding context (parent section content).
    #[serde(default)]
    include_context: Option<bool>,
    /// Enable fuzzy path matching (allows path drift tolerance).
    #[serde(default)]
    fuzzy: Option<bool>,
}

/// Read a section from a document using semantic addressing (Triple-A protocol).
///
/// The address parameter supports three formats:
/// - `#anchor-id` - Resolve by explicit `:ID:` property drawer attribute
/// - `/Heading/Subheading` - Resolve by structural heading path
/// - `@content-hash` - Resolve by Blake3 content fingerprint
///
/// Resolution follows the Triple-A protocol: ID → Path → Hash fallback.
/// When `fuzzy` is enabled, path drift tolerance allows approximate matches.
#[allow(missing_docs)]
#[zhenfa_tool(
    name = "wendao.semantic_read",
    description = "Read a section from a document using semantic addressing (Triple-A protocol: #id, /path, or @hash).",
    tool_struct = "WendaoSemanticReadTool",
    mutation_scope = "wendao.semantic_read"
)]
pub async fn wendao_semantic_read(
    ctx: &ZhenfaContext,
    args: WendaoSemanticReadArgs,
) -> Result<String, ZhenfaError> {
    let address = Address::parse(&args.address).ok_or_else(|| {
        ZhenfaError::invalid_arguments(format!(
            "invalid address format: '{}'. Use #id, /path/to/heading, or @hash",
            args.address
        ))
    })?;

    let index = ctx.link_graph_index()?;
    let doc_id = index.resolve_doc_id_pub(&args.doc).ok_or_else(|| {
        ZhenfaError::invalid_arguments(format!("document not found: '{}'", args.doc))
    })?;

    // Build dual indices for enhanced resolution
    let registry = index.build_registry_index();
    let topology = index.build_topology_index();

    // Determine resolution mode based on fuzzy flag
    let mode = if args.fuzzy.unwrap_or(false) {
        ResolveMode::Discover {
            fuzzy: true,
            max_results: 5,
        }
    } else {
        ResolveMode::Anchor
    };

    // Try enhanced resolution first
    let (node, resolved_path, resolved_id, match_type, similarity) =
        match resolve_with_indices(&registry, &topology, &address, doc_id, mode) {
            Ok(enhanced) => (
                enhanced.node,
                enhanced.resolved_path,
                enhanced.resolved_id,
                enhanced.match_type,
                enhanced.similarity,
            ),
            Err(_) => {
                // Fallback to legacy resolution
                let trees = index.all_page_index_trees();
                let resolved = resolve_node(trees, &address, doc_id).ok_or_else(|| {
                    ZhenfaError::execution(format!(
                        "address '{}' not found in document '{}'",
                        args.address, args.doc
                    ))
                })?;
                let path = resolved.node.metadata.structural_path.clone();
                let id = resolved.node.metadata.attributes.get("ID").cloned();
                (
                    resolved.node,
                    path,
                    id,
                    MatchType::Exact,
                    1.0,
                )
            }
        };

    // Read document content via index root
    let doc_path = index.doc_path(&args.doc).ok_or_else(|| {
        ZhenfaError::execution(format!("document path not found: '{}'", args.doc))
    })?;
    let root = index.root();
    let full_path = root.join(doc_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| {
        ZhenfaError::execution(format!("failed to read document '{}': {}", doc_path, e))
    })?;

    // Extract section content using byte range
    let (byte_start, byte_end) = node.metadata.byte_range.ok_or_else(|| {
        ZhenfaError::execution(format!(
            "section '{}' has no byte range information",
            node.node_id
        ))
    })?;

    let section_content = &content[byte_start..byte_end];
    let include_context = args.include_context.unwrap_or(false);

    // Build enhanced response with resolution metadata
    let mut response = format!(
        "<section node_id=\"{}\" title=\"{}\" address=\"{}\">\n",
        node.node_id,
        node.title,
        address.to_display_string()
    );

    // Add resolution metadata
    response.push_str("  <resolution>\n");
    response.push_str(&format!(
        "    <resolved_path>{}</resolved_path>\n",
        resolved_path.join("/")
    ));
    if let Some(ref id) = resolved_id {
        response.push_str(&format!("    <resolved_id>#{}</resolved_id>\n", id));
    }
    response.push_str(&format!(
        "    <match_type>{}</match_type>\n",
        match_type_to_string(match_type)
    ));
    response.push_str(&format!("    <similarity>{:.2}</similarity>\n", similarity));
    response.push_str("  </resolution>\n");

    response.push_str(&format!(
        "  <metadata line_range=\"{}-{}\" byte_range=\"{}-{}\" token_count=\"{}\"/>\n",
        node.metadata.line_range.0,
        node.metadata.line_range.1,
        byte_start,
        byte_end,
        node.metadata.token_count
    ));

    if let Some(ref hash) = node.metadata.content_hash {
        response.push_str(&format!("  <content_hash>{}</content_hash>\n", hash));
    }

    if include_context {
        // Include parent context
        if let Some(parent_id) = &node.parent_id {
            let trees = index.all_page_index_trees();
            if let Some(parent_nodes) = trees.get(doc_id) {
                if let Some(parent) = find_node_by_id(parent_nodes, parent_id) {
                    if let Some((p_start, p_end)) = parent.metadata.byte_range {
                        let parent_content = &content[p_start..p_end];
                        response.push_str("  <parent_context>\n");
                        response.push_str(&format!("    <title>{}</title>\n", parent.title));
                        response.push_str("    <content><![CDATA[\n");
                        response.push_str(parent_content);
                        response.push_str("\n    ]]></content>\n");
                        response.push_str("  </parent_context>\n");
                    }
                }
            }
        }
    }

    response.push_str("  <content><![CDATA[\n");
    response.push_str(section_content);
    response.push_str("\n  ]]></content>\n");
    response.push_str("</section>\n");

    Ok(response)
}

/// Convert MatchType to human-readable string.
fn match_type_to_string(match_type: MatchType) -> &'static str {
    match match_type {
        MatchType::Exact => "exact",
        MatchType::Suffix => "suffix",
        MatchType::TitleSubstring => "title_substring",
        MatchType::HashFallback => "hash_fallback",
        MatchType::CaseInsensitive => "case_insensitive",
    }
}

/// Arguments for semantic section editing via Triple-A addressing.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WendaoSemanticEditArgs {
    /// Document stem or ID (e.g., "README" or "docs/architecture").
    doc: String,
    /// Semantic address using Triple-A protocol.
    address: String,
    /// New content for the section.
    new_content: String,
    /// Verify content hash before modification (prevents concurrent edit conflicts).
    #[serde(default)]
    verify_hash: Option<bool>,
    /// Create the section if it doesn't exist (only for path-based addresses).
    #[serde(default)]
    create_if_missing: Option<bool>,
    /// Generate a `:ID: <uuid>` property drawer for newly created sections.
    /// Only applies when create_if_missing is true.
    #[serde(default)]
    generate_id: Option<bool>,
    /// Optional prefix for generated IDs (e.g., "arch" -> ":ID: arch-abc123").
    #[serde(default)]
    id_prefix: Option<String>,
}

/// Edit a section in a document using semantic addressing (Triple-A protocol).
///
/// This tool performs atomic byte-range modifications, preserving document formatting
/// and avoiding the "format violence" of full-tree re-rendering.
///
/// The verify_hash option enables optimistic concurrency control:
/// the tool will fail if the section's content hash has changed since reading.
#[allow(missing_docs)]
#[zhenfa_tool(
    name = "wendao.semantic_edit",
    description = "Edit a section in a document using semantic addressing with atomic byte-range modification.",
    tool_struct = "WendaoSemanticEditTool",
    mutation_scope = "wendao.semantic_edit"
)]
pub async fn wendao_semantic_edit(
    ctx: &ZhenfaContext,
    args: WendaoSemanticEditArgs,
) -> Result<String, ZhenfaError> {
    use crate::link_graph::{ModificationError, replace_byte_range};

    let address = Address::parse(&args.address).ok_or_else(|| {
        ZhenfaError::invalid_arguments(format!(
            "invalid address format: '{}'. Use #id, /path/to/heading, or @hash",
            args.address
        ))
    })?;

    let index = ctx.link_graph_index()?;
    let doc_id = index.resolve_doc_id_pub(&args.doc).ok_or_else(|| {
        ZhenfaError::invalid_arguments(format!("document not found: '{}'", args.doc))
    })?;

    let trees = index.all_page_index_trees();
    let resolved = resolve_node(trees, &address, doc_id);

    // Read document content (needed for both edit and create)
    let doc_path = index.doc_path(&args.doc).ok_or_else(|| {
        ZhenfaError::execution(format!("document path not found: '{}'", args.doc))
    })?;

    let root = index.root();
    let full_path = root.join(doc_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| {
        ZhenfaError::execution(format!("failed to read document '{}': {}", doc_path, e))
    })?;

    // Handle missing section with create_if_missing
    let (new_content, section_title, byte_start, byte_end, new_hash, sibling_context) =
        match resolved {
            Some(r) => {
                // Existing section - perform atomic modification
                let node = &r.node;
                let (byte_start, byte_end) = node.metadata.byte_range.ok_or_else(|| {
                    ZhenfaError::execution(format!(
                        "section '{}' has no byte range information",
                        node.node_id
                    ))
                })?;

                let expected_hash = if args.verify_hash.unwrap_or(true) {
                    node.metadata.content_hash.as_deref()
                } else {
                    None
                };

                let result = replace_byte_range(&content, byte_start, byte_end, &args.new_content, expected_hash)
                .map_err(|e| match e {
                    ModificationError::ByteRangeOutOfBounds { start, end, content_len } => {
                        ZhenfaError::execution(format!(
                            "byte range out of bounds: {}-{} (content length: {})",
                            start, end, content_len
                        ))
                    }
                    ModificationError::HashMismatch { expected, actual } => {
                        ZhenfaError::execution(format!(
                            "content hash mismatch: expected '{}', got '{}'. The section may have been modified since you read it.",
                            expected, actual
                        ))
                    }
                    ModificationError::NoByteRange => {
                        ZhenfaError::execution("section has no byte range information")
                    }
                })?;

                (
                    result.new_content,
                    node.title.clone(),
                    byte_start,
                    byte_end,
                    result.new_hash,
                    None,
                )
            }
            None => {
                // Section not found - check if create_if_missing is enabled
                if !args.create_if_missing.unwrap_or(false) {
                    return Err(ZhenfaError::execution(format!(
                        "address '{}' not found in document '{}'",
                        args.address, args.doc
                    )));
                }

                // Only Path-type addresses support create_if_missing
                let path_components = match &address {
                    Address::Path(components) => components.clone(),
                    Address::Id(_) => {
                        return Err(ZhenfaError::invalid_arguments(
                            "create_if_missing requires a path-based address (e.g., /Section/Subsection). ID-based addresses cannot be auto-created.",
                        ));
                    }
                    Address::Hash(_) => {
                        return Err(ZhenfaError::invalid_arguments(
                            "create_if_missing requires a path-based address (e.g., /Section/Subsection). Hash-based addresses cannot be auto-created.",
                        ));
                    }
                    Address::Block { .. } => {
                        return Err(ZhenfaError::invalid_arguments(
                            "create_if_missing requires a path-based address (e.g., /Section/Subsection). Block-based addresses cannot be auto-created.",
                        ));
                    }
                };

                // Find insertion point
                let insertion_info =
                    section_create::find_insertion_point(&content, &path_components);

                // Build new section content with optional ID generation
                let build_options = section_create::BuildSectionOptions {
                    generate_id: args.generate_id.unwrap_or(false),
                    id_prefix: args.id_prefix.clone(),
                };

                let sections_content = section_create::build_new_sections_content_with_options(
                    &insertion_info.remaining_path,
                    insertion_info.start_level,
                    &args.new_content,
                    &build_options,
                );

                // Build sibling context string for response
                let sibling_context = format_sibling_context(&insertion_info);

                // Insert at determined position
                let mut new_doc = String::with_capacity(content.len() + sections_content.len());
                new_doc.push_str(&content[..insertion_info.insertion_byte]);
                if insertion_info.insertion_byte > 0
                    && !content[..insertion_info.insertion_byte].ends_with('\n')
                {
                    new_doc.push('\n');
                }
                new_doc.push_str(&sections_content);
                new_doc.push_str(&content[insertion_info.insertion_byte..]);

                let new_hash = section_create::compute_content_hash(&sections_content);
                let section_title = path_components
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "Section".to_string());

                let sibling_ctx: Option<String> = Some(sibling_context);
                (
                    new_doc,
                    section_title,
                    insertion_info.insertion_byte,
                    insertion_info.insertion_byte + sections_content.len(),
                    new_hash,
                    sibling_ctx,
                )
            }
        };

    // Write back to file
    std::fs::write(&full_path, &new_content).map_err(|e| {
        ZhenfaError::execution(format!("failed to write document '{}': {}", doc_path, e))
    })?;

    // Build response
    let sibling_xml: String = sibling_context.unwrap_or_default();
    Ok(format!(
        "<edit_result>\n\
         \x20  <document>{}</document>\n\
         \x20  <section title=\"{}\"/>\n\
         \x20  <address original=\"{}\"/>\n\
         \x20  <byte_range start=\"{}\" end=\"{}\"/>\n\
         \x20  <new_hash>{}</new_hash>{}\n\
         </edit_result>\n",
        args.doc, section_title, args.address, byte_start, byte_end, new_hash, sibling_xml
    ))
}

/// Find a node by its node_id in a tree - used by semantic_read for context lookup.
fn find_node_by_id(nodes: &[PageIndexNode], target_id: &str) -> Option<PageIndexNode> {
    for node in nodes {
        if node.node_id == target_id {
            return Some(node.clone());
        }
        if let Some(found) = find_node_by_id(&node.children, target_id) {
            return Some(found);
        }
    }
    None
}

/// Format sibling context as XML for the response.
fn format_sibling_context(info: &section_create::InsertionInfo) -> String {
    let mut result = String::new();

    if let Some(ref prev) = info.prev_sibling {
        result.push_str(&format!(
            "\n         \x20  <prev_sibling title=\"{}\">{}</prev_sibling>",
            prev.title,
            if prev.preview.is_empty() {
                "..."
            } else {
                &prev.preview
            }
        ));
    }

    if let Some(ref next) = info.next_sibling {
        result.push_str(&format!(
            "\n         \x20  <next_sibling title=\"{}\">{}</next_sibling>",
            next.title,
            if next.preview.is_empty() {
                "..."
            } else {
                &next.preview
            }
        ));
    }

    result
}
