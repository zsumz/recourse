//! Documentation follows validated retirement chains to the active endpoint.

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
    fn new() -> Self {
        let sequence = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recourse-replacement-docs-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path)
            .unwrap_or_else(|error| panic!("create replacement docs fixture: {error}"));
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
        .unwrap_or_else(|error| panic!("run replacement docs command: {error}"))
}

#[test]
fn retired_pages_show_direct_and_terminal_replacements() {
    let sandbox = Sandbox::new();
    let current = sandbox.path("catalog.json");
    let lock = sandbox.path("catalog.lock");
    let out = sandbox.path("problems");
    let mut artifact: serde_json::Value = serde_json::from_slice(CATALOG)
        .unwrap_or_else(|error| panic!("decode catalog fixture: {error}"));
    let diagnostics = artifact["diagnostics"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("fixture diagnostics must be an array"));
    let mut terminal = diagnostics[1].clone();
    terminal["number"] = serde_json::json!(1010);
    terminal["code"] = serde_json::json!("DSP-1010");
    terminal["type"] = serde_json::json!("https://dispatch.invalid/problems/DSP-1010");
    terminal["title"] = serde_json::json!("Unified dispatch failure");
    diagnostics.push(terminal);
    write_json(&current, &artifact);

    assert_success(&run(&[
        Path::new("accept"),
        Path::new("--current"),
        &current,
        Path::new("--lock"),
        &lock,
    ]));
    retire(&lock, "DSP-1004", "DSP-1009");
    retire(&lock, "DSP-1009", "DSP-1010");
    artifact["diagnostics"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("fixture diagnostics must be an array"))
        .retain(|diagnostic| diagnostic["code"] == "DSP-1010");
    artifact["problem_sets"] = serde_json::json!({"createJob": []});
    write_json(&current, &artifact);

    assert_success(&run(&[
        Path::new("docs"),
        Path::new("--current"),
        &current,
        Path::new("--lock"),
        &lock,
        Path::new("--out"),
        &out,
    ]));
    let page = fs::read_to_string(out.join("retired/DSP-1004.md"))
        .unwrap_or_else(|error| panic!("read retired chain page: {error}"));
    assert!(page.contains("Replacement: `DSP-1009`"));
    assert!(page.contains("Terminal replacement: `DSP-1010`"));
    assert!(!page.contains('→'));
}

fn retire(lock: &Path, code: &str, replacement: &str) {
    assert_success(&run(&[
        Path::new("retire"),
        Path::new("--lock"),
        lock,
        Path::new(code),
        Path::new("--reason"),
        Path::new("Superseded by the next diagnostic."),
        Path::new("--replacement"),
        Path::new(replacement),
    ]));
}

fn write_json(path: &Path, value: &serde_json::Value) {
    fs::write(
        path,
        serde_json::to_vec_pretty(value)
            .unwrap_or_else(|error| panic!("encode catalog fixture: {error}")),
    )
    .unwrap_or_else(|error| panic!("write catalog fixture: {error}"));
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
