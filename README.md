<p align="center">
  <img src="./recourse-logo.svg" alt="recourse" width="720">
</p>

<p align="center">
  <strong>Versioned failure contracts for Rust services.</strong>
</p>

<p align="center">
  Declare a failure once. Recourse keeps its code, schema, HTTP behavior,
  documentation, and compatibility history in sync—without exposing private
  error context.
</p>

<p align="center">
  <a href="https://github.com/zsumz/recourse/actions/workflows/ci.yml"><img src="https://github.com/zsumz/recourse/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
</p>

<p align="center">
  <a href="#install">Install</a>
  <span> · </span>
  <a href="#five-minute-loop">Five-minute loop</a>
  <span> · </span>
  <a href="#model">Model</a>
  <span> · </span>
  <a href="#why-recourse">Why Recourse</a>
  <span> · </span>
  <a href="#packages">Packages</a>
  <span> · </span>
  <a href="#cli">CLI</a>
</p>

<br />

## Install

Pin release candidates exactly:

```sh
cargo add recourse@=0.0.1-rc.3
```

Add the Axum adapter only at the application boundary:

```sh
cargo add recourse-axum@=0.0.1-rc.3
```

The core package has no async runtime or application-framework dependency.

## Five-minute loop

Start with typed public evidence and one permanent declaration. The complete
compiling example lives in the
[Dispatch reference service](./reference/dispatch-diagnostics/src/problem/job_not_found.rs).

```rust
#[derive(Debug, Serialize, JsonSchema)]
pub struct JobNotFoundEvidence {
    pub job_id: JobId,
}

impl PublicEvidence for JobNotFoundEvidence {}

pub enum JobNotFound {}

impl DiagnosticType for JobNotFound {
    type Catalog = DispatchCatalog;
    type Evidence = JobNotFoundEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Job not found";
    const DETAIL: &'static str = "No job exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &["Check the job identifier."];
    const DOCS: &'static str = "Verify the identifier before retrying.";
}

impl HttpProblemType for JobNotFound {
    type Policy = Fixed<404>;
}
```

Register it explicitly, then return it through the Axum request context:

```rust
let catalog = Catalog::<DispatchCatalog>::builder()
    .problem::<JobNotFound>()
    .build()?;

Err(problems.problem::<JobNotFound>(JobNotFoundEvidence { job_id }))
```

The full registration and handler are
[`catalog.rs`](./reference/dispatch-diagnostics/src/catalog.rs) and
[`jobs.rs`](./reference/dispatch-api-axum/src/jobs.rs). Emit and accept the
initial catalog once:

```sh
cargo install recourse-cli --version 0.0.1-rc.3 --locked
cargo run -p dispatch-catalog > diagnostics/catalog.json
cargo recourse accept \
  --current diagnostics/catalog.json \
  --lock diagnostics/catalog.lock
```

Run the reference API and request an unknown job:

```sh
cargo run -p dispatch-api-axum
curl -i \
  -H 'authorization: Bearer dispatch-demo' \
  http://127.0.0.1:3000/jobs/job_01K00000000000000000000000
```

The response is a strict RFC 9457 Problem with the governed identity and typed
evidence:

```json
{
  "type": "https://dispatch.invalid/problems/DSP-1003",
  "title": "Job not found",
  "status": 404,
  "code": "DSP-1003",
  "evidence": {
    "job_id": "job_01K00000000000000000000000"
  }
}
```

Now add a required `trace_id: String` to `JobNotFoundEvidence`, regenerate the
catalog, and check it without updating the lock:

```sh
cargo run -p dispatch-catalog > diagnostics/catalog.json
cargo recourse check \
  --current diagnostics/catalog.json \
  --lock diagnostics/catalog.lock
```

Recourse exits unsuccessfully with the contract break and its remedy:

```text
error[REC-COMPAT-013]: Existing emitters may not provide the new field.
  diagnostic  DSP-1003
  path        evidence_schema.properties.trace_id
  previous    absent
  current     required

Make it optional or mint a new code.
```

That loop—declaration, response, artifact, lock, compatibility decision—is the
product. Private source errors travel through a separate, non-serializable
reporting path and cannot become evidence by accident.

## Model

Most Problem Details libraries help return an error body. Recourse governs the
contract around a publicly observable failure as the system changes.

```text
diagnostic declarations
        │
        ▼
 explicit catalog ───────► HTTP Problems, operations, and health findings
        │
        ▼
  catalog.json ──────────► catalog.lock ──────────► compatibility checks

 source errors ──────────► PrivateReport ─────────► application reporter
```

One diagnostic identity can survive an immediate HTTP failure, a failure
recorded after asynchronous work was accepted, and a health finding describing
present system state. Each surface keeps its own envelope and semantics.

## Why Recourse

| Concern | Basic Problem crate | Recourse |
| --- | ---: | ---: |
| Serialize an RFC 9457 body | Yes | Yes |
| Permanent code and type identity | Manual | Governed |
| Typed evidence schema | Manual | Built in |
| Public/private separation | Convention | Type boundary |
| Compatibility history | No | Lock and tombstones |
| Tolerant bounded client | Usually no | Built in |
| Generated type documentation | Usually no | Built in |

Recourse is the contract layer around publicly observable failures—not a
replacement for `thiserror`, an observability backend, or only a nicer Problem
builder.

## Features

- permanent diagnostic codes and type URIs
- typed, schema-governed public evidence
- strict RFC 9457 Problem construction and HTTP policy headers
- structurally separate public Problems and private reports
- durable operation diagnostics and health findings
- bounded tolerant decoding for old and future-compatible payloads
- deterministic catalog artifacts, append-only locks, and compatibility checks

## Packages

| Package | Role |
| --- | --- |
| [`recourse`](./crates/recourse/) | Framework-neutral protocol, catalog, encoding, and decoding |
| [`recourse-axum`](./crates/recourse-axum/) | Axum request context, lifecycle handling, and response translation |
| [`recourse-cli`](./crates/recourse-cli/) | Catalog checks, lock updates, explanations, and generated diagnostic pages |

The framework-neutral and Axum [Dispatch reference packages](./reference/)
exercise the public APIs across HTTP, background work, CLI, catalog, and
compatibility boundaries.

## CLI

Installing `recourse-cli` provides the `cargo-recourse` binary and the
`cargo recourse` subcommand:

```sh
cargo install recourse-cli --version 0.0.1-rc.3 --locked
cargo recourse check --current diagnostics/catalog.json --lock diagnostics/catalog.lock
cargo recourse explain --current diagnostics/catalog.json DSP-1003
```

## Verification

Run the complete repository and package gate with:

```sh
scripts/check
```

The gate tests, lints, packages, extracts, installs, and runs every publishable
crate. Smoque drives an external consumer built only from packaged archives
over real HTTP, including the terminal SSE failure path. This edge check
requires Node 22.18 or newer. The gate does not publish or push.

## Project

[Changelog](./CHANGELOG.md) · [Contributing](./CONTRIBUTING.md) ·
[Support](./SUPPORT.md) · [Security](./SECURITY.md) ·
[Releasing](./RELEASING.md)

## License

MIT. See [LICENSE](./LICENSE).
