# Support policy

## Release support

Recourse is pre-1.0. The newest published release line receives fixes; older
release candidates are retained as history but are not maintained. RC users
should pin all three public crates to one exact cohort, for example:

```toml
recourse = "=0.0.1-rc.3"
recourse-axum = "=0.0.1-rc.3"
```

Breaking source changes may occur between release candidates and will be called
out in [CHANGELOG.md](./CHANGELOG.md). Stable releases follow Cargo's SemVer
contract for their compatible release line.

## Protocol compatibility

Cargo source compatibility and Recourse catalog compatibility are separate
contracts:

- Cargo versions govern whether Rust source can update safely.
- `catalog.lock` governs public diagnostic identities, schemas, HTTP policy,
  reservations, retirements, and replacement history.

Updating the crate without accepting a changed catalog does not rewrite the
application's failure contract. Run `cargo recourse check` in CI and review any
`REC-COMPAT-*` finding before updating the lock.

## Rust versions

The current MSRV is Rust 1.96. CI tests that toolchain and the current stable
toolchain. Any MSRV increase requires a release-version change and changelog
entry.

## Getting help

Use [GitHub issues](https://github.com/zsumz/recourse/issues) for reproducible
bugs and focused feature discussions. Use the private process in
[SECURITY.md](./SECURITY.md) for suspected vulnerabilities.
