# Changelog

All notable changes to Recourse are documented here. The three public crates
ship as one versioned cohort.

## [0.0.1-rc.2] - Unreleased

### Migrating from rc.1

- Replace `Unauthorized` with `BearerUnauthorized`.
- Replace `RetryAfter::At(time)` and `RetryAfter::at(time)` with
  `RetryAfter::try_at(time)?`. Use `RetryAfter::after(duration)` instead of the
  former `After` variant, and clone rather than copy stored retry values.
- Replace `DEFAULT_PUBLIC_TEXT_BYTES` and `PublicText::with_max_bytes` with
  `DEFAULT_PUBLIC_TEXT_CHARS` and `PublicText::with_max_chars`. Match
  `PublicTextError::TooLong { actual_chars, max_chars }` and
  `ParameterNameError::TooLong { actual_chars }` using the character fields.
- Treat `ProblemOccurrence::instance()` as a `UriReference`, and match
  `ProblemOccurrenceError::InvalidInstance(error)` as a tuple variant.
- Add a wildcard arm when matching public reporting, validation, wire, and
  `LayerBuildError` enums; they are now non-exhaustive.
- Do not require `Catalog`, `Problem`, `HealthFinding`, or
  `OperationDiagnostic` to implement `UnwindSafe` or `RefUnwindSafe`; runtime
  validators intentionally make those auto-traits unavailable.

### Added

- consuming access to an Axum failure's encoded Problem for terminal SSE frames;
- explicit diagnostic retirement with permanent tombstones and replacement chains;
- catalog-aware client conformance findings for type, status, and headers;
- tagged-release verification and GitHub release receipts;
- packaged HTTP and CLI smoke coverage driven by Smoque.

### Changed

- narrowed `recourse-axum` to Axum's `matched-path` capability instead of its
  default feature set;
- pinned public prerelease dependencies to the exact Recourse cohort;
- made evolving public reporting and implementation enums non-exhaustive;
- pointed the binary-only CLI documentation metadata at its README;
- added a latest-stable Rust lane alongside the Rust 1.96 MSRV gate;
- made source-compatibility CI execute its full patch lint set against the
  finalized RC.2 public API snapshot.

### Fixed

- HTTP status/header invariants and URI-reference identity preservation;
- runtime and schema bounds for governed wire values;
- Axum panic, readiness, clone, observer, and fault-reporter containment;
- per-request consumption of readiness failures across scope-preparation errors;
- atomic lock replacement and recoverable documentation publication;
- replacement-chain validation, cycle rejection, and documentation;
- recursive duplicate-member rejection and explicit wrong-type findings in
  received JSON;
- release checksums that attest the exact crates.io archives rather than local
  source candidates.

## [0.0.1-rc.1] - 2026-08-11

- First public preview of the framework-neutral core, Axum adapter, Cargo CLI,
  catalog lifecycle, and Dispatch reference system.

[0.0.1-rc.2]: https://github.com/zsumz/recourse/compare/v0.0.1-rc.1...HEAD
[0.0.1-rc.1]: https://github.com/zsumz/recourse/releases/tag/v0.0.1-rc.1
