//! End-to-end refusal of incompatible schemas and retired-code reuse.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU32, Ordering},
};

const CATALOG: &[u8] = include_bytes!("fixtures/cli-two-surface.json");
static NEXT_SANDBOX: AtomicU32 = AtomicU32::new(1);

struct Sandbox(PathBuf);

impl Sandbox {
    fn new(label: &str) -> Self {
        let sequence = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recourse-governance-{}-{label}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap_or_else(|error| panic!("create governance fixture: {error}"));
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
        .unwrap_or_else(|error| panic!("run governance command: {error}"))
}

fn accepted_fixture(label: &str) -> (Sandbox, PathBuf, PathBuf) {
    let sandbox = Sandbox::new(label);
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
    assert!(accept.status.success(), "{}", stderr(&accept));
    (sandbox, current, lock)
}

fn check(current: &Path, lock: &Path) -> Output {
    run(&[
        Path::new("check"),
        Path::new("--current"),
        current,
        Path::new("--lock"),
        lock,
        Path::new("--format"),
        Path::new("json"),
    ])
}

fn read_json(path: &Path) -> serde_json::Value {
    let body = fs::read(path).unwrap_or_else(|error| panic!("read governance fixture: {error}"));
    serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("decode governance fixture: {error}"))
}

fn write_json(path: &Path, value: &serde_json::Value) {
    let body = serde_json::to_vec_pretty(value)
        .unwrap_or_else(|error| panic!("encode governance fixture: {error}"));
    fs::write(path, body).unwrap_or_else(|error| panic!("write governance fixture: {error}"));
}

fn report(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("command must emit a JSON report: {error}"))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn an_incompatible_evidence_schema_fails_check_with_a_precise_diagnostic() {
    let (sandbox, current, lock) = accepted_fixture("schema");
    let mut artifact = read_json(&current);
    let schema = &mut artifact["diagnostics"][0]["evidence_schema"];
    schema["properties"]["trace_id"] = serde_json::json!({ "type": "string" });
    let Some(required) = schema["required"].as_array_mut() else {
        panic!("fixture evidence schema must list required properties");
    };
    required.push(serde_json::json!("trace_id"));
    write_json(&current, &artifact);

    let checked = check(&current, &lock);
    assert_eq!(checked.status.code(), Some(1), "{}", stderr(&checked));
    let checked = report(&checked);
    assert_eq!(checked["compatible"], false);
    assert_eq!(checked["has_breaking"], true);
    assert_eq!(checked["has_forbidden"], false);
    assert_eq!(
        checked["changes"],
        serde_json::json!([{
            "id": "REC-COMPAT-013",
            "severity": "breaking",
            "code": "DSP-1004",
            "path": "evidence_schema.properties.trace_id",
            "previous": "absent",
            "current": "required",
            "reason": "Existing emitters may not provide the new field.",
            "action": "Make it optional or mint a new code."
        }])
    );
    drop(sandbox);
}

#[test]
fn a_retired_code_cannot_be_reused_even_with_acknowledgement() {
    let (sandbox, current, lock) = accepted_fixture("retired");
    let mut history = read_json(&lock);
    let entry = history["entries"].as_array_mut().and_then(|entries| {
        entries
            .iter_mut()
            .find(|entry| entry["diagnostic"]["code"] == "DSP-1009")
    });
    let Some(entry) = entry else {
        panic!("DSP-1009 must be locked before retirement");
    };
    entry["state"] = serde_json::json!("retired");
    entry["reason"] = serde_json::json!("The legacy worker was removed.");
    write_json(&lock, &history);
    let tombstoned = fs::read(&lock).unwrap_or_else(|error| panic!("read tombstone: {error}"));

    let checked = check(&current, &lock);
    assert_eq!(checked.status.code(), Some(1), "{}", stderr(&checked));
    let checked = report(&checked);
    assert_eq!(checked["has_forbidden"], true);
    assert_eq!(
        checked["changes"],
        serde_json::json!([{
            "id": "REC-COMPAT-002",
            "severity": "forbidden",
            "code": "DSP-1009",
            "path": "state",
            "previous": "retired",
            "current": "active",
            "reason": "Retired codes remain tombstoned.",
            "action": "Mint a new diagnostic code."
        }])
    );

    let refused = run(&[
        Path::new("accept"),
        Path::new("--current"),
        &current,
        Path::new("--lock"),
        &lock,
        Path::new("--acknowledge-breaking"),
        Path::new("--format"),
        Path::new("json"),
    ]);
    assert_eq!(refused.status.code(), Some(1), "{}", stderr(&refused));
    assert_eq!(report(&refused)["changes"][0]["id"], "REC-COMPAT-002");
    assert_eq!(
        fs::read(&lock).unwrap_or_else(|error| panic!("read refused lock: {error}")),
        tombstoned
    );
    drop(sandbox);
}
