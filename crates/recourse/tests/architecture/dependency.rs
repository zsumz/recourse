//! Workspace dependency edges are exact reviewed boundaries.

#[path = "dependency/features.rs"]
mod features;

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use super::support::{manifest_paths, read, workspace_root};

#[derive(Debug)]
struct DependencyRule {
    package: &'static str,
    allowed_internal: &'static [&'static str],
    allowed_external: &'static [&'static str],
}

const RULES: &[DependencyRule] = &[
    DependencyRule {
        package: "recourse",
        allowed_internal: &[],
        allowed_external: &[
            "fluent-uri",
            "http",
            "httpdate",
            "jsonschema",
            "schemars",
            "serde",
            "serde_json",
            "syn",
            "time",
            "toml",
        ],
    },
    DependencyRule {
        package: "recourse-axum",
        allowed_internal: &["recourse"],
        allowed_external: &[
            "axum",
            "futures-util",
            "http",
            "schemars",
            "serde",
            "serde_json",
            "tokio",
            "tower",
            "ulid",
        ],
    },
    DependencyRule {
        package: "recourse-cli",
        allowed_internal: &["recourse"],
        allowed_external: &["serde_json"],
    },
    DependencyRule {
        package: "dispatch-model",
        allowed_internal: &[],
        allowed_external: &["schemars", "serde", "serde_json"],
    },
    DependencyRule {
        package: "dispatch-diagnostics",
        allowed_internal: &["dispatch-model", "recourse"],
        allowed_external: &["schemars", "serde"],
    },
    DependencyRule {
        package: "dispatch-service",
        allowed_internal: &["dispatch-diagnostics", "dispatch-model", "recourse"],
        allowed_external: &["http", "ulid"],
    },
    DependencyRule {
        package: "dispatch-api-axum",
        allowed_internal: &[
            "dispatch-diagnostics",
            "dispatch-model",
            "dispatch-service",
            "dispatch-worker",
            "recourse",
            "recourse-axum",
        ],
        allowed_external: &["axum", "serde", "serde_json", "time", "tokio", "tower"],
    },
    DependencyRule {
        package: "dispatch-worker",
        allowed_internal: &[
            "dispatch-diagnostics",
            "dispatch-model",
            "dispatch-service",
            "recourse",
        ],
        allowed_external: &["serde_json"],
    },
    DependencyRule {
        package: "dispatch-cli",
        allowed_internal: &["dispatch-diagnostics", "dispatch-model", "recourse"],
        allowed_external: &["http", "serde_json"],
    },
    DependencyRule {
        package: "dispatch-catalog",
        allowed_internal: &["dispatch-diagnostics", "recourse"],
        allowed_external: &[],
    },
];

fn violations(manifests: &[PathBuf], rules: &[DependencyRule]) -> Vec<String> {
    let packages = manifests
        .iter()
        .map(|path| manifest_package(path).0)
        .collect::<BTreeSet<_>>();
    let rules = rules
        .iter()
        .map(|rule| (rule.package, rule))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut findings = Vec::new();

    for manifest in manifests {
        let (package, dependencies) = manifest_package(manifest);
        seen.insert(package.clone());
        let Some(rule) = rules.get(package.as_str()) else {
            findings.push(format!("{package} has no reviewed dependency boundary"));
            continue;
        };
        let internal = dependencies
            .iter()
            .filter(|name| packages.contains(*name))
            .cloned()
            .collect::<BTreeSet<_>>();
        let external = dependencies
            .iter()
            .filter(|name| !packages.contains(*name))
            .cloned()
            .collect::<BTreeSet<_>>();
        compare(
            &package,
            "internal",
            &internal,
            rule.allowed_internal,
            &mut findings,
        );
        compare(
            &package,
            "external",
            &external,
            rule.allowed_external,
            &mut findings,
        );
    }
    for package in rules.keys() {
        if !seen.contains(*package) {
            findings.push(format!(
                "dependency boundary names missing package {package}"
            ));
        }
    }
    findings
}

fn compare(
    package: &str,
    kind: &str,
    actual: &BTreeSet<String>,
    allowed: &[&str],
    findings: &mut Vec<String>,
) {
    let allowed = allowed
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    for dependency in actual.difference(&allowed) {
        findings.push(format!(
            "{package} has unreviewed {kind} dependency {dependency}"
        ));
    }
    for dependency in allowed.difference(actual) {
        findings.push(format!(
            "{package} has stale allowed {kind} dependency {dependency}"
        ));
    }
}

fn manifest_package(path: &Path) -> (String, BTreeSet<String>) {
    let value = read(path)
        .parse::<toml::Value>()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let Some(package) = value["package"]["name"].as_str() else {
        panic!("{} has no package name", path.display());
    };
    let mut dependencies = BTreeSet::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        collect_dependencies(value.get(section), &mut dependencies);
    }
    (package.to_owned(), dependencies)
}

fn collect_dependencies(value: Option<&toml::Value>, dependencies: &mut BTreeSet<String>) {
    let Some(table) = value.and_then(toml::Value::as_table) else {
        return;
    };
    for (declared, specification) in table {
        let package = specification
            .get("package")
            .and_then(toml::Value::as_str)
            .unwrap_or(declared);
        dependencies.insert(package.to_owned());
    }
}

#[test]
fn live_dependency_graph_matches_reviewed_boundaries() {
    let workspace = workspace_root();
    let findings = violations(&manifest_paths(&workspace), RULES);

    assert!(
        findings.is_empty(),
        "dependency violations:\n{}",
        findings.join("\n")
    );
}

#[test]
fn an_unreviewed_edge_is_rejected() {
    let actual = ["recourse-axum".to_owned()].into_iter().collect();
    let mut findings = Vec::new();
    compare("recourse", "internal", &actual, &[], &mut findings);

    assert!(
        findings
            .iter()
            .any(|item| item.contains("unreviewed internal dependency"))
    );
}
