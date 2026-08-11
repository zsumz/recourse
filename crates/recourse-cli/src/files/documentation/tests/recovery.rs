//! Documentation-tree crash-recovery regressions.

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use super::StagingTree;

static NEXT_SANDBOX: AtomicU32 = AtomicU32::new(1);

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let sequence = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recourse-documentation-tree-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap_or_else(|error| panic!("create tree fixture: {error}"));
        Self(path)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn stage_creation_recovers_an_interrupted_fallback_commit() {
    let sandbox = Sandbox::new();
    let out = sandbox.0.join("problems");
    let backup = sandbox.0.join(".recourse-backup-problems");
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create live tree: {error}"));
    fs::write(out.join("notes.md"), "preserve after recovery\n")
        .unwrap_or_else(|error| panic!("write recovery sentinel: {error}"));
    fs::rename(&out, &backup)
        .unwrap_or_else(|error| panic!("simulate interrupted commit: {error}"));

    let staging = StagingTree::new(&out)
        .unwrap_or_else(|error| panic!("recover interrupted commit: {error}"));
    staging
        .copy_existing(&out)
        .unwrap_or_else(|error| panic!("copy recovered tree: {error}"));
    fs::write(staging.path().join("index.md"), "new generation\n")
        .unwrap_or_else(|error| panic!("write staged generation: {error}"));
    staging
        .commit(&out)
        .unwrap_or_else(|error| panic!("commit recovered tree: {error}"));

    assert_eq!(
        fs::read_to_string(out.join("notes.md"))
            .unwrap_or_else(|error| panic!("read recovered sentinel: {error}")),
        "preserve after recovery\n"
    );
    assert!(out.join("index.md").is_file());
    assert!(!backup.exists());
}
