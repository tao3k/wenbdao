use std::path::Path;
use std::sync::Arc;

use crate::analyzers::cache::{
    RepositoryAnalysisCacheKey, ValkeyAnalysisCache, build_repository_analysis_cache_key,
    load_cached_repository_analysis, store_cached_repository_analysis,
};
use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::{
    AnalysisContext, PluginLinkContext, RepoIntelligencePlugin, RepositoryAnalysisOutput,
};
use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::skeptic;
use crate::git::checkout::{
    CheckoutSyncMode, ResolvedRepositorySourceKind, discover_checkout_metadata,
    resolve_repository_source,
};

use super::bootstrap::bootstrap_builtin_registry;
use super::cached::CachedRepositoryAnalysis;
use super::merge::{hydrate_repository_record, merge_repository_analysis};
use super::registry::load_registered_repository;
use super::relation_dedupe::dedupe_relations;

/// Analyze one repository from configuration into normalized records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when config loading or repository
/// analysis fails.
pub fn analyze_repository_from_config_with_registry(
    repo_id: &str,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let repository = load_registered_repository(repo_id, config_path, cwd)?;
    analyze_registered_repository_with_registry(&repository, cwd, registry)
}

/// Analyze one repository from configuration into normalized records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when config loading or repository
/// analysis fails.
pub fn analyze_repository_from_config(
    repo_id: &str,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    analyze_repository_from_config_with_registry(repo_id, config_path, cwd, &registry)
}

/// Analyze one already-resolved registered repository.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn analyze_registered_repository_with_registry(
    repository: &RegisteredRepository,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    analyze_registered_repository_bundle_with_registry(repository, cwd, registry)
        .map(|cached| cached.analysis)
}

/// Analyze one already-resolved registered repository and preserve its stable cache identity.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn analyze_registered_repository_bundle_with_registry(
    repository: &RegisteredRepository,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<CachedRepositoryAnalysis, RepoIntelligenceError> {
    let repository_source = resolve_analysis_source(repository, cwd)?;
    let repository_root = repository_source.checkout_root.clone();
    let analysis_context = AnalysisContext {
        repository: repository.clone(),
        repository_root: repository_root.clone(),
    };
    let plugins = registry.resolve_for_repository(repository)?;
    preflight_repository_plugins(&plugins, &analysis_context, repository_root.as_path())?;
    let checkout_metadata = discover_checkout_metadata(repository_root.as_path());
    let cache_key = build_repository_analysis_cache_key(
        repository,
        &repository_source,
        checkout_metadata.as_ref(),
    );
    if let Some(cached) = load_cached_repository_analysis(&cache_key)? {
        return Ok(CachedRepositoryAnalysis {
            cache_key,
            analysis: cached,
        });
    }

    let valkey_cache = ValkeyAnalysisCache::new()?;
    if let Some(cached) = load_cached_analysis_from_valkey(&cache_key, valkey_cache.as_ref())? {
        return Ok(CachedRepositoryAnalysis {
            cache_key,
            analysis: cached,
        });
    }

    if repository.plugins.is_empty() {
        return Err(RepoIntelligenceError::MissingRequiredPlugin {
            repo_id: repository.id.clone(),
            plugin_id: "any".to_string(),
        });
    }

    let mut output = analyze_repository_plugins(
        repository,
        repository_root.as_path(),
        &analysis_context,
        &plugins,
    )?;

    let link_context = PluginLinkContext {
        repository: repository.clone(),
        repository_root: repository_root.clone(),
        modules: output.modules.clone(),
        symbols: output.symbols.clone(),
        examples: output.examples.clone(),
        docs: output.docs.clone(),
    };
    enrich_repository_relations(&plugins, &link_context, &mut output)?;
    dedupe_relations(&mut output.relations);

    if output.repository.is_none() {
        output.repository = Some(repository.into());
    }
    if let Some(record) = output.repository.as_mut() {
        hydrate_repository_record(
            record,
            repository,
            repository_root.as_path(),
            checkout_metadata.as_ref(),
        );
    }

    let audit_results = skeptic::audit_symbols(&output.symbols, &output.docs, &output.relations);
    for symbol in &mut output.symbols {
        if let Some(state) = audit_results.get(&symbol.symbol_id) {
            symbol.verification_state.clone_from(&Some(state.clone()));
        }
    }

    if let Some(ref cache) = valkey_cache {
        cache.set(&cache_key, &output);
    }
    store_cached_repository_analysis(cache_key.clone(), &output)?;

    Ok(CachedRepositoryAnalysis {
        cache_key,
        analysis: output,
    })
}

/// Analyze one already-resolved registered repository.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn analyze_registered_repository(
    repository: &RegisteredRepository,
    cwd: &Path,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    analyze_registered_repository_with_registry(repository, cwd, &registry)
}

fn resolve_analysis_source(
    repository: &RegisteredRepository,
    cwd: &Path,
) -> Result<crate::git::checkout::ResolvedRepositorySource, RepoIntelligenceError> {
    let status_source = resolve_repository_source(repository, cwd, CheckoutSyncMode::Status)?;
    if matches!(
        status_source.source_kind,
        ResolvedRepositorySourceKind::ManagedRemote
    ) || !status_source.checkout_root.is_dir()
    {
        resolve_repository_source(repository, cwd, CheckoutSyncMode::Ensure)
    } else {
        Ok(status_source)
    }
}

fn preflight_repository_plugins(
    plugins: &[Arc<dyn RepoIntelligencePlugin>],
    analysis_context: &AnalysisContext,
    repository_root: &Path,
) -> Result<(), RepoIntelligenceError> {
    for plugin in plugins {
        plugin.preflight_repository(analysis_context, repository_root)?;
    }
    Ok(())
}

fn load_cached_analysis_from_valkey(
    cache_key: &RepositoryAnalysisCacheKey,
    valkey_cache: Option<&ValkeyAnalysisCache>,
) -> Result<Option<RepositoryAnalysisOutput>, RepoIntelligenceError> {
    let Some(cache) = valkey_cache else {
        return Ok(None);
    };
    let Some(cached) = cache.get(cache_key) else {
        return Ok(None);
    };
    store_cached_repository_analysis(cache_key.clone(), &cached)?;
    Ok(Some(cached))
}

fn analyze_repository_plugins(
    repository: &RegisteredRepository,
    repository_root: &Path,
    analysis_context: &AnalysisContext,
    plugins: &[Arc<dyn RepoIntelligencePlugin>],
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let mut output = RepositoryAnalysisOutput::default();
    let mut any_plugin_output = false;

    for plugin in plugins {
        let plugin_output = plugin.analyze_repository(analysis_context, repository_root)?;
        any_plugin_output = true;
        merge_repository_analysis(&mut output, plugin_output);
    }

    if any_plugin_output {
        Ok(output)
    } else {
        Err(RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "repo `{}` produced no repository analysis output",
                repository.id
            ),
        })
    }
}

fn enrich_repository_relations(
    plugins: &[Arc<dyn RepoIntelligencePlugin>],
    link_context: &PluginLinkContext,
    output: &mut RepositoryAnalysisOutput,
) -> Result<(), RepoIntelligenceError> {
    for plugin in plugins {
        output
            .relations
            .extend(plugin.enrich_relations(link_context)?);
    }
    Ok(())
}
