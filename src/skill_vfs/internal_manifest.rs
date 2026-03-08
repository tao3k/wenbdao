use anyhow::{Context, Result};
use xiuxian_skills::{
    InternalSkillManifest, InternalSkillManifestScan, parse_internal_skill_manifest_seed,
};

use super::SkillVfsResolver;

impl SkillVfsResolver {
    /// Load and validate one internal skill manifest through the Wendao resolver.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest URI cannot be resolved, the TOML cannot be parsed,
    /// mandatory fields are empty, the structured description is invalid, or any bound
    /// `wendao://` / `$wendao://` resource cannot be resolved by this resolver.
    pub fn load_internal_skill_manifest(
        &self,
        manifest_uri: &str,
    ) -> Result<InternalSkillManifest> {
        let source_path = self
            .resolve_path(manifest_uri)
            .with_context(|| format!("resolve internal manifest URI `{manifest_uri}`"))?;
        self.load_internal_skill_manifest_with_source_path(manifest_uri, source_path)
    }

    fn load_internal_skill_manifest_with_source_path(
        &self,
        manifest_uri: &str,
        source_path: std::path::PathBuf,
    ) -> Result<InternalSkillManifest> {
        let content = self
            .read_utf8_shared(manifest_uri)
            .with_context(|| format!("read internal manifest URI `{manifest_uri}`"))?;
        let seed = parse_internal_skill_manifest_seed(content.as_ref()).map_err(|error| {
            anyhow::anyhow!("parse internal manifest URI `{manifest_uri}`: {error}")
        })?;

        let qianhuan_background = validate_bound_wendao_resource(
            self,
            seed.qianhuan_background.as_deref(),
            "qianhuan.background",
        )?;

        let flow_definition = validate_bound_wendao_resource(
            self,
            seed.flow_definition.as_deref(),
            "workflow.flow_definition",
        )?;

        Ok(InternalSkillManifest {
            manifest_id: seed.manifest_id,
            tool_name: seed.tool_name,
            description: seed.description,
            workflow_type: seed.workflow_type,
            internal_id: seed.internal_id,
            metadata: seed.metadata,
            annotations: seed.annotations,
            source_path,
            qianhuan_background,
            flow_definition,
        })
    }

    /// Discover and validate every mounted internal skill manifest.
    #[must_use]
    pub fn scan_internal_manifests(&self) -> InternalSkillManifestScan {
        let manifest_uris = self.list_internal_manifest_uris();
        let mut scan = InternalSkillManifestScan {
            discovered_paths: Vec::with_capacity(manifest_uris.len()),
            manifests: Vec::with_capacity(manifest_uris.len()),
            issues: Vec::new(),
        };

        for manifest_uri in manifest_uris {
            let source_path = match self.resolve_path(manifest_uri.as_str()) {
                Ok(path) => {
                    scan.discovered_paths.push(path.clone());
                    path
                }
                Err(error) => {
                    scan.issues.push(format!("{manifest_uri} -> {error}"));
                    continue;
                }
            };

            match self.load_internal_skill_manifest_with_source_path(
                manifest_uri.as_str(),
                source_path.clone(),
            ) {
                Ok(manifest) => scan.manifests.push(manifest),
                Err(error) => scan
                    .issues
                    .push(format!("{} -> {error}", source_path.display())),
            }
        }

        scan
    }
}

fn validate_bound_wendao_resource(
    resolver: &SkillVfsResolver,
    raw_value: Option<&str>,
    field_name: &str,
) -> Result<Option<String>> {
    let Some(target) = raw_value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if target.starts_with("wendao://") || target.starts_with("$wendao://") {
        resolver
            .read_utf8_shared(target)
            .with_context(|| format!("{field_name} must resolve via SkillVfsResolver"))?;
    }
    Ok(Some(target.to_string()))
}
