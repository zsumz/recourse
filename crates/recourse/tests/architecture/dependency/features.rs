//! Framework adapters activate only capabilities they own.

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
