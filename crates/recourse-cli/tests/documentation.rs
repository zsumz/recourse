//! End-to-end explanation and deterministic documentation fixtures.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU32, Ordering},
};

const CATALOG: &[u8] = include_bytes!("../../../conformance/catalogs/cli-two-surface.json");
static NEXT_SANDBOX: AtomicU32 = AtomicU32::new(1);

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let sequence = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("recourse-docs-{}-{sequence}", std::process::id()));
        fs::create_dir(&path).unwrap_or_else(|error| panic!("create docs fixture: {error}"));
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(arguments: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-recourse"))
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("run documentation command: {error}"))
}

fn accepted_fixture() -> (Sandbox, PathBuf, PathBuf) {
    let sandbox = Sandbox::new();
    let current = sandbox.path("catalog.json");
    let lock = sandbox.path("catalog.lock");
    fs::write(&current, CATALOG).unwrap_or_else(|error| panic!("write catalog: {error}"));
    let accept = run(&[
        Path::new("accept"),
        Path::new("--current"),
        &current,
        Path::new("--lock"),
        &lock,
    ]);
    assert!(accept.status.success());
    (sandbox, current, lock)
}

#[test]
fn explain_returns_the_complete_governed_definition() {
    let (sandbox, current, _) = accepted_fixture();
    let output = run(&[
        Path::new("explain"),
        Path::new("--current"),
        &current,
        Path::new("DSP-1004"),
        Path::new("--format"),
        Path::new("json"),
    ]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("explanation must be JSON: {error}"));
    assert_eq!(value["code"], "DSP-1004");
    assert_eq!(value["surfaces"]["http"]["status"], 409);
    drop(sandbox);
}

#[test]
fn docs_generate_every_active_page_and_preserve_unowned_files() {
    let (sandbox, current, lock) = accepted_fixture();
    let out = sandbox.path("problems");
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create docs output: {error}"));
    fs::write(out.join("notes.md"), "owned by the application\n")
        .unwrap_or_else(|error| panic!("write unowned page: {error}"));

    let output = run(&[
        Path::new("docs"),
        Path::new("--current"),
        &current,
        Path::new("--lock"),
        &lock,
        Path::new("--out"),
        &out,
        Path::new("--format"),
        Path::new("json"),
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let result: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("docs result must be JSON: {error}"));
    assert_eq!(result["generated"].as_array().map(Vec::len), Some(3));
    assert!(out.join("index.md").is_file());
    assert!(out.join("DSP-1004.md").is_file());
    assert!(out.join("notes.md").is_file());

    let page = fs::read_to_string(out.join("DSP-1004.md"))
        .unwrap_or_else(|error| panic!("read generated page: {error}"));
    assert!(page.contains("# DSP-1004: Idempotency key conflict"));
    assert!(page.contains("| `original_job_id` | yes | reference \\#/$defs/JobId |"));
    assert!(page.contains("- `createJob`"));

    let stale = out.join("DSP-0999.md");
    fs::write(&stale, "old generated page\n")
        .unwrap_or_else(|error| panic!("write stale page: {error}"));
    let manifest = out.join(".recourse-generated");
    let mut owned =
        fs::read_to_string(&manifest).unwrap_or_else(|error| panic!("read docs manifest: {error}"));
    owned.push_str("DSP-0999.md\n");
    fs::write(&manifest, owned).unwrap_or_else(|error| panic!("extend docs manifest: {error}"));
    let rerun = docs(&current, &lock, &out);
    assert!(rerun.status.success(), "{}", stderr(&rerun));
    assert!(!stale.exists());
    assert!(out.join("notes.md").is_file());
}

#[test]
fn retired_history_gets_a_separate_page() {
    let (sandbox, current, lock) = accepted_fixture();
    let mut artifact: serde_json::Value = serde_json::from_slice(CATALOG)
        .unwrap_or_else(|error| panic!("decode catalog fixture: {error}"));
    let diagnostics = artifact["diagnostics"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("fixture diagnostics must be an array"));
    diagnostics.retain(|diagnostic| diagnostic["code"] != "DSP-1009");
    fs::write(
        &current,
        serde_json::to_vec_pretty(&artifact)
            .unwrap_or_else(|error| panic!("encode retired catalog: {error}")),
    )
    .unwrap_or_else(|error| panic!("write retired catalog: {error}"));

    let mut history: serde_json::Value = serde_json::from_slice(
        &fs::read(&lock).unwrap_or_else(|error| panic!("read accepted lock: {error}")),
    )
    .unwrap_or_else(|error| panic!("decode accepted lock: {error}"));
    let entry = history["entries"]
        .as_array_mut()
        .and_then(|entries| {
            entries
                .iter_mut()
                .find(|entry| entry["diagnostic"]["code"] == "DSP-1009")
        })
        .unwrap_or_else(|| panic!("DSP-1009 must be locked"));
    entry["state"] = serde_json::json!("retired");
    entry["reason"] = serde_json::json!("The legacy worker was removed.");
    fs::write(
        &lock,
        serde_json::to_vec_pretty(&history)
            .unwrap_or_else(|error| panic!("encode retired lock: {error}")),
    )
    .unwrap_or_else(|error| panic!("write retired lock: {error}"));

    let out = sandbox.path("retired-problems");
    let output = docs(&current, &lock, &out);
    assert!(output.status.success(), "{}", stderr(&output));
    let page = fs::read_to_string(out.join("retired/DSP-1009.md"))
        .unwrap_or_else(|error| panic!("read retired page: {error}"));
    assert!(page.contains("- State: **Retired**"));
    assert!(page.contains("The legacy worker was removed."));
    assert!(page.contains("## Historical impact"));
}

fn docs(current: &Path, lock: &Path, out: &Path) -> Output {
    run(&[
        Path::new("docs"),
        Path::new("--current"),
        current,
        Path::new("--lock"),
        lock,
        Path::new("--out"),
        out,
        Path::new("--format"),
        Path::new("json"),
    ])
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
