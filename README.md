# recourse

Failures with a way forward.

Recourse is a governed diagnostics protocol for Rust applications and services.
It keeps strict server construction separate from tolerant, bounded client
decoding and keeps public Problems structurally separate from private reports.

The workspace publishes three packages:

- `recourse`: framework-neutral protocol types, catalogs, compatibility, and
  encoding and decoding;
- `recourse-axum`: Axum request context and response translation;
- `recourse-cli`: artifact checks, lock updates, explanations, and generated
  diagnostic pages through the `cargo-recourse` binary.

The Dispatch reference packages and conformance fixtures exercise the public
APIs across HTTP, background-work, CLI, catalog, and compatibility boundaries.

Run the complete local gate with:

```sh
scripts/check
```

The gate tests, lints, packages, extracts, installs, and runs the public crates,
including a Ballast-shaped external consumer. It does not publish or push.

Recourse is available under the [MIT License](LICENSE).
