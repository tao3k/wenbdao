mod builder;
mod thinning;

pub(crate) use builder::build_page_index_tree;
pub(crate) use thinning::{DEFAULT_PAGE_INDEX_THINNING_TOKEN_THRESHOLD, thin_page_index_tree};
