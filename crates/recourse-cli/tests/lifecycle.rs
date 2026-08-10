//! End-to-end artifact lifecycle and machine-output behavior.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU32, Ordering},
};

const CATALOG: &[u8] = include_bytes!("../../../conformance/catalogs/cli-two-surface.json");
static NEXT_SANDBOX: AtomicU32 = AtomicU32::new(1);

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(label: &str) -> Self {
        let sequence = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "recourse-cli-{}-{label}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root)
            .unwrap_or_else(|error| panic!("create isolated CLI fixture: {error}"));
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run(arguments: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-recourse"))
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("run cargo-recourse fixture: {error}"))
}

fn bootstrap(current: &Path, lock: &Path) {
    fs::write(current, CATALOG).unwrap_or_else(|error| panic!("write current fixture: {error}"));
    let output = run(&[
        Path::new("accept"),
        Path::new("--current"),
        current,
        Path::new("--lock"),
        lock,
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn accept_check_and_reserve_complete_one_lock_lifecycle() {
    let sandbox = Sandbox::new("lifecycle");
    let current = sandbox.path("catalog.json");
    let lock = sandbox.path("catalog.lock");
    bootstrap(&current, &lock);

    let check = run(&[
        Path::new("check"),
        Path::new("--current"),
        &current,
        Path::new("--lock"),
        &lock,
        Path::new("--format"),
        Path::new("json"),
    ]);
    assert!(check.status.success(), "{}", stderr(&check));
    let value: serde_json::Value = serde_json::from_slice(&check.stdout)
        .unwrap_or_else(|error| panic!("check must emit JSON: {error}"));
    assert_eq!(value["compatible"], true);
    assert_eq!(value["changes"], serde_json::json!([]));

    let reserve = run(&[
        Path::new("reserve"),
        Path::new("--lock"),
        &lock,
        Path::new("--format"),
        Path::new("json"),
    ]);
    assert!(reserve.status.success(), "{}", stderr(&reserve));
    let value: serde_json::Value = serde_json::from_slice(&reserve.stdout)
        .unwrap_or_else(|error| panic!("reservation must emit JSON: {error}"));
    assert_eq!(value["code"], "DSP-1010");
    assert_eq!(value["state"], "reserved");
}

#[test]
fn breaking_acceptance_requires_the_explicit_acknowledgement() {
    let sandbox = Sandbox::new("breaking");
    let current = sandbox.path("catalog.json");
    let lock = sandbox.path("catalog.lock");
    bootstrap(&current, &lock);
    let accepted = fs::read(&lock).unwrap_or_else(|error| panic!("read initial lock: {error}"));

    let mut artifact: serde_json::Value =
        serde_json::from_slice(CATALOG).unwrap_or_else(|error| panic!("decode fixture: {error}"));
    artifact["diagnostics"][0]["surfaces"]["http"]["status"] = serde_json::json!(418);
    fs::write(
        &current,
        serde_json::to_vec_pretty(&artifact)
            .unwrap_or_else(|error| panic!("encode changed fixture: {error}")),
    )
    .unwrap_or_else(|error| panic!("write changed fixture: {error}"));

    let refused = run(&[
        Path::new("accept"),
        Path::new("--current"),
        &current,
        Path::new("--lock"),
        &lock,
        Path::new("--format"),
        Path::new("json"),
    ]);
    assert_eq!(refused.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&refused.stdout)
        .unwrap_or_else(|error| panic!("refusal must emit JSON: {error}"));
    assert_eq!(value["changes"][0]["id"], "REC-COMPAT-010");
    assert_eq!(
        fs::read(&lock).unwrap_or_else(|error| panic!("read refused lock: {error}")),
        accepted
    );

    let accepted_break = run(&[
        Path::new("accept"),
        Path::new("--current"),
        &current,
        Path::new("--lock"),
        &lock,
        Path::new("--acknowledge-breaking"),
    ]);
    assert!(
        accepted_break.status.success(),
        "{}",
        stderr(&accepted_break)
    );
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
