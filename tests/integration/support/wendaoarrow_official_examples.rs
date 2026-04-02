use std::process::{Command, Stdio};

use super::wendaoarrow_common::{
    WendaoArrowServiceGuard, repo_root, reserve_test_port, wait_for_health, wendaoarrow_script,
};
use xiuxian_wendao_julia::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH, LinkGraphJuliaAnalyzerLaunchManifest,
    LinkGraphJuliaDeploymentArtifact, LinkGraphJuliaRerankRuntimeConfig,
};

pub(crate) async fn spawn_wendaoarrow_stream_scoring_service() -> (String, WendaoArrowServiceGuard)
{
    spawn_wendaoarrow_official_example(
        "run_stream_scoring_flight_server.sh",
        "spawn WendaoArrow stream scoring service",
    )
    .await
}

pub(crate) async fn spawn_wendaoarrow_stream_metadata_service() -> (String, WendaoArrowServiceGuard)
{
    spawn_wendaoarrow_official_example(
        "run_stream_metadata_flight_server.sh",
        "spawn WendaoArrow stream metadata service",
    )
    .await
}

pub(crate) async fn spawn_wendaoanalyzer_stream_linear_blend_service()
-> (String, WendaoArrowServiceGuard) {
    spawn_wendaoanalyzer_service_from_manifest(&LinkGraphJuliaAnalyzerLaunchManifest {
        launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
        args: vec!["--service-mode".to_string(), "stream".to_string()],
    })
    .await
}

pub(crate) fn wendaoanalyzer_deployment_artifact_from_runtime(
    runtime: &LinkGraphJuliaRerankRuntimeConfig,
) -> LinkGraphJuliaDeploymentArtifact {
    runtime.deployment_artifact()
}

pub(crate) async fn spawn_wendaoanalyzer_service_from_manifest(
    manifest: &LinkGraphJuliaAnalyzerLaunchManifest,
) -> (String, WendaoArrowServiceGuard) {
    let port = reserve_test_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let script = repo_root().join(&manifest.launcher_path);
    let mut command = Command::new("bash");
    command.arg(script).arg("--port").arg(port.to_string());

    for argument in &manifest.args {
        command.arg(argument);
    }

    let child = command
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn WendaoAnalyzer service: {error}"));
    let guard = WendaoArrowServiceGuard::new(child);

    wait_for_health(base_url.as_str()).await;
    (base_url, guard)
}

pub(crate) async fn spawn_wendaoanalyzer_service_from_artifact(
    artifact: &LinkGraphJuliaDeploymentArtifact,
) -> (String, WendaoArrowServiceGuard) {
    spawn_wendaoanalyzer_service_from_manifest(&artifact.launch).await
}

async fn spawn_wendaoarrow_official_example(
    script_name: &str,
    error_context: &str,
) -> (String, WendaoArrowServiceGuard) {
    let port = reserve_test_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let script = wendaoarrow_script(script_name);

    let child = Command::new("bash")
        .arg(script)
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("{error_context}: {error}"));
    let guard = WendaoArrowServiceGuard::new(child);

    wait_for_health(base_url.as_str()).await;
    (base_url, guard)
}
