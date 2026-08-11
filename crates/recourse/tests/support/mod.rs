//! Deterministic repository inventories shared by architecture tests.

use std::{
    fs,
    path::{Path, PathBuf},
};

const RUST_ROOTS: [&str; 2] = ["crates", "reference"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FileClass {
    Facade,
    Implementation,
    Test,
    Auxiliary,
}

pub(crate) fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(workspace) = manifest.parent().and_then(Path::parent) else {
        panic!("recourse crate should live under <workspace>/crates");
    };
    workspace.to_path_buf()
}

pub(crate) fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

pub(crate) fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn rust_files(workspace: &Path) -> Vec<PathBuf> {
    files_with_name(workspace, None)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect()
}

pub(crate) fn rust_files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(root, &mut files);
    files.retain(|path| path.extension().is_some_and(|extension| extension == "rs"));
    files.sort();
    files
}

pub(crate) fn manifest_paths(workspace: &Path) -> Vec<PathBuf> {
    files_with_name(workspace, Some("Cargo.toml"))
}

fn files_with_name(workspace: &Path, name: Option<&str>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in RUST_ROOTS {
        collect_files(&workspace.join(root), &mut files);
    }
    files.retain(|path| {
        name.is_none_or(|required| path.file_name().is_some_and(|value| value == required))
    });
    files.sort();
    files
}

pub(crate) fn classify(root: &Path, path: &Path) -> FileClass {
    let relative = display_path(root, path);
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if relative.contains("/tests/") || name.ends_with("_test.rs") {
        FileClass::Test
    } else if matches!(name, "lib.rs" | "main.rs" | "mod.rs") {
        FileClass::Facade
    } else if relative.contains("/examples/") || relative.contains("/src/bin/") {
        FileClass::Auxiliary
    } else {
        FileClass::Implementation
    }
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }
    let entries =
        fs::read_dir(root).unwrap_or_else(|error| panic!("read {}: {error}", root.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("read entry under {}: {error}", root.display()))
            .path();
        if path.is_dir() {
            if !matches!(
                path.file_name().and_then(|value| value.to_str()),
                Some(".git" | "target")
            ) {
                collect_files(&path, files);
            }
        } else if path.is_file() {
            files.push(path);
        }
    }
}
