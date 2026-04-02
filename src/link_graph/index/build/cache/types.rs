use crate::link_graph::index::LinkGraphIndex;

#[derive(Debug)]
pub(in crate::link_graph::index::build) enum CacheLookupOutcome {
    Hit(Box<LinkGraphIndex>),
    Miss(&'static str),
}
