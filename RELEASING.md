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

Before opening the release pull request, create and push the signed annotated
API snapshot expected by source-SemVer CI. The target is the reviewed API
commit, not the moving branch head:

```sh
git tag -s api/v0.0.1-rc.3 c14b100c4dae2c859520fbdf427dbb785c9b0990 \
  -m "recourse api v0.0.1-rc.3"
test "$(git rev-parse 'api/v0.0.1-rc.3^{commit}')" = \
  c14b100c4dae2c859520fbdf427dbb785c9b0990
git push origin api/v0.0.1-rc.3
```

Do not move or recreate an API snapshot tag. A later API baseline gets a new
versioned tag and an intentional CI update.

Release commits and annotated tags are PGP-signed by
`zsumz <shawn@zsumz.com>`. The workflow revision on `main` pins the primary
fingerprint `B58439871CD2A7275B20CC19EC8E4D26598A0373`, imports the public key
from that trusted revision, and requires both the tag and commit to report that
exact fingerprint. The copy in the candidate checkout is documentation only;
it is never its own root of trust.

## Tag

Create the signed annotated tag that exactly matches `release.toml`, then push
the reviewed commit and tag:

```sh
git tag -s v0.0.1-rc.3 -m "recourse v0.0.1-rc.3"
git push origin main
git push origin v0.0.1-rc.3
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
gh workflow run release.yml --ref main -f tag=v0.0.1-rc.3
```

The workflow keeps two checkouts: the exact `main` revision that supplied the
workflow provides the trusted key, fingerprint, and verification script; the
requested tag supplies the candidate under test. It verifies the tag and
commit signatures plus `origin/main` ancestry, proves the candidate checkout
stays clean through the canonical gate, downloads the exact crates.io
archives, reruns their extracted-package and Smoque tests, and records their
SHA-256 checksums. Only those downloaded registry bytes are attached to the
GitHub prerelease or release. A failed verification creates no GitHub release
and can be safely rerun after the registry is available.

## Post-publish verification

- Confirm all three registry versions and exact normalized dependencies.
- Confirm docs.rs succeeds for both libraries and the CLI links to its README.
- Install `recourse-cli` from the registry and verify `cargo recourse --version`.
- Follow the root README from a fresh temporary project using registry
  dependencies only.
- Confirm the GitHub release contains the three exact crates.io archives and
  `SHA256SUMS`.
