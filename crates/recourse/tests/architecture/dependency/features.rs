//! Dependency features are explicit and cannot alter canonical output.

use crate::support::{read, workspace_root};

#[test]
fn axum_adapter_enables_only_its_owned_framework_capability() {
    let workspace = workspace_root();
    let root = read(&workspace.join("Cargo.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse workspace manifest: {error}"));
    assert_eq!(
        root["workspace"]["dependencies"]["axum"]["default-features"].as_bool(),
        Some(false)
    );
    assert_eq!(
        root["workspace"]["dependencies"]["tower"]["default-features"].as_bool(),
        Some(false)
    );

    let adapter = read(&workspace.join("crates/recourse-axum/Cargo.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse adapter manifest: {error}"));
    assert_eq!(
        adapter["dependencies"]["axum"]["features"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        adapter["dependencies"]["axum"]["features"][0].as_str(),
        Some("matched-path")
    );
}

#[test]
fn schema_validation_is_offline_by_construction() {
    let workspace = workspace_root();
    let root = read(&workspace.join("Cargo.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse workspace manifest: {error}"));
    assert_eq!(
        root["workspace"]["dependencies"]["jsonschema"]["default-features"].as_bool(),
        Some(false)
    );
}

#[test]
fn serde_json_numeric_identity_features_are_owned_directly() {
    let workspace = workspace_root();
    let root = read(&workspace.join("Cargo.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse workspace manifest: {error}"));
    let features = root["workspace"]["dependencies"]["serde_json"]["features"]
        .as_array()
        .unwrap_or_else(|| panic!("serde_json features must be explicit"));
    assert_eq!(features.len(), 2);
    for required in ["arbitrary_precision", "float_roundtrip"] {
        assert!(
            features
                .iter()
                .any(|feature| feature.as_str() == Some(required)),
            "serde_json dependency omits {required}"
        );
    }
}

#[test]
fn preserve_order_consumers_must_match_default_canonical_bytes() {
    let workspace = workspace_root();
    let preserve = read(&workspace.join("conformance/canonical-json/preserve-order/Cargo.toml"))
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse preserve-order manifest: {error}"));
    for dependency in ["schemars", "serde_json"] {
        assert_eq!(
            preserve["dependencies"][dependency]["features"][0].as_str(),
            Some("preserve_order")
        );
    }
    let script = read(&workspace.join("scripts/check-dispatch-artifacts"));
    for required in [
        "recourse-canonical-default-consumer",
        "recourse-canonical-preserve-order-consumer",
        "canonical JSON changed under downstream preserve_order features",
    ] {
        assert!(script.contains(required), "canonical gate omits {required}");
    }
}

#[test]
fn raw_json_feature_consumers_must_fail_closed() {
    let workspace = workspace_root();
    for (directory, feature) in [
        ("raw-value", "raw_value"),
        ("arbitrary-precision", "arbitrary_precision"),
    ] {
        let manifest = read(
            &workspace
                .join("conformance/canonical-json")
                .join(directory)
                .join("Cargo.toml"),
        )
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse {directory} manifest: {error}"));
        assert_eq!(
            manifest["dependencies"]["serde_json"]["features"][0].as_str(),
            Some(feature)
        );
    }
    let script = read(&workspace.join("scripts/check-dispatch-artifacts"));
    for required in [
        "recourse-raw-value-consumer",
        "recourse-arbitrary-precision-consumer",
    ] {
        assert!(script.contains(required), "canonical gate omits {required}");
    }
}
