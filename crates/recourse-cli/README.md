# recourse-cli

`recourse-cli` provides the `cargo-recourse` binary, which governs checked
catalog artifacts without compiling or executing application code. Cargo
invokes that binary as its `recourse` subcommand.

```console
cargo install recourse-cli --version 0.0.1-rc.2 --locked
cargo recourse --help
```

```console
cargo recourse check --current diagnostics/catalog.json --lock diagnostics/catalog.lock
cargo recourse accept --current diagnostics/catalog.json --lock diagnostics/catalog.lock
cargo recourse reserve --lock diagnostics/catalog.lock
cargo recourse retire --lock diagnostics/catalog.lock DSP-1004 --reason "Superseded" --replacement DSP-1017
cargo recourse explain --current diagnostics/catalog.json DSP-1004
cargo recourse docs --current diagnostics/catalog.json --lock diagnostics/catalog.lock --out docs/problems
```

Compatibility failures have stable `REC-COMPAT-*` identities, human guidance,
and structured output under `--format json`. Breaking changes require
`--acknowledge-breaking`; forbidden namespace or tombstone violations can never
be accepted.

The package name follows the `recourse-*` family. The executable keeps Cargo's
subcommand convention: installing `recourse-cli` provides `cargo-recourse`,
which users invoke as `cargo recourse`.

See the [repository](https://github.com/zsumz/recourse) for executable catalog
fixtures, compatibility tests, and the Dispatch reference packages.
