//! Release identity and packaged-consumer behavior remain executable.

use std::path::Path;

use super::support::{read, workspace_root};

const PUBLIC_PACKAGES: [(&str, &str); 3] = [
    ("recourse", "crates/recourse"),
    ("recourse-axum", "crates/recourse-axum"),
    ("recourse-cli", "crates/recourse-cli"),
];
const RELEASE_VERSION: &str = "0.0.1-rc.2";

#[test]
fn release_inventory_matches_public_manifests() {
    let workspace = workspace_root();
    let release = parse(&workspace.join("release.toml"));
    assert_eq!(release["schema"].as_integer(), Some(1));
    assert_eq!(release["version"].as_str(), Some(RELEASE_VERSION));
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
    let release = parse(&workspace.join("release.toml"));
    let release_version = release["version"]
        .as_str()
        .unwrap_or_else(|| panic!("release version must be a string"));
    let manifest = parse(&workspace.join("Cargo.toml"));
    let package = &manifest["workspace"]["package"];
    assert_eq!(package["version"].as_str(), Some(release_version));
    assert_eq!(package["rust-version"].as_str(), Some("1.96"));
    let exact_release = format!("={release_version}");
    for dependency in ["recourse", "recourse-axum"] {
        assert_eq!(
            manifest["workspace"]["dependencies"][dependency]["version"].as_str(),
            Some(exact_release.as_str()),
            "{dependency} workspace dependency version drifted"
        );
    }
}

#[test]
fn canonical_gate_proves_extracted_packages_through_smoque() {
    let workspace = workspace_root();
    let canonical = read(&workspace.join("scripts/check"));
    let packages = read(&workspace.join("scripts/check-packages"));
    let archives = read(&workspace.join("scripts/check-package-archives"));
    let launcher = read(&workspace.join("scripts/smoke-smoque"));
    let smoke = read(&workspace.join("smoke/package.smoke.mts"));

    assert!(canonical.contains("scripts/check-packages"));
    for required in ["cargo \"${package_args[@]}\"", "--offline"] {
        assert!(
            packages.contains(required),
            "package gate omits {required:?}"
        );
    }
    for required in [
        "tar -xzf",
        "cargo test --manifest-path",
        "package_test_target=\"$package_work/self-tests\"",
        "RECOURSE_PACKAGE_TARGET=$package_work/consumer",
        "RECOURSE_CORE_PACKAGE",
        "scripts/smoke-smoque smoke/package.smoke.mts --ci",
    ] {
        assert!(
            archives.contains(required),
            "archive gate omits {required:?}"
        );
    }
    for required in [
        "smoque@0.1.2",
        "sha512-tV8g4sT6HbGNEIknfPTJiD14kXzsTEgImPvX+1Y4QIY2bdUQ2eAZQBTO5EJqnl9fMxVzJ4oHFba7fEtAxyXNCw==",
        "npm pack --json --ignore-scripts",
        "actual_integrity",
        "npx --yes --package=\"$stage/$archive\" smoque",
    ] {
        assert!(
            launcher.contains(required),
            "Smoque launcher omits {required:?}"
        );
    }
    for required in [
        "from \"smoque\"",
        "t.process.start",
        "t.http.get",
        "cargo-recourse",
        "application/problem+json",
        "PRIVATE_CANARY",
        "event-stream",
    ] {
        assert!(
            smoke.contains(required),
            "Smoque package smoke omits {required:?}"
        );
    }
}

#[test]
fn published_core_excludes_repository_wide_architecture_tests() {
    let manifest = parse(&workspace_root().join("crates/recourse/Cargo.toml"));
    let excludes = manifest["package"]["exclude"]
        .as_array()
        .unwrap_or_else(|| panic!("recourse package must declare exclusions"))
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();

    assert_eq!(
        excludes,
        [
            "tests/architecture.rs",
            "tests/architecture/**",
            "tests/support/**",
        ]
    );
}

#[test]
fn crate_page_readmes_contain_first_use_instructions() {
    let workspace = workspace_root();
    let expectations = [
        ("recourse", format!("recourse = \"={RELEASE_VERSION}\"")),
        (
            "recourse-axum",
            format!("recourse-axum = \"={RELEASE_VERSION}\""),
        ),
        (
            "recourse-cli",
            format!("cargo install recourse-cli --version {RELEASE_VERSION} --locked"),
        ),
    ];
    for (package, expected) in expectations {
        let readme = read(&workspace.join("crates").join(package).join("README.md"));
        assert!(
            readme.contains(&expected),
            "{package} README omits {expected:?}"
        );
    }
}

#[test]
fn binary_only_cli_documentation_points_to_its_readme() {
    let manifest = parse(&workspace_root().join("crates/recourse-cli/Cargo.toml"));

    assert!(manifest.get("lib").is_none());
    assert_eq!(
        manifest["package"]["documentation"].as_str(),
        Some("https://github.com/zsumz/recourse/tree/main/crates/recourse-cli#readme")
    );
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
