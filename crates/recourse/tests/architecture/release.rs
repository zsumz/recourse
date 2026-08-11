//! Release identity and packaged-consumer behavior remain executable.

use std::path::Path;

use super::support::{read, workspace_root};

const PUBLIC_PACKAGES: [(&str, &str); 3] = [
    ("recourse", "crates/recourse"),
    ("recourse-axum", "crates/recourse-axum"),
    ("recourse-cli", "crates/recourse-cli"),
];

#[test]
fn release_inventory_matches_public_manifests() {
    let workspace = workspace_root();
    let release = parse(&workspace.join("release.toml"));
    assert_eq!(release["schema"].as_integer(), Some(1));
    assert_eq!(release["version"].as_str(), Some("0.0.1"));
    let packages = release["package"]
        .as_array()
        .unwrap_or_else(|| panic!("release package inventory must be an array"));
    assert_eq!(packages.len(), PUBLIC_PACKAGES.len());

    for (entry, (name, path)) in packages.iter().zip(PUBLIC_PACKAGES) {
        assert_eq!(entry["name"].as_str(), Some(name));
        assert_eq!(entry["path"].as_str(), Some(path));
        assert_public_manifest(&workspace.join(path).join("Cargo.toml"), name);
    }
    assert_eq!(packages[2]["binary"].as_str(), Some("cargo-recourse"));
}

#[test]
fn workspace_and_public_dependencies_use_release_version() {
    let workspace = workspace_root();
    let manifest = parse(&workspace.join("Cargo.toml"));
    let package = &manifest["workspace"]["package"];
    assert_eq!(package["version"].as_str(), Some("0.0.1"));
    assert_eq!(package["rust-version"].as_str(), Some("1.96"));
    for dependency in ["recourse", "recourse-axum"] {
        assert_eq!(
            manifest["workspace"]["dependencies"][dependency]["version"].as_str(),
            Some("0.0.1"),
            "{dependency} workspace dependency version drifted"
        );
    }
}

#[test]
fn canonical_gate_proves_extracted_packages_and_installed_cli() {
    let workspace = workspace_root();
    let canonical = read(&workspace.join("scripts/check"));
    let packages = read(&workspace.join("scripts/check-packages"));
    let consumer = read(&workspace.join("smoke/ballast-consumer/src/main.rs"));

    assert!(canonical.contains("scripts/check-packages"));
    for required in [
        "cargo \"${package_args[@]}\"",
        "tar -xzf",
        "cargo run --manifest-path",
        "cargo install",
        "cargo-recourse",
        "--offline",
    ] {
        assert!(
            packages.contains(required),
            "package gate omits {required:?}"
        );
    }
    for required in [
        "BallastCatalog",
        "DeploymentNotFound",
        "application/problem+json",
        "PRIVATE_CANARY",
        "FaultReporter",
    ] {
        assert!(
            consumer.contains(required),
            "consumer smoke omits {required:?}"
        );
    }
}

#[test]
fn crate_page_readmes_contain_first_use_instructions() {
    let workspace = workspace_root();
    let expectations = [
        ("recourse", "recourse = \"0.0.1\""),
        ("recourse-axum", "recourse-axum = \"0.0.1\""),
        (
            "recourse-cli",
            "cargo install recourse-cli --version 0.0.1 --locked",
        ),
    ];
    for (package, expected) in expectations {
        let readme = read(&workspace.join("crates").join(package).join("README.md"));
        assert!(
            readme.contains(expected),
            "{package} README omits {expected:?}"
        );
    }
}

fn assert_public_manifest(path: &Path, name: &str) {
    let manifest = parse(path);
    let package = &manifest["package"];
    assert_eq!(package["name"].as_str(), Some(name));
    assert_eq!(package["version"]["workspace"].as_bool(), Some(true));
    assert_ne!(
        package.get("publish").and_then(toml::Value::as_bool),
        Some(false)
    );
    for field in [
        "description",
        "documentation",
        "readme",
        "keywords",
        "categories",
    ] {
        assert!(package.get(field).is_some(), "{name} omits package.{field}");
    }
}

fn parse(path: &Path) -> toml::Value {
    read(path)
        .parse()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}
