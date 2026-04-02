//! Template rendering utilities for docs governance.

mod footer;
mod index;
mod landing;
mod links;
mod planning;
mod shared;

pub use footer::render_index_footer_with_values;
pub use index::render_package_docs_index;
pub use landing::render_section_landing_page;
pub use links::link_target;
pub use planning::{
    plan_index_footer_block_insertion, plan_index_relations_block_insertion,
    plan_index_section_link_insertion,
};
pub use shared::standard_section_specs;
