use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use xiuxian_skills::{
    InternalSkillManifest, InternalSkillManifestScan, InternalSkillNativeAliasMountReport,
    InternalSkillNativeAliasSpec, InternalSkillWorkflowType,
    compile_internal_skill_manifest_aliases,
};

impl From<AuthorizedInternalSkillManifestScan> for InternalSkillManifestScan {
    fn from(auth: AuthorizedInternalSkillManifestScan) -> Self {
        Self {
            discovered_paths: auth.discovered_paths,
            manifests: auth.manifests,
            issues: auth.issues,
        }
    }
}

use super::super::{SkillVfsResolver, WendaoResourceUri};
use super::catalog::InternalSkillIntentCatalog;
use super::report::{InternalSkillAuthorityReport, build_authority_report};

/// Validation result for only the manifests explicitly authorized by root `SKILL.md` files.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthorizedInternalSkillManifestScan {
    /// Concrete paths for every discovered authorized manifest candidate.
    pub discovered_paths: Vec<PathBuf>,
    /// Successfully parsed and validated authorized manifests.
    pub manifests: Vec<InternalSkillManifest>,
    /// Human-readable issues for authorized manifests that could not be resolved or validated.
    pub issues: Vec<String>,
    /// Authority classification report used to derive the authorized manifest set.
    pub authority: InternalSkillAuthorityReport,
}

/// Prepared internal native-alias payload derived from the authorized-manifest scan.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthorizedInternalSkillNativeAliasScan {
    /// Pre-mount report with discovery, authority, and compile-stage diagnostics populated.
    pub report: InternalSkillNativeAliasMountReport<InternalSkillWorkflowType>,
    /// Runtime-ready alias specs that Daochang can mount into the native tool registry.
    pub compiled_specs: Vec<InternalSkillNativeAliasSpec<InternalSkillWorkflowType>>,
}

impl SkillVfsResolver {
    /// Compare `SKILL.md` intention links against physically mounted internal manifests.
    ///
    /// # Errors
    ///
    /// Returns an error when the internal-skill `LinkGraphIndex` cannot be built or when a
    /// mounted `SKILL.md` document cannot be reparsed for raw intent targets.
    pub fn audit_internal_manifest_authority(&self) -> Result<InternalSkillAuthorityReport> {
        let catalog = self.collect_internal_manifest_intents()?;
        Ok(self.audit_internal_manifest_authority_with_catalog(&catalog))
    }

    /// Compare physical manifests against a reusable internal-skill intent catalog.
    #[must_use]
    pub fn audit_internal_manifest_authority_with_catalog(
        &self,
        catalog: &InternalSkillIntentCatalog,
    ) -> InternalSkillAuthorityReport {
        let physical_manifests = self
            .list_internal_manifest_uris()
            .into_iter()
            .collect::<BTreeSet<_>>();
        build_authority_report(&physical_manifests, catalog)
    }

    /// Discover and validate only the manifests explicitly authorized by root `SKILL.md` files.
    ///
    /// Authority mismatches remain in the returned report, while load and validation failures for
    /// authorized manifests are collected in `issues` just like `scan_internal_manifests()`. Call
    /// [`Self::scan_authorized_internal_manifests_with_catalog`] when you already hold a reusable
    /// [`InternalSkillIntentCatalog`] built from cached link-graph indexes.
    ///
    /// # Errors
    ///
    /// Returns an error when authority auditing cannot build or traverse the internal-skill link
    /// graph. Validation errors for individual authorized manifests are reported in `issues`.
    pub fn scan_authorized_internal_manifests(
        &self,
    ) -> Result<AuthorizedInternalSkillManifestScan> {
        let catalog = self.collect_internal_manifest_intents()?;
        Ok(self.scan_authorized_internal_manifests_with_catalog(&catalog))
    }

