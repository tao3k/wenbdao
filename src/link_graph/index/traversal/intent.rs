use std::fs;

use super::super::LinkGraphIndex;
use crate::link_graph::parser::parse_note;

impl LinkGraphIndex {
    /// Return raw markdown note-link targets and attachment targets for one indexed document.
    ///
    /// This reparses the backing markdown file so callers can inspect unresolved intent targets
    /// that may not materialize as graph edges yet.
    pub(crate) fn intent_targets(
        &self,
        stem_or_id: &str,
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let doc = self
            .resolve_doc(stem_or_id)
            .ok_or_else(|| format!("link graph document `{stem_or_id}` not found"))?;
        let source_path = self.root.join(&doc.path);
        let content = fs::read_to_string(&source_path).map_err(|error| {
            format!(
                "read link graph document `{}`: {error}",
                source_path.display()
            )
        })?;
        let parsed =
            parse_note(source_path.as_path(), &self.root, content.as_str()).ok_or_else(|| {
                format!(
                    "parse link graph document `{}` as markdown note",
                    source_path.display()
                )
            })?;
        Ok((parsed.link_targets, parsed.attachment_targets))
    }
}
