pub(crate) const COLUMN_ID: &str = "id";
pub(crate) const COLUMN_ENTITY_KIND: &str = "entity_kind";
pub(crate) const COLUMN_NAME: &str = "name";
pub(crate) const COLUMN_NAME_FOLDED: &str = "name_folded";
pub(crate) const COLUMN_QUALIFIED_NAME: &str = "qualified_name";
pub(crate) const COLUMN_QUALIFIED_NAME_FOLDED: &str = "qualified_name_folded";
pub(crate) const COLUMN_PATH: &str = "path";
pub(crate) const COLUMN_PATH_FOLDED: &str = "path_folded";
pub(crate) const COLUMN_LANGUAGE: &str = "language";
pub(crate) const COLUMN_SYMBOL_KIND: &str = "symbol_kind";
pub(crate) const COLUMN_MODULE_ID: &str = "module_id";
pub(crate) const COLUMN_SIGNATURE: &str = "signature";
pub(crate) const COLUMN_SIGNATURE_FOLDED: &str = "signature_folded";
pub(crate) const COLUMN_SUMMARY: &str = "summary";
pub(crate) const COLUMN_SUMMARY_FOLDED: &str = "summary_folded";
pub(crate) const COLUMN_RELATED_SYMBOLS_FOLDED: &str = "related_symbols_folded";
pub(crate) const COLUMN_RELATED_MODULES_FOLDED: &str = "related_modules_folded";
pub(crate) const COLUMN_LINE_START: &str = "line_start";
pub(crate) const COLUMN_LINE_END: &str = "line_end";
pub(crate) const COLUMN_AUDIT_STATUS: &str = "audit_status";
pub(crate) const COLUMN_VERIFICATION_STATE: &str = "verification_state";
pub(crate) const COLUMN_ATTRIBUTES_JSON: &str = "attributes_json";
pub(crate) const COLUMN_HIERARCHICAL_URI: &str = "hierarchical_uri";
pub(crate) const COLUMN_HIERARCHY: &str = "hierarchy";
pub(crate) const COLUMN_IMPLICIT_BACKLINKS: &str = "implicit_backlinks";
pub(crate) const COLUMN_IMPLICIT_BACKLINK_ITEMS_JSON: &str = "implicit_backlink_items_json";
pub(crate) const COLUMN_PROJECTION_PAGE_IDS: &str = "projection_page_ids";
pub(crate) const COLUMN_SALIENCY_SCORE: &str = "saliency_score";
pub(crate) const COLUMN_SEARCH_TEXT: &str = "search_text";
pub(crate) const COLUMN_HIT_JSON: &str = "hit_json";

pub(crate) const ENTITY_KIND_SYMBOL: &str = "symbol";
pub(crate) const ENTITY_KIND_MODULE: &str = "module";
pub(crate) const ENTITY_KIND_EXAMPLE: &str = "example";
pub(crate) const ENTITY_KIND_IMPORT: &str = "import";

#[derive(Debug, Clone)]
pub(crate) struct RepoEntityRow {
    pub(crate) id: String,
    pub(crate) entity_kind: String,
    pub(crate) name: String,
    pub(crate) name_folded: String,
    pub(crate) qualified_name: String,
    pub(crate) qualified_name_folded: String,
    pub(crate) path: String,
    pub(crate) path_folded: String,
    pub(crate) language: String,
    pub(crate) symbol_kind: String,
    pub(crate) module_id: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) signature_folded: String,
    pub(crate) summary: Option<String>,
    pub(crate) summary_folded: String,
    pub(crate) related_symbols_folded: String,
    pub(crate) related_modules_folded: String,
    pub(crate) line_start: Option<u32>,
    pub(crate) line_end: Option<u32>,
    pub(crate) audit_status: Option<String>,
    pub(crate) verification_state: Option<String>,
    pub(crate) attributes_json: Option<String>,
    pub(crate) hierarchical_uri: Option<String>,
    pub(crate) hierarchy: Vec<String>,
    pub(crate) implicit_backlinks: Vec<String>,
    pub(crate) implicit_backlink_items_json: Option<String>,
    pub(crate) projection_page_ids: Vec<String>,
    pub(crate) saliency_score: f64,
    pub(crate) search_text: String,
    pub(crate) hit_json: String,
}

