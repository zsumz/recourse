//! Dependency provenance and licensing remain explicit and fail closed.

use std::collections::BTreeSet;

use super::support::{manifest_paths, read, workspace_root};

#[test]
fn every_package_inherits_mit_and_public_sources_carry_the_text() {
    let workspace = workspace_root();
    let root_manifest = parse_manifest(&workspace.join("Cargo.toml"));
    assert_eq!(
        root_manifest["workspace"]["package"]["license"].as_str(),
        Some("MIT")
    );
    for path in manifest_paths(&workspace) {
        let manifest = parse_manifest(&path);
        assert_eq!(
            manifest["package"]["license"]["workspace"].as_bool(),
            Some(true),
            "{} must inherit the workspace license",
            path.display()
        );
    }
    let license = read(&workspace.join("LICENSE"));
    for package in ["recourse", "recourse-axum", "recourse-cli"] {
        assert_eq!(
            read(&workspace.join("crates").join(package).join("LICENSE")),
            license,
            "{package} package license drifted"
        );
    }
}

#[test]
fn supply_chain_gate_audits_advisories_licenses_and_sources() {
    let workspace = workspace_root();
    let workflow = read(&workspace.join(".github/workflows/ci.yml"));
    let policy = read(&workspace.join("deny.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse deny.toml: {error}"));

    assert!(workflow.contains("cargo install cargo-deny --version 0.20.2 --locked"));
    assert!(workflow.contains("cargo deny --all-features check advisories licenses sources"));
    assert_eq!(policy["graph"]["all-features"].as_bool(), Some(true));
    assert_license_policy(&policy);
    assert_source_policy(&policy);
}

fn assert_license_policy(policy: &toml::Value) {
    let allowed = policy["licenses"]["allow"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        allowed,
        BTreeSet::from(["Apache-2.0", "MIT", "Unicode-3.0"])
    );
    assert_eq!(policy["licenses"]["include-dev"].as_bool(), Some(true));
    assert_eq!(
        policy["licenses"]["unused-allowed-license"].as_str(),
        Some("deny")
    );
    assert_eq!(
        policy["licenses"]["unused-license-exception"].as_str(),
        Some("deny")
    );
    let exceptions = policy["licenses"]["exceptions"]
        .as_array()
        .unwrap_or_else(|| panic!("license exceptions must be an array"));
    assert_eq!(exceptions.len(), 1);
    assert_eq!(exceptions[0]["crate"].as_str(), Some("matchit"));
    assert_eq!(exceptions[0]["allow"][0].as_str(), Some("BSD-3-Clause"));
    assert_eq!(
        policy["licenses"]["private"]["ignore"].as_bool(),
        Some(false)
    );
}

fn assert_source_policy(policy: &toml::Value) {
    assert_eq!(policy["sources"]["unknown-registry"].as_str(), Some("deny"));
    assert_eq!(policy["sources"]["unknown-git"].as_str(), Some("deny"));
    assert_eq!(
        policy["sources"]["allow-registry"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        policy["sources"]["allow-git"].as_array().map(Vec::len),
        Some(0)
    );
}

fn parse_manifest(path: &std::path::Path) -> toml::Value {
    read(path)
        .parse()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}