    /// Discover and validate authorized manifests from a reusable internal-skill intent catalog.
    #[must_use]
    pub fn scan_authorized_internal_manifests_with_catalog(
        &self,
        catalog: &InternalSkillIntentCatalog,
    ) -> AuthorizedInternalSkillManifestScan {
        let authority = self.audit_internal_manifest_authority_with_catalog(catalog);
        build_authorized_internal_manifest_scan(self, authority)
    }

    /// Discover, validate, and precompile authorized internal native aliases for runtime mounting.
    ///
    /// # Errors
    ///
    /// Returns an error when authority auditing cannot build or traverse the internal-skill link
    /// graph. Validation and compile errors for individual manifests are retained in the returned
    /// report `issues` list.
    pub fn scan_authorized_internal_native_aliases(
        &self,
        root: &Path,
    ) -> Result<AuthorizedInternalSkillNativeAliasScan> {
        let catalog = self.collect_internal_manifest_intents()?;
        Ok(self.scan_authorized_internal_native_aliases_with_catalog(root, &catalog))
    }

    /// Discover and precompile authorized internal native aliases from a reusable intent catalog.
    #[must_use]
    pub fn scan_authorized_internal_native_aliases_with_catalog(
        &self,
        root: &Path,
        catalog: &InternalSkillIntentCatalog,
    ) -> AuthorizedInternalSkillNativeAliasScan {
        let scan = self.scan_authorized_internal_manifests_with_catalog(catalog);
        build_authorized_internal_native_alias_scan(root, scan)
    }
}

fn build_authorized_internal_manifest_scan(
    resolver: &SkillVfsResolver,
    authority: InternalSkillAuthorityReport,
) -> AuthorizedInternalSkillManifestScan {
    let authorized_manifests = authority.authorized_manifests.clone();
    let mut scan = AuthorizedInternalSkillManifestScan {
        discovered_paths: Vec::with_capacity(authorized_manifests.len()),
        manifests: Vec::with_capacity(authorized_manifests.len()),
        issues: Vec::new(),
        authority,
    };

    for manifest_uri in authorized_manifests {
        let parsed_uri = match WendaoResourceUri::parse(manifest_uri.as_str()) {
            Ok(uri) => uri,
            Err(error) => {
                scan.issues.push(format!("{manifest_uri} -> {error}"));
                continue;
            }
        };
        let source_path = match resolver.resolve_parsed_uri(&parsed_uri) {
            Ok(path) => {
                scan.discovered_paths.push(path.clone());
                path
            }
            Err(error) => {
                scan.issues.push(format!("{manifest_uri} -> {error}"));
                continue;
            }
        };

        match resolver.load_internal_skill_manifest(manifest_uri.as_str()) {
            Ok(manifest) => scan.manifests.push(manifest),
            Err(error) => scan
                .issues
                .push(format!("{} -> {error}", source_path.display())),
        }
    }

    scan
}

fn build_authorized_internal_native_alias_scan(
    root: &Path,
    scan: AuthorizedInternalSkillManifestScan,
) -> AuthorizedInternalSkillNativeAliasScan {
    let AuthorizedInternalSkillManifestScan {
        discovered_paths,
        manifests,
        issues,
        authority,
    } = scan;
    let compilation = compile_internal_skill_manifest_aliases(manifests);

    let mut report = InternalSkillNativeAliasMountReport::from_root(root);
    report.discovered_paths = discovered_paths;
    report.authorized_count = authority.authorized_manifests.len();
    report.ghost_count = authority.ghost_links.len();
    report.unauthorized_count = authority.unauthorized_manifests.len();
    report.issues.extend(issues);
    report.issues.extend(
        authority
            .ghost_links
            .iter()
            .map(|uri| format!("{uri} -> declared by SKILL.md but manifest is missing")),
    );
    report.issues.extend(
        authority
            .unauthorized_manifests
            .iter()
            .map(|uri| format!("{uri} -> manifest is present but not granted by root SKILL.md")),
    );
    report.issues.extend(compilation.issues);

    AuthorizedInternalSkillNativeAliasScan {
        report,
        compiled_specs: compilation.compiled_specs,
    }
}
