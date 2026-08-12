//! Documentation-tree crash-recovery regressions.

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use super::{StagingTree, transaction::Transaction};

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
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create live tree: {error}"));
    fs::write(out.join("notes.md"), "preserve after recovery\n")
        .unwrap_or_else(|error| panic!("write recovery sentinel: {error}"));
    let transaction = Transaction::begin(&out)
        .unwrap_or_else(|error| panic!("begin interrupted transaction: {error}"));
    transaction
        .back_up(&out)
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
}

#[test]
fn unowned_legacy_backup_collision_is_never_deleted() {
    let sandbox = Sandbox::new();
    let out = sandbox.0.join("problems");
    let backup = sandbox.0.join(".recourse-backup-problems");
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create live tree: {error}"));
    fs::create_dir(&backup).unwrap_or_else(|error| panic!("create colliding tree: {error}"));
    let live = b"live sentinel\n";
    let unrelated = b"unrelated sentinel\n";
    fs::write(out.join("sentinel"), live)
        .unwrap_or_else(|error| panic!("write live sentinel: {error}"));
    fs::write(backup.join("sentinel"), unrelated)
        .unwrap_or_else(|error| panic!("write unrelated sentinel: {error}"));

    assert!(StagingTree::new(&out).is_err());
    assert_eq!(
        fs::read(out.join("sentinel")).unwrap_or_else(|error| panic!("read live: {error}")),
        live
    );
    assert_eq!(
        fs::read(backup.join("sentinel")).unwrap_or_else(|error| panic!("read unrelated: {error}")),
        unrelated
    );
}

#[test]
fn mismatched_ownership_marker_never_authorizes_deletion() {
    let sandbox = Sandbox::new();
    let out = sandbox.0.join("problems");
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create live tree: {error}"));
    fs::write(out.join("old"), b"old sentinel\n")
        .unwrap_or_else(|error| panic!("write old sentinel: {error}"));
    let transaction = Transaction::begin(&out)
        .unwrap_or_else(|error| panic!("begin interrupted transaction: {error}"));
    transaction
        .back_up(&out)
        .unwrap_or_else(|error| panic!("back up live tree: {error}"));
    let backup = fs::read_dir(&sandbox.0)
        .unwrap_or_else(|error| panic!("list transaction artifacts: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".recourse-backup-"))
        })
        .unwrap_or_else(|| panic!("transaction backup must exist"));
    fs::write(backup.join(".recourse-transaction-owner"), b"not owned")
        .unwrap_or_else(|error| panic!("corrupt ownership marker: {error}"));
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create replacement tree: {error}"));
    fs::write(out.join("new"), b"new sentinel\n")
        .unwrap_or_else(|error| panic!("write new sentinel: {error}"));

    assert!(StagingTree::new(&out).is_err());
    assert_eq!(
        fs::read(backup.join("old")).unwrap_or_else(|error| panic!("read old tree: {error}")),
        b"old sentinel\n"
    );
    assert_eq!(
        fs::read(out.join("new")).unwrap_or_else(|error| panic!("read new tree: {error}")),
        b"new sentinel\n"
    );
}

#[test]
fn unrelated_replacement_never_authorizes_backup_deletion() {
    let sandbox = Sandbox::new();
    let out = sandbox.0.join("problems");
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create live tree: {error}"));
    let old = b"old sentinel\n";
    let unrelated = b"unrelated sentinel\n";
    fs::write(out.join("sentinel"), old)
        .unwrap_or_else(|error| panic!("write old sentinel: {error}"));
    let transaction = Transaction::begin(&out)
        .unwrap_or_else(|error| panic!("begin interrupted transaction: {error}"));
    transaction
        .back_up(&out)
        .unwrap_or_else(|error| panic!("back up live tree: {error}"));
    let backup = find_backup(&sandbox.0);
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create unrelated tree: {error}"));
    fs::write(out.join("sentinel"), unrelated)
        .unwrap_or_else(|error| panic!("write unrelated sentinel: {error}"));

    assert!(StagingTree::new(&out).is_err());
    assert_eq!(
        fs::read(backup.join("sentinel"))
            .unwrap_or_else(|error| panic!("read backed-up tree: {error}")),
        old
    );
    assert_eq!(
        fs::read(out.join("sentinel"))
            .unwrap_or_else(|error| panic!("read unrelated tree: {error}")),
        unrelated
    );
}

#[test]
fn matching_replacement_completes_interrupted_commit() {
    let sandbox = Sandbox::new();
    let out = sandbox.0.join("problems");
    let staged = sandbox.0.join("staged");
    fs::create_dir(&out).unwrap_or_else(|error| panic!("create live tree: {error}"));
    fs::write(out.join("old"), b"old sentinel\n")
        .unwrap_or_else(|error| panic!("write old sentinel: {error}"));
    fs::create_dir(&staged).unwrap_or_else(|error| panic!("create staged tree: {error}"));
    fs::write(staged.join("new"), b"new sentinel\n")
        .unwrap_or_else(|error| panic!("write new sentinel: {error}"));
    let transaction = Transaction::begin(&out)
        .unwrap_or_else(|error| panic!("begin interrupted transaction: {error}"));
    transaction
        .mark_staged(&staged)
        .unwrap_or_else(|error| panic!("mark staged tree: {error}"));
    transaction
        .back_up(&out)
        .unwrap_or_else(|error| panic!("back up live tree: {error}"));
    let backup = find_backup(&sandbox.0);
    fs::rename(&staged, &out).unwrap_or_else(|error| panic!("install staged tree: {error}"));

    let _staging =
        StagingTree::new(&out).unwrap_or_else(|error| panic!("finish interrupted commit: {error}"));
    assert!(!backup.exists());
    assert_eq!(
        fs::read(out.join("new")).unwrap_or_else(|error| panic!("read new tree: {error}")),
        b"new sentinel\n"
    );
    assert!(!out.join(".recourse-transaction-owner").exists());
}

fn find_backup(parent: &std::path::Path) -> PathBuf {
    fs::read_dir(parent)
        .unwrap_or_else(|error| panic!("list transaction artifacts: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".recourse-backup-"))
        })
        .unwrap_or_else(|| panic!("transaction backup must exist"))
}
