//! Framework and runtime capabilities stay outside neutral source roots.

use std::{collections::BTreeMap, path::Path};

use super::support::{display_path, read, rust_files_under, workspace_root};
use syn::{
    ItemExternCrate, ItemUse, UseTree,
    visit::{self, Visit},
};

#[derive(Debug)]
struct CapabilityRule {
    root: &'static str,
    forbidden: &'static [&'static str],
}

const CORE_FORBIDDEN: &[&str] = &[
    "actix_web",
    "axum",
    "clap",
    "poem",
    "recourse_axum",
    "rocket",
    "tokio",
    "tonic",
    "tower",
    "tracing",
];
const ADAPTER_FORBIDDEN: &[&str] = &["axum", "recourse_axum", "tokio", "tower"];
const RULES: &[CapabilityRule] = &[
    CapabilityRule {
        root: "crates/recourse/src",
        forbidden: CORE_FORBIDDEN,
    },
    CapabilityRule {
        root: "reference/dispatch-diagnostics/src",
        forbidden: ADAPTER_FORBIDDEN,
    },
    CapabilityRule {
        root: "reference/dispatch-service/src",
        forbidden: ADAPTER_FORBIDDEN,
    },
    CapabilityRule {
        root: "reference/dispatch-worker/src",
        forbidden: ADAPTER_FORBIDDEN,
    },
    CapabilityRule {
        root: "reference/dispatch-cli/src",
        forbidden: ADAPTER_FORBIDDEN,
    },
    CapabilityRule {
        root: "reference/dispatch-catalog/src",
        forbidden: ADAPTER_FORBIDDEN,
    },
];

fn violations(root: &Path, rules: &[CapabilityRule]) -> Vec<String> {
    let mut findings = Vec::new();
    for rule in rules {
        let source_root = root.join(rule.root);
        assert!(
            source_root.is_dir(),
            "capability root {} is missing",
            source_root.display()
        );
        for path in rust_files_under(&source_root) {
            let source = read(&path);
            let syntax = syn::parse_file(&source)
                .unwrap_or_else(|error| panic!("parse {}: {error}", display_path(root, &path)));
            let mut collector = PathCollector::default();
            collector.visit_file(&syntax);
            for observed in collector.expanded_paths() {
                for forbidden in rule.forbidden {
                    if matches_path(&observed, forbidden) {
                        findings.push(format!(
                            "{} reaches forbidden capability {forbidden} through {observed}",
                            display_path(root, &path)
                        ));
                    }
                }
            }
        }
    }
    findings
}

fn matches_path(observed: &str, forbidden: &str) -> bool {
    observed == forbidden
        || observed
            .strip_prefix(forbidden)
            .is_some_and(|suffix| suffix.starts_with("::"))
}

#[derive(Default)]
struct PathCollector {
    paths: Vec<String>,
    aliases: BTreeMap<String, String>,
}

impl PathCollector {
    fn expanded_paths(self) -> Vec<String> {
        self.paths
            .iter()
            .map(|path| expand_alias(path, &self.aliases))
            .collect()
    }
}

impl<'ast> Visit<'ast> for PathCollector {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        self.paths.push(
            path.segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        );
        visit::visit_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        collect_use_paths(
            String::new(),
            &item.tree,
            &mut self.paths,
            &mut self.aliases,
        );
        visit::visit_item_use(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast ItemExternCrate) {
        let original = item.ident.to_string();
        self.paths.push(original.clone());
        if let Some((_, alias)) = &item.rename {
            self.aliases.insert(alias.to_string(), original);
        }
        visit::visit_item_extern_crate(self, item);
    }
}

fn collect_use_paths(
    prefix: String,
    tree: &UseTree,
    paths: &mut Vec<String>,
    aliases: &mut BTreeMap<String, String>,
) {
    match tree {
        UseTree::Path(path) => {
            let next = append_segment(&prefix, &path.ident.to_string());
            collect_use_paths(next, &path.tree, paths, aliases);
        }
        UseTree::Name(name) => paths.push(append_segment(&prefix, &name.ident.to_string())),
        UseTree::Rename(rename) => {
            let original = append_segment(&prefix, &rename.ident.to_string());
            paths.push(original.clone());
            aliases.insert(rename.rename.to_string(), original);
        }
        UseTree::Glob(_) => paths.push(prefix),
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_paths(prefix.clone(), item, paths, aliases);
            }
        }
    }
}

fn append_segment(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_owned()
    } else {
        format!("{prefix}::{segment}")
    }
}

fn expand_alias(path: &str, aliases: &BTreeMap<String, String>) -> String {
    let (first, suffix) = path.split_once("::").unwrap_or((path, ""));
    let Some(original) = aliases.get(first) else {
        return path.to_owned();
    };
    if suffix.is_empty() {
        original.clone()
    } else {
        format!("{original}::{suffix}")
    }
}

#[test]
fn live_source_respects_capability_ownership() {
    let workspace = workspace_root();
    let findings = violations(&workspace, RULES);

    assert!(
        findings.is_empty(),
        "capability violations:\n{}",
        findings.join("\n")
    );
}

#[test]
fn aliases_cannot_hide_a_forbidden_capability() {
    let mut collector = PathCollector::default();
    let source = "use axum as framework; fn route() { framework::serve(); }";
    let syntax = syn::parse_file(source).unwrap_or_else(|error| panic!("parse fixture: {error}"));
    collector.visit_file(&syntax);
    let paths = collector.expanded_paths();

    assert!(paths.iter().any(|path| matches_path(path, "axum")));
}
