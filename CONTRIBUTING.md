# Contributing

Recourse welcomes focused bug reports, design discussions, documentation
improvements, and pull requests.

## Before changing code

- Open an issue first for new protocol surface or compatibility semantics.
- Keep the framework-neutral contract in `recourse`; framework behavior belongs
  in a leaf adapter such as `recourse-axum`.
- Preserve the structural boundary between public evidence and private reports.
- Add behavioral, architecture, or boundary tests for the contract being
  changed.

The repository declares Rust 1.96 as its MSRV. The packaged smoke gate also
requires Node.js 22.18 or newer and invokes the pinned Smoque release through
`npx`.

## Verify a change

Run the canonical gate from the repository root:

```sh
scripts/check
```

It formats, lints, tests, builds documentation, checks deterministic artifacts,
packages every public crate, tests the extracted archives, and drives a real
packaged HTTP/CLI consumer with Smoque. The gate does not publish or push.

For a quick inner loop, run the narrow test for the behavior being changed, then
finish with `scripts/check` before requesting review.

## Pull requests

Keep commits small and explain the contract or failure mode the change proves.
Avoid unrelated cleanup. Update the changelog when user-visible behavior,
compatibility rules, MSRV, or public APIs change. By contributing, you agree
that your contribution is licensed under the repository's MIT license.
