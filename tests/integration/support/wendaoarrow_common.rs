use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::sleep;

pub(crate) struct WendaoArrowServiceGuard {
    child: Child,
}

impl WendaoArrowServiceGuard {
    pub(crate) fn new(child: Child) -> Self {
        Self { child }
    }

    pub(crate) fn kill(&mut self) {
        if let Some(_status) = self
            .child
            .try_wait()
            .unwrap_or_else(|error| panic!("poll WendaoArrow child: {error}"))
        {
            return;
        }
        self.child
            .kill()
            .unwrap_or_else(|error| panic!("kill WendaoArrow child: {error}"));
        let _ = self.child.wait();
    }
}

impl Drop for WendaoArrowServiceGuard {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub(crate) async fn wait_for_health(base_url: &str) {
    let socket_addr = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .unwrap_or(base_url)
        .to_string();

    for _ in 0..50 {
        if TcpStream::connect(&socket_addr).await.is_ok() {
            return;
        }
        sleep(Duration::from_millis(200)).await;
    }

    panic!("real WendaoArrow Flight service did not become ready in time");
}

pub(crate) fn reserve_test_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .unwrap_or_else(|error| panic!("reserve WendaoArrow test port: {error}"))
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../")
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve repo root: {error}"))
}

pub(crate) fn wendaoarrow_package_dir() -> PathBuf {
    repo_root()
        .join(".data/WendaoArrow")
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoArrow package dir: {error}"))
}

pub(crate) fn wendaoarrow_script(name: &str) -> PathBuf {
    wendaoarrow_package_dir()
        .join("scripts")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoArrow script `{name}`: {error}"))
}
