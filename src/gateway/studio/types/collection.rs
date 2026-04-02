use specta::TypeCollection;

use super::config::{UiPluginArtifact, UiPluginLaunchSpec};

/// Build the Studio Specta type collection used by `export_types`.
#[must_use]
pub fn studio_type_collection() -> TypeCollection {
    TypeCollection::default()
        .register::<UiPluginArtifact>()
        .register::<UiPluginLaunchSpec>()
}

#[cfg(test)]
mod tests {
    use super::studio_type_collection;
    use specta_typescript::{BigIntExportBehavior, Typescript};

    #[test]
    fn studio_type_collection_exports_generic_plugin_artifact_types_only() {
        let exported = Typescript::new()
            .bigint(BigIntExportBehavior::Number)
            .export(&studio_type_collection())
            .expect("export studio typescript bindings");

        assert!(exported.contains("UiPluginArtifact"));
        assert!(exported.contains("UiPluginLaunchSpec"));
        assert!(!exported.contains("UiCompatDeploymentArtifact"));
        assert!(!exported.contains("UiJuliaDeploymentArtifact"));
    }
}
