//! Shared fixture materialization for link-graph hybrid retrieval tests.

use std::io;

use xiuxian_wendao::LinkGraphIndex;

use super::link_graph_fixture_tree::materialize_link_graph_fixture;

pub(crate) const EXPECTED_FIXTURE_ROOT: &str = "link_graph/hybrid/expected";

pub(crate) struct HybridFixture {
    _temp_dir: tempfile::TempDir,
    index: LinkGraphIndex,
}

impl HybridFixture {
    pub(crate) fn build() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = materialize_link_graph_fixture("link_graph/hybrid/input")?;
        let index = LinkGraphIndex::build(temp_dir.path()).map_err(|error| error.clone())?;
        Ok(Self {
            _temp_dir: temp_dir,
            index,
        })
    }

    pub(crate) fn index(&self) -> &LinkGraphIndex {
        &self.index
    }

    pub(crate) fn alpha_leaf_anchor_id(&self) -> Result<String, Box<dyn std::error::Error>> {
        let roots = self
            .index
            .page_index("alpha")
            .ok_or_else(|| io::Error::other("missing alpha page index"))?;
        roots
            .first()
            .and_then(|root| root.children.first())
            .and_then(|details| details.children.first())
            .map(|leaf| leaf.node_id.clone())
            .ok_or_else(|| io::Error::other("missing alpha leaf anchor").into())
    }
}
