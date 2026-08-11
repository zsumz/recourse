//! Rust source stays domain-shaped, documented, and explicit about failure.

use std::path::{Path, PathBuf};

use super::support::{FileClass, classify, display_path, read, rust_files, workspace_root};
use syn::{
    Attribute, ExprMethodCall, Item, Macro,
    visit::{self, Visit},
};

const GENERIC_MODULE_NAMES: &[&str] = &["common", "helpers", "misc", "utils"];

fn violations(root: &Path, files: &[PathBuf]) -> Vec<String> {
    let mut findings = Vec::new();
    for path in files {
        let source = read(path);
        let relative = display_path(root, path);
        if !source
            .trim_start_matches(|value: char| value.is_whitespace() || value == '\u{feff}')
            .starts_with("//!")
        {
            findings.push(format!("{relative} must begin with a //! module contract"));
        }
        check_generic_name(&relative, &mut findings);

        let syntax =
            syn::parse_file(&source).unwrap_or_else(|error| panic!("parse {relative}: {error}"));
        let class = classify(root, path);
        if class == FileClass::Facade {
            check_facade(&relative, &syntax, &mut findings);
        }
        if !matches!(class, FileClass::Test | FileClass::Auxiliary) {
            let mut visitor = ProductionVisitor::default();
            visitor.visit_file(&syntax);
            for finding in visitor.findings {
                findings.push(format!("{relative} {finding}"));
            }
        }
    }
    check_sibling_test_declarations(root, files, &mut findings);
    findings
}

fn check_generic_name(relative: &str, findings: &mut Vec<String>) {
    let normalized = relative.trim_end_matches(".rs");
    for part in normalized.split('/') {
        if GENERIC_MODULE_NAMES.contains(&part) {
            findings.push(format!(
                "{relative} uses generic module name {part}; name the owned domain"
            ));
        }
    }
}

fn check_facade(relative: &str, syntax: &syn::File, findings: &mut Vec<String>) {
    let is_main = relative.ends_with("/main.rs");
    for item in &syntax.items {
        let allowed = matches!(item, Item::Mod(_) | Item::Use(_) | Item::ExternCrate(_))
            || is_main && matches!(item, Item::Fn(function) if function.sig.ident == "main");
        if !allowed {
            findings.push(format!(
                "{relative} contains facade implementation {}",
                item_kind(item)
            ));
        }
    }
}

const fn item_kind(item: &Item) -> &'static str {
    match item {
        Item::Const(_) => "const",
        Item::Enum(_) => "enum",
        Item::Fn(_) => "function",
        Item::Impl(_) => "impl",
        Item::Static(_) => "static",
        Item::Struct(_) => "struct",
        Item::Trait(_) => "trait",
        Item::Type(_) => "type",
        _ => "item",
    }
}

fn check_sibling_test_declarations(root: &Path, files: &[PathBuf], findings: &mut Vec<String>) {
    for path in files.iter().filter(|path| {
        path.file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.ends_with("_test.rs"))
            && path.components().any(|part| part.as_os_str() == "src")
    }) {
        let relative = display_path(root, path);
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(parent) = path.parent() else {
            findings.push(format!("{relative} has no parent module"));
            continue;
        };
        let facade = ["mod.rs", "lib.rs", "main.rs"]
            .iter()
            .map(|name| parent.join(name))
            .find(|candidate| candidate.is_file());
        let Some(facade) = facade else {
            findings.push(format!("{relative} has no sibling facade"));
            continue;
        };
        let syntax = syn::parse_file(&read(&facade))
            .unwrap_or_else(|error| panic!("parse {}: {error}", facade.display()));
        let declared = syntax.items.iter().any(|item| {
            let Item::Mod(module) = item else {
                return false;
            };
            module.ident == stem && module.content.is_none() && module.attrs.iter().any(is_cfg_test)
        });
        if !declared {
            findings.push(format!(
                "{relative} must be declared by its facade with #[cfg(test)]"
            ));
        }
    }
}

#[derive(Default)]
struct ProductionVisitor {
    findings: Vec<&'static str>,
}

impl<'ast> Visit<'ast> for ProductionVisitor {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if attribute.path().is_ident("allow") || attribute.path().is_ident("expect") {
            self.findings.push("uses a lint suppression");
        }
        visit::visit_attribute(self, attribute);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        match call.method.to_string().as_str() {
            "expect" => self.findings.push("uses expect()"),
            "unwrap" => self.findings.push("uses unwrap()"),
            _ => {}
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_macro(&mut self, invocation: &'ast Macro) {
        let name = invocation
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        if name.as_deref().is_some_and(|value| {
            matches!(
                value,
                "dbg" | "panic" | "todo" | "unimplemented" | "unreachable"
            )
        }) {
            self.findings.push("uses a forbidden macro");
        }
        visit::visit_macro(self, invocation);
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if module.content.is_some() && module.attrs.iter().any(is_cfg_test) {
            self.findings.push("contains an inline #[cfg(test)] module");
        }
        visit::visit_item_mod(self, module);
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        if function.attrs.iter().any(is_test_attribute) {
            self.findings
                .push("contains a test function in production source");
        }
        if function.sig.unsafety.is_some() {
            self.findings.push("declares an unsafe function");
        }
        visit::visit_item_fn(self, function);
    }

    fn visit_expr_unsafe(&mut self, expression: &'ast syn::ExprUnsafe) {
        self.findings.push("contains an unsafe block");
        visit::visit_expr_unsafe(self, expression);
    }
}

fn is_cfg_test(attribute: &Attribute) -> bool {
    let syn::Meta::List(list) = &attribute.meta else {
        return false;
    };
    list.path.is_ident("cfg") && list.tokens.to_string() == "test"
}

fn is_test_attribute(attribute: &Attribute) -> bool {
    is_cfg_test(attribute)
        || attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
}

#[test]
fn live_source_has_domain_shape_and_separate_tests() {
    let workspace = workspace_root();
    let findings = violations(&workspace, &rust_files(&workspace));

    assert!(
        findings.is_empty(),
        "source-shape violations:\n{}",
        findings.join("\n")
    );
}

#[test]
fn inline_tests_and_generic_modules_are_rejected() {
    let mut visitor = ProductionVisitor::default();
    let source = "#[cfg(test)] mod tests { #[test] fn hidden() {} }";
    let syntax = syn::parse_file(source).unwrap_or_else(|error| panic!("parse fixture: {error}"));
    visitor.visit_file(&syntax);
    let mut findings = Vec::new();
    check_generic_name("src/utils.rs", &mut findings);

    assert!(!visitor.findings.is_empty());
    assert!(findings.iter().any(|item| item.contains("generic module")));
}
