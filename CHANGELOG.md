# Changelog

All notable changes to Recourse are documented here. The three public crates
ship as one versioned cohort.

## [0.0.1-rc.2] - Unreleased

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
- added a latest-stable Rust lane alongside the Rust 1.96 MSRV gate.

### Fixed

- HTTP status/header invariants and URI-reference identity preservation;
- runtime and schema bounds for governed wire values;
- Axum panic, readiness, clone, observer, and fault-reporter containment;
- atomic lock replacement and transactional documentation publication;
- replacement-chain validation, cycle rejection, and documentation;
- recursive duplicate-member rejection in received JSON.

## [0.0.1-rc.1] - 2026-08-11

- First public preview of the framework-neutral core, Axum adapter, Cargo CLI,
  catalog lifecycle, and Dispatch reference system.

[0.0.1-rc.2]: https://github.com/zsumz/recourse/compare/v0.0.1-rc.1...HEAD
[0.0.1-rc.1]: https://github.com/zsumz/recourse/releases/tag/v0.0.1-rc.1
