//! Signed tags and published registry bytes are the only GitHub release input.

use super::{
    repository::assert_external_actions_are_pinned,
    support::{read, workspace_root},
};

#[test]
fn signed_tags_verify_published_packages_before_github_release() {
    let workspace = workspace_root();
    let workflow = read(&workspace.join(".github/workflows/release.yml"));
    let verification = read(&workspace.join("scripts/verify-release-tag"));
    let published = read(&workspace.join("scripts/check-published-packages"));
    let archives = read(&workspace.join("scripts/check-package-archives"));
    let key = read(&workspace.join("etc/release-signing-key.asc"));

    for required in [
        "workflow_dispatch:",
        "if: github.ref == 'refs/heads/main'",
        "ref: ${{ github.sha }}",
        "path: trusted",
        "ref: ${{ inputs.tag }}",
        "path: candidate",
        "RECOURSE_RELEASE_FINGERPRINT: B58439871CD2A7275B20CC19EC8E4D26598A0373",
        "../trusted/etc/release-signing-key.asc",
        "../trusted/scripts/verify-release-tag",
        "--import-options show-only",
        "scripts/check-clean-tree",
        "scripts/check",
        "scripts/check-published-packages",
        "registry-packages/*.crate",
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
    for required in ["required_files+=(Cargo.lock)", "--locked --quiet"] {
        assert!(
            archives.contains(required),
            "published archive verification is missing {required:?}"
        );
    }
    for required in [
        "https://crates.io/api/v1/crates/",
        "scripts/check-package-archives",
        "--retry-all-errors",
        "recourse-release-verifier/0.0.1",
    ] {
        assert!(
            published.contains(required),
            "published package check is missing {required:?}"
        );
    }
    for required in [
        "verify-$kind",
        "VALIDSIG",
        "expected_fingerprint",
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

#[test]
fn release_guide_reproduces_the_frozen_api_snapshot() {
    let guide = read(&workspace_root().join("RELEASING.md"));
    for required in [
        "git tag -s api/v0.0.1-rc.2 ed742880b9edd7b692b5dfb585c07c5ceeb7fd43",
        "git rev-parse 'api/v0.0.1-rc.2^{commit}'",
        "git push origin api/v0.0.1-rc.2",
        "Do not move or recreate an API snapshot tag",
    ] {
        assert!(guide.contains(required), "release guide omits {required:?}");
    }
}
