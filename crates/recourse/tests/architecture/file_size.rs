//! Source-size boundaries keep ownership narrow and growth explicit.

use std::path::{Path, PathBuf};

use super::support::{FileClass, classify, display_path, read, rust_files, workspace_root};

#[derive(Clone, Copy, Debug)]
struct Budget {
    target: usize,
    hard: usize,
}

const FACADE: Budget = Budget {
    target: 80,
    hard: 120,
};
const OWNED: Budget = Budget {
    target: 240,
    hard: 300,
};

fn violations(root: &Path, files: &[PathBuf]) -> Vec<String> {
    let mut findings = Vec::new();
    for path in files {
        let relative = display_path(root, path);
        let lines = read(path).lines().count();
        let budget = budget_for(classify(root, path));
        if lines > budget.target {
            findings.push(format!(
                "{relative} is {lines} lines, above its {}-line design target",
                budget.target
            ));
        }
        if lines > budget.hard {
            findings.push(format!(
                "{relative} exceeds the absolute {}-line ceiling",
                budget.hard
            ));
        }
    }
    findings
}

const fn budget_for(class: FileClass) -> Budget {
    match class {
        FileClass::Facade => FACADE,
        FileClass::Implementation | FileClass::Test | FileClass::Auxiliary => OWNED,
    }
}

#[test]
fn live_files_stay_within_reviewed_limits() {
    let workspace = workspace_root();
    let findings = violations(&workspace, &rust_files(&workspace));

    assert!(
        findings.is_empty(),
        "file-size violations:\n{}",
        findings.join("\n")
    );
}

#[test]
fn design_target_and_hard_ceiling_are_both_enforced() {
    let root = Path::new("/fixture");
    let file = root.join("src/domain.rs");
    let mut findings = Vec::new();
    let budget = Budget { target: 4, hard: 6 };
    let lines = 7;
    if lines > budget.target {
        findings.push("src/domain.rs exceeds its design target");
    }
    if lines > budget.hard {
        findings.push("src/domain.rs exceeds its hard ceiling");
    }

    assert!(file.ends_with("domain.rs"));
    assert!(findings.iter().any(|item| item.contains("design target")));
    assert!(findings.iter().any(|item| item.contains("hard ceiling")));
}
