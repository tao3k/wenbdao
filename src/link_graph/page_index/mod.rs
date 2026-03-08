mod builder;
mod thinning;

pub(in crate::link_graph) use builder::build_page_index_tree;
pub(in crate::link_graph) use thinning::{
    DEFAULT_PAGE_INDEX_THINNING_TOKEN_THRESHOLD, thin_page_index_tree,
};