impl RepoEntityRow {
    pub(crate) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(crate) fn update_fingerprint(&self, hasher: &mut blake3::Hasher) {
        update_string(hasher, b"id", self.id.as_str());
        update_string(hasher, b"entity_kind", self.entity_kind.as_str());
        update_string(hasher, b"name", self.name.as_str());
        update_string(hasher, b"name_folded", self.name_folded.as_str());
        update_string(hasher, b"qualified_name", self.qualified_name.as_str());
        update_string(
            hasher,
            b"qualified_name_folded",
            self.qualified_name_folded.as_str(),
        );
        update_string(hasher, b"path", self.path.as_str());
        update_string(hasher, b"path_folded", self.path_folded.as_str());
        update_string(hasher, b"language", self.language.as_str());
        update_string(hasher, b"symbol_kind", self.symbol_kind.as_str());
        update_optional_string(hasher, b"module_id", self.module_id.as_deref());
        update_optional_string(hasher, b"signature", self.signature.as_deref());
        update_string(hasher, b"signature_folded", self.signature_folded.as_str());
        update_optional_string(hasher, b"summary", self.summary.as_deref());
        update_string(hasher, b"summary_folded", self.summary_folded.as_str());
        update_string(
            hasher,
            b"related_symbols_folded",
            self.related_symbols_folded.as_str(),
        );
        update_string(
            hasher,
            b"related_modules_folded",
            self.related_modules_folded.as_str(),
        );
        update_optional_u32(hasher, b"line_start", self.line_start);
        update_optional_u32(hasher, b"line_end", self.line_end);
        update_optional_string(hasher, b"audit_status", self.audit_status.as_deref());
        update_optional_string(
            hasher,
            b"verification_state",
            self.verification_state.as_deref(),
        );
        update_optional_string(hasher, b"attributes_json", self.attributes_json.as_deref());
        update_optional_string(
            hasher,
            b"hierarchical_uri",
            self.hierarchical_uri.as_deref(),
        );
        update_string_list(hasher, b"hierarchy", self.hierarchy.as_slice());
        update_string_list(
            hasher,
            b"implicit_backlinks",
            self.implicit_backlinks.as_slice(),
        );
        update_optional_string(
            hasher,
            b"implicit_backlink_items_json",
            self.implicit_backlink_items_json.as_deref(),
        );
        update_string_list(
            hasher,
            b"projection_page_ids",
            self.projection_page_ids.as_slice(),
        );
        update_f64(hasher, b"saliency_score", self.saliency_score);
        update_string(hasher, b"search_text", self.search_text.as_str());
        update_string(hasher, b"hit_json", self.hit_json.as_str());
    }
}

fn update_string(hasher: &mut blake3::Hasher, field: &[u8], value: &str) {
    update_bytes(hasher, field, value.as_bytes());
}

fn update_optional_string(hasher: &mut blake3::Hasher, field: &[u8], value: Option<&str>) {
    begin_field(hasher, field);
    match value {
        Some(value) => {
            hasher.update(&[1]);
            update_len_prefixed_bytes(hasher, value.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn update_optional_u32(hasher: &mut blake3::Hasher, field: &[u8], value: Option<u32>) {
    begin_field(hasher, field);
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hasher.update(&value.to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

fn update_f64(hasher: &mut blake3::Hasher, field: &[u8], value: f64) {
    begin_field(hasher, field);
    hasher.update(&value.to_bits().to_le_bytes());
}

fn update_string_list(hasher: &mut blake3::Hasher, field: &[u8], values: &[String]) {
    begin_field(hasher, field);
    hasher.update(&(values.len() as u64).to_le_bytes());
    for value in values {
        update_len_prefixed_bytes(hasher, value.as_bytes());
    }
}

fn update_bytes(hasher: &mut blake3::Hasher, field: &[u8], value: &[u8]) {
    begin_field(hasher, field);
    update_len_prefixed_bytes(hasher, value);
}

fn begin_field(hasher: &mut blake3::Hasher, field: &[u8]) {
    hasher.update(field);
    hasher.update(&[0xff]);
}

fn update_len_prefixed_bytes(hasher: &mut blake3::Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
}
