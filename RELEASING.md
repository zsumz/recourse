# Releasing

Recourse publishes `recourse`, `recourse-axum`, and `recourse-cli` as one exact
version cohort. Publication is manual; after crates.io has the complete cohort,
the signed-tag workflow creates the immutable verification receipt and GitHub
release from the registry's exact archive bytes.

## Prepare

1. Update the workspace version, exact public dependency pins, `release.toml`,
   package READMEs, lockfiles, and the changelog.
2. Run `scripts/check` from a clean checkout.
3. Run the supply-chain policy used by CI:

   ```sh
   cargo deny --all-features check advisories licenses sources
   ```

4. Review each source-candidate `.crate` archive in `target/package/` and run
   `scripts/check-clean-tree`. These candidates prove package contents and
   behavior, but are not the later crates.io receipt.

Release commits and annotated tags are PGP-signed by
`zsumz <shawn@zsumz.com>`. The release workflow verifies that identity with the
public key in `etc/release-signing-key.asc`.

## Tag

Create the signed annotated tag that exactly matches `release.toml`, then push
the reviewed commit and tag:

```sh
git tag -s v0.0.1-rc.2 -m "recourse v0.0.1-rc.2"
git push origin main
git push origin v0.0.1-rc.2
```

## Publish to crates.io

Publish the signed tag's checkout in the order recorded by `release.toml`:

```sh
cargo publish -p recourse --locked
cargo publish -p recourse-axum --locked
cargo publish -p recourse-cli --locked
```

Wait for each package to become available in the registry before publishing
its dependent package. Never use `--no-verify` for publication.

## Verify and create the GitHub release

After all three packages are visible on crates.io, dispatch the release
workflow from `main`:

```sh
gh workflow run release.yml --ref main -f tag=v0.0.1-rc.2
```

The workflow checks out the tag, verifies the tag and commit signatures plus
`origin/main` ancestry, proves the checkout stays clean through the canonical
gate, downloads the exact crates.io archives, reruns their extracted-package
and Smoque tests, and records their SHA-256 checksums. Only those downloaded
registry bytes are attached to the GitHub prerelease or release. A failed
verification creates no GitHub release and can be safely rerun after the
registry is available.

## Post-publish verification

- Confirm all three registry versions and exact normalized dependencies.
- Confirm docs.rs succeeds for both libraries and the CLI links to its README.
- Install `recourse-cli` from the registry and verify `cargo recourse --version`.
- Follow the root README from a fresh temporary project using registry
  dependencies only.
- Confirm the GitHub release contains the three exact crates.io archives and
  `SHA256SUMS`.
