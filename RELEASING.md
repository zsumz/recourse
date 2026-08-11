# Releasing

Recourse publishes `recourse`, `recourse-axum`, and `recourse-cli` as one exact
version cohort. Publication is manual; a signed tag creates the immutable
verification receipt and GitHub release.

## Prepare

1. Update the workspace version, exact public dependency pins, `release.toml`,
   package READMEs, lockfiles, and the changelog.
2. Run `scripts/check` from a clean checkout.
3. Run the supply-chain policy used by CI:

   ```sh
   cargo deny --all-features check advisories licenses sources
   ```

4. Review each `.crate` archive in `target/package/` and confirm the worktree is
   clean.

Release commits and annotated tags are PGP-signed by
`zsumz <shawn@zsumz.com>`. The release workflow verifies that identity with the
public key in `etc/release-signing-key.asc`.

## Tag and verify

Create the signed annotated tag that exactly matches `release.toml`, then push
the reviewed commit and tag:

```sh
git tag -s v0.0.1-rc.2 -m "recourse v0.0.1-rc.2"
git push origin main
git push origin v0.0.1-rc.2
```

The tag workflow verifies the tag and commit signatures, `origin/main`
ancestry, the canonical gate, package archives, and SHA-256 checksums before it
creates a GitHub prerelease or release. A failed verification does not publish
anything.

## Publish to crates.io

After the tag gate succeeds, publish in the order recorded by `release.toml`:

```sh
cargo publish -p recourse --locked
cargo publish -p recourse-axum --locked
cargo publish -p recourse-cli --locked
```

Wait for each package to become available in the registry before publishing
its dependent package. Never use `--no-verify` for publication.

## Post-publish verification

- Confirm all three registry versions and exact normalized dependencies.
- Confirm docs.rs succeeds for both libraries and the CLI links to its README.
- Install `recourse-cli` from the registry and verify `cargo recourse --version`.
- Follow the root README from a fresh temporary project using registry
  dependencies only.
- Confirm the GitHub release contains the three verified archives and
  `SHA256SUMS`.
