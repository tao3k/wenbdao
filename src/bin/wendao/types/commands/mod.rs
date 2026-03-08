use clap::Subcommand;

mod agentic;
mod attachments;
mod graph;
mod hmas;
mod saliency;
mod search;

pub(crate) use agentic::AgenticCommand;
pub(crate) use attachments::AttachmentsArgs;
pub(crate) use graph::{
    MetadataArgs, NeighborsArgs, PageIndexArgs, RelatedArgs, ResolveArgs, TocArgs,
};
pub(crate) use hmas::HmasCommand;
pub(crate) use saliency::SaliencyCommand;
pub(crate) use search::SearchArgs;

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Search notes by title/path/stem/tags.
    Search(Box<SearchArgs>),
    /// Return link-graph stats.
    Stats,
    /// Return table-of-contents rows.
    Toc(TocArgs),
    /// Return neighbors for a note.
    Neighbors(NeighborsArgs),
    /// Return related notes for a note.
    Related(RelatedArgs),
    /// Return metadata for a note.
    Metadata(MetadataArgs),
    /// Return hierarchical `PageIndex` roots for a note.
    PageIndex(PageIndexArgs),
    /// Resolve ambiguous stem/id/path input into canonical candidates.
    Resolve(ResolveArgs),
    /// Search extracted local attachments by query/extension/type.
    Attachments(AttachmentsArgs),
    /// Read/update `GraphMem` saliency state.
    Saliency {
        #[command(subcommand)]
        command: SaliencyCommand,
    },
    /// Validate HMAS markdown blackboard protocol blocks.
    Hmas {
        #[command(subcommand)]
        command: HmasCommand,
    },
    /// Manage agentic suggested-link proposals and decision audit rows.
    Agentic {
        #[command(subcommand)]
        command: AgenticCommand,
    },
}
