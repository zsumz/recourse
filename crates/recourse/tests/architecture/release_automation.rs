//! Signed tags and published registry bytes are the only GitHub release input.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

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
        "git tag -s api/v0.0.1-rc.3 c14b100c4dae2c859520fbdf427dbb785c9b0990",
        "git rev-parse 'api/v0.0.1-rc.3^{commit}'",
        "git push origin api/v0.0.1-rc.3",
        "Do not move or recreate an API snapshot tag",
    ] {
        assert!(guide.contains(required), "release guide omits {required:?}");
    }
}

#[test]
fn git_tagger_identity_format_matches_the_release_contract() {
    let repository = tagger_test_repository();
    let identity = git(
        &repository,
        &[
            "for-each-ref",
            "--format=%(taggername) %(taggeremail)",
            "refs/tags/test-release",
        ],
    );
    assert_git_success(&identity, "read tagger identity");
    assert_eq!(
        String::from_utf8(identity.stdout)
            .unwrap_or_else(|error| panic!("Git tagger identity should be UTF-8: {error}"))
            .trim(),
        "zsumz <shawn@zsumz.com>"
    );

    let verifier = read(&workspace_root().join("scripts/verify-release-tag"));
    assert!(verifier.contains("--format='%(taggername) %(taggeremail)'"));
    assert!(!verifier.contains("<%(taggeremail)>"));
    fs::remove_dir_all(&repository)
        .unwrap_or_else(|error| panic!("remove {}: {error}", repository.display()));
}

fn tagger_test_repository() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("system time should follow the Unix epoch: {error}"))
        .as_nanos();
    let repository = std::env::temp_dir().join(format!(
        "recourse-tagger-format-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&repository)
        .unwrap_or_else(|error| panic!("create {}: {error}", repository.display()));

    assert_git_success(
        &git(&repository, &["init", "--quiet"]),
        "initialize temporary repository",
    );
    assert_git_success(
        &git(
            &repository,
            &[
                "-c",
                "commit.gpgSign=false",
                "-c",
                "user.name=zsumz",
                "-c",
                "user.email=shawn@zsumz.com",
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                "test: seed tag target",
            ],
        ),
        "create tag target",
    );
    assert_git_success(
        &git(
            &repository,
            &[
                "-c",
                "tag.gpgSign=false",
                "-c",
                "user.name=zsumz",
                "-c",
                "user.email=shawn@zsumz.com",
                "tag",
                "--annotate",
                "test-release",
                "-m",
                "test release",
            ],
        ),
        "create annotated tag",
    );
    repository
}

fn git(repository: &Path, arguments: &[&str]) -> Output {
    Command::new("git")
        .current_dir(repository)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("run Git in {}: {error}", repository.display()))
}

fn assert_git_success(output: &Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
