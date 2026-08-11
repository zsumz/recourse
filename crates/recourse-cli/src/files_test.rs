//! Bounded-read and atomic-replacement filesystem regressions.

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use super::{error::CommandError, files};

static NEXT_SANDBOX: AtomicU32 = AtomicU32::new(1);

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let sequence = NEXT_SANDBOX.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("recourse-files-{}-{sequence}", std::process::id()));
        fs::create_dir(&path).unwrap_or_else(|error| panic!("create filesystem sandbox: {error}"));
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

#[test]
fn bounded_reads_stop_after_the_first_excess_byte() {
    let sandbox = Sandbox::new();
    let path = sandbox.path("oversized.json");
    fs::write(&path, b"123456").unwrap_or_else(|error| panic!("write oversized fixture: {error}"));

    assert!(matches!(
        files::read_bounded(&path, 5),
        Err(CommandError::InputTooLarge { maximum: 5, .. })
    ));
}

#[test]
fn failed_atomic_writes_leave_the_old_lock_byte_identical() {
    let sandbox = Sandbox::new();
    let path = sandbox.path("catalog.lock");
    let old = b"old-lock";
    let new = b"complete-new-lock";
    fs::write(&path, old).unwrap_or_else(|error| panic!("write old lock: {error}"));

    for written in [0, 5, new.len()] {
        let result = files::atomic_replace_with(&path, new, |file, body| {
            file.write_all(&body[..written])?;
            Err(io::Error::other("injected write failure"))
        });
        assert!(result.is_err());
        assert_eq!(
            fs::read(&path).unwrap_or_else(|error| panic!("read preserved lock: {error}")),
            old
        );
        assert_eq!(
            fs::read_dir(&sandbox.0)
                .unwrap_or_else(|error| panic!("list filesystem sandbox: {error}"))
                .count(),
            1
        );
    }
}

#[test]
fn successful_atomic_write_replaces_the_complete_lock_and_preserves_mode() {
    let sandbox = Sandbox::new();
    let path = sandbox.path("catalog.lock");
    fs::write(&path, b"old-lock").unwrap_or_else(|error| panic!("write old lock: {error}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640))
            .unwrap_or_else(|error| panic!("set old lock mode: {error}"));
    }
    let before = permission_marker(&path);

    files::atomic_replace_with(&path, b"new-lock", Write::write_all)
        .unwrap_or_else(|error| panic!("replace lock atomically: {error}"));

    assert_eq!(
        fs::read(&path).unwrap_or_else(|error| panic!("read new lock: {error}")),
        b"new-lock"
    );
    assert_eq!(permission_marker(&path), before);
}

#[cfg(unix)]
fn permission_marker(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .unwrap_or_else(|error| panic!("read lock metadata: {error}"))
        .permissions()
        .mode()
}

#[cfg(not(unix))]
fn permission_marker(path: &std::path::Path) -> bool {
    fs::metadata(path)
        .unwrap_or_else(|error| panic!("read lock metadata: {error}"))
        .permissions()
        .readonly()
}
