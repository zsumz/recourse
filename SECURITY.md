# Security policy

## Supported versions

Security fixes are made on the latest published Recourse release line. Release
candidates are previews: upgrade to the newest RC before reporting behavior
that may already have been corrected. See [SUPPORT.md](./SUPPORT.md) for the
version and Rust support policy.

## Reporting a vulnerability

Please report suspected vulnerabilities through a
[private GitHub security advisory](https://github.com/zsumz/recourse/security/advisories/new).
Do not open a public issue for a vulnerability or include private application
data, credentials, or production failure payloads in a report.

Include the affected Recourse crate and version, the security impact, a minimal
reproduction when safe, and any known mitigations. You can expect an
acknowledgement after the report has been reviewed; remediation timing depends
on severity and the scope of the fix.

Recourse's public/private type boundary reduces accidental disclosure, but it
does not make caller-supplied evidence safe automatically. Applications remain
responsible for deciding which values are appropriate to expose publicly.
