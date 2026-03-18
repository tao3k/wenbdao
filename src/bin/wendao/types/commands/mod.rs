use clap::Subcommand;

mod agentic;
mod attachments;
mod audit;
mod fix;
#[cfg(feature = "zhenfa-router")]
mod gateway;
mod graph;
mod hmas;
mod saliency;
mod search;
mod sentinel;

pub(crate) use agentic::AgenticCommand;
pub(crate) use attachments::AttachmentsArgs;
pub(crate) use audit::AuditArgs;
pub(crate) use fix::FixArgs;
#[cfg(feature = "zhenfa-router")]
pub(crate) use gateway::{GatewayArgs, GatewayCommand, GatewayStartArgs};
pub(crate) use graph::{MetadataArgs, NeighborsArgs, RelatedArgs, ResolveArgs, TocArgs};
pub(crate) use hmas::HmasCommand;
pub(crate) use saliency::SaliencyCommand;
pub(crate) use search::SearchArgs;
pub(crate) use sentinel::{SentinelArgs, SentinelCommand, SentinelWatchArgs};

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Search notes by title/path/stem/tags.
    Search(Box<SearchArgs>),
    /// Audit documents for structural and logical consistency.
    Audit(AuditArgs),
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
    /// Apply automated fixes to documents based on semantic audit issues.
    ///
    /// Uses byte-precise surgical fixes with CAS verification for safe,
    /// atomic modifications. Run with --dry-run to preview changes.
    Fix(FixArgs),
    /// Start the Wendao API gateway server with webhook notifications.
    #[cfg(feature = "zhenfa-router")]
    Gateway(GatewayArgs),
    /// Start the Project Sentinel file observer for real-time semantic drift detection.
    Sentinel(SentinelArgs),
}
