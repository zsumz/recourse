//! Core protocol, framework adapter, and reference packages keep distinct ownership.

use std::{collections::BTreeSet, path::Path};

use super::support::{read, workspace_root};

const FRAMEWORK_CAPABILITIES: &[&str] = &[
    "actix-web",
    "axum",
    "poem",
    "rocket",
    "tokio",
    "tonic",
    "tower",
];

#[test]
fn core_production_dependencies_are_framework_and_runtime_neutral() {
    let workspace = workspace_root();
    let actual = dependencies(
        &workspace.join("crates/recourse/Cargo.toml"),
        &["dependencies"],
    );
    let expected = BTreeSet::from([
        "fluent-uri".to_owned(),
        "http".to_owned(),
        "httpdate".to_owned(),
        "schemars".to_owned(),
        "serde".to_owned(),
        "serde_json".to_owned(),
        "time".to_owned(),
    ]);

    assert_eq!(actual, expected);
    assert!(
        FRAMEWORK_CAPABILITIES
            .iter()
            .all(|name| !actual.contains(*name))
    );
}

#[test]
fn framework_neutral_dispatch_packages_do_not_reach_the_axum_adapter() {
    let workspace = workspace_root();
    for package in [
        "dispatch-diagnostics",
        "dispatch-service",
        "dispatch-worker",
        "dispatch-cli",
        "dispatch-catalog",
    ] {
        let actual = dependencies(
            &workspace.join("reference").join(package).join("Cargo.toml"),
            &["dependencies", "dev-dependencies", "build-dependencies"],
        );
        assert!(
            !actual.contains("recourse-axum"),
            "{package} reaches recourse-axum"
        );
        for capability in FRAMEWORK_CAPABILITIES {
            assert!(
                !actual.contains(*capability),
                "{package} reaches {capability}"
            );
        }
    }
}

#[test]
fn axum_adapter_is_the_framework_integration_leaf() {
    let workspace = workspace_root();
    let actual = dependencies(
        &workspace.join("crates/recourse-axum/Cargo.toml"),
        &["dependencies", "dev-dependencies"],
    );

    for required in ["recourse", "axum", "tokio", "tower"] {
        assert!(actual.contains(required), "recourse-axum omits {required}");
    }
}

fn dependencies(path: &Path, sections: &[&str]) -> BTreeSet<String> {
    let manifest = read(path)
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let mut names = BTreeSet::new();
    for section in sections {
        let Some(table) = manifest.get(*section).and_then(toml::Value::as_table) else {
            continue;
        };
        for (declared, specification) in table {
            let package = specification
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(declared);
            names.insert(package.to_owned());
        }
    }
    names
}
