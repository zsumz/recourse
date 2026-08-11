//! Signed tags are the only path to verified GitHub release artifacts.

use super::{
    repository::assert_external_actions_are_pinned,
    support::{read, workspace_root},
};

#[test]
fn signed_tags_verify_packages_before_github_release() {
    let workspace = workspace_root();
    let workflow = read(&workspace.join(".github/workflows/release.yml"));
    let verification = read(&workspace.join("scripts/verify-release-tag"));
    let key = read(&workspace.join("etc/release-signing-key.asc"));

    for required in [
        "tags:",
        "scripts/verify-release-tag",
        "scripts/check",
        "target/package/*.crate",
        "SHA256SUMS",
        "actions/upload-artifact@",
        "actions/download-artifact@",
        "gh release create",
        "--prerelease",
    ] {
        assert!(
            workflow.contains(required),
            "tag release workflow is missing {required:?}"
        );
    }
    for required in [
        "git verify-tag",
        "git verify-commit",
        "origin/main",
        "zsumz <shawn@zsumz.com>",
    ] {
        assert!(
            verification.contains(required),
            "release identity check is missing {required:?}"
        );
    }
    assert!(key.contains("BEGIN PGP PUBLIC KEY BLOCK"));
    assert_external_actions_are_pinned(&workflow);
}
