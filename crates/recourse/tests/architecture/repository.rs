//! Workspace shape, CI, and toolchain inputs stay reproducible.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use super::support::{display_path, manifest_paths, read, workspace_root};

#[test]
fn workspace_members_match_the_package_inventory() {
    let workspace = workspace_root();
    let manifest = read(&workspace.join("Cargo.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse workspace Cargo.toml: {error}"));
    let Some(members) = manifest["workspace"]["members"].as_array() else {
        panic!("workspace members must be an array");
    };
    let declared = members
        .iter()
        .map(|value| {
            value.as_str().map_or_else(
                || panic!("workspace member must be a string"),
                str::to_owned,
            )
        })
        .collect::<BTreeSet<_>>();
    let actual = manifest_paths(&workspace)
        .iter()
        .map(|path| match path.parent() {
            Some(parent) => display_path(&workspace, parent),
            None => panic!("crate manifest has no parent: {}", path.display()),
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(declared, actual, "workspace member inventory drifted");
}

#[test]
fn repository_has_no_nested_git_metadata_or_submodules() {
    let workspace = workspace_root();
    let mut directories = Vec::new();
    collect_directories(&workspace, &workspace, &mut directories);
    let nested_git = directories
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == ".git"))
        .map(|path| display_path(&workspace, path))
        .collect::<Vec<_>>();

    assert!(nested_git.is_empty(), "nested Git metadata: {nested_git:?}");
    assert!(!workspace.join(".gitmodules").exists());
}

#[test]
fn continuous_integration_runs_pinned_canonical_gates() {
    let workspace = workspace_root();
    let workflow = read(&workspace.join(".github/workflows/ci.yml"));
    let setup = read(&workspace.join(".github/actions/setup-rust/action.yml"));

    for required in [
        "permissions:\n  contents: read",
        "timeout-minutes:",
        "persist-credentials: false",
        "cargo fetch --locked",
        "scripts/check",
        "framework-neutral",
        "-p recourse",
        "-p dispatch-service",
    ] {
        assert!(workflow.contains(required), "CI is missing {required:?}");
    }
    for required in ["toolchain: \"1.96.0\"", "components: rustfmt, clippy"] {
        assert!(
            setup.contains(required),
            "Rust setup is missing {required:?}"
        );
    }
    for source in [&workflow, &setup] {
        assert_external_actions_are_pinned(source);
    }
}

#[test]
fn source_semver_gate_uses_a_frozen_local_baseline() {
    let workflow = read(&workspace_root().join(".github/workflows/ci.yml"));
    let baseline = "eb30b0659eea477fb255b5bcfcb91c11d31f758d";

    for required in [
        "source-semver:",
        "fetch-depth: 0",
        "cargo-semver-checks --version 0.49.0 --locked",
        "cargo semver-checks --package recourse\n",
        "cargo semver-checks --package recourse-axum\n",
        baseline,
    ] {
        assert!(
            workflow.contains(required),
            "source SemVer CI is missing {required:?}"
        );
    }
    assert_eq!(workflow.matches(baseline).count(), 2);
}

#[test]
fn scheduled_fuzzing_uses_pinned_tools_and_the_full_target_set() {
    let workspace = workspace_root();
    let workflow = read(&workspace.join(".github/workflows/fuzz.yml"));
    let script = read(&workspace.join("scripts/fuzz-long"));

    for required in [
        "schedule:",
        "workflow_dispatch:",
        "nightly-2026-08-01",
        "cargo-fuzz --version 0.13.2 --locked",
        "scripts/fuzz-long",
    ] {
        assert!(
            workflow.contains(required),
            "fuzz CI is missing {required:?}"
        );
    }
    for target in [
        "received_problem",
        "received_operation",
        "received_health",
        "catalog_artifact",
        "catalog_lock",
        "catalog_schema",
        "compatibility",
        "code",
        "terminal_escape",
    ] {
        assert!(script.contains(target), "long fuzzing omits {target}");
    }
    assert_external_actions_are_pinned(&workflow);
}

#[test]
fn catalog_determinism_runs_on_three_operating_systems() {
    let workflow = read(&workspace_root().join(".github/workflows/ci.yml"));

    for required in [
        "catalog-determinism:",
        "ubuntu-24.04",
        "macos-15",
        "windows-2025",
        "bash scripts/check-dispatch-artifacts",
    ] {
        assert!(
            workflow.contains(required),
            "catalog matrix is missing {required:?}"
        );
    }
}

#[test]
fn toolchain_and_workspace_rust_version_remain_aligned() {
    let workspace = workspace_root();
    let toolchain = read(&workspace.join("rust-toolchain.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse rust-toolchain.toml: {error}"));
    let manifest = read(&workspace.join("Cargo.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse Cargo.toml: {error}"));

    assert_eq!(toolchain["toolchain"]["channel"].as_str(), Some("1.96.0"));
    assert_eq!(
        manifest["workspace"]["package"]["rust-version"].as_str(),
        Some("1.96")
    );
}

fn assert_external_actions_are_pinned(source: &str) {
    for line in source.lines().map(str::trim) {
        let Some(action) = line.strip_prefix("- uses: ") else {
            continue;
        };
        if action.starts_with("./") {
            continue;
        }
        let revision = action.split_once('@').map_or("", |(_, value)| value);
        assert!(
            revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "external action must use a full commit SHA: {action}"
        );
    }
}

fn collect_directories(workspace: &Path, root: &Path, directories: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(root).unwrap_or_else(|error| panic!("read {}: {error}", root.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("read entry under {}: {error}", root.display()))
            .path();
        if !path.is_dir() {
            continue;
        }
        if path == workspace.join(".git") || path == workspace.join("target") {
            continue;
        }
        if path.file_name().is_some_and(|name| name == ".git") {
            directories.push(path);
            continue;
        }
        directories.push(path.clone());
        collect_directories(workspace, &path, directories);
    }
}
