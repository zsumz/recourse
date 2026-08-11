# recourse

`recourse` is the framework-neutral core of the Recourse governed diagnostics
protocol.

```toml
[dependencies]
recourse = "0.0.1"
```

It provides:

- permanent catalog codes and type URIs;
- typed, schema-governed public evidence;
- strict RFC 9457 Problem construction and HTTP policy headers;
- structurally separate public Problems and private reports;
- durable operation diagnostics and health findings;
- bounded tolerant client decoding;
- append-only catalog locks and conservative compatibility classification.

Applications register declarations explicitly with `Catalog::builder()`. The
core crate has no async runtime or application-framework dependency. Framework
adapters consume its concrete response parts at the edge.

This minimal program declares one permanent diagnostic and encodes its
canonical HTTP Problem without a server framework:

```rust
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    http::{CorrelationId, Fixed, HttpProblemType, ProblemOccurrence},
};

enum ServiceCatalog {}

impl CatalogSpec for ServiceCatalog {
    const NAME: &'static str = "example-service";
    const PREFIX: &'static str = "EXM";
    const TYPE_BASE: &'static str = "https://example.invalid/problems/";
}

enum ResourceMissing {}

impl DiagnosticType for ResourceMissing {
    type Catalog = ServiceCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1001);
    const TITLE: &'static str = "Resource missing";
    const DETAIL: &'static str = "The requested resource does not exist.";
    const SUGGESTIONS: &'static [&'static str] = &["Check the resource identifier."];
    const DOCS: &'static str = "Verify the identifier before retrying.";
}

impl HttpProblemType for ResourceMissing {
    type Policy = Fixed<404>;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = Catalog::<ServiceCatalog>::builder()
        .problem::<ResourceMissing>()
        .build()?;
    let occurrence = ProblemOccurrence::new(
        CorrelationId::new("request-01")?,
        "/problem-occurrences/request-01",
    )?;
    let encoded = catalog
        .try_problem::<ResourceMissing>(occurrence, NoEvidence)?
        .try_encode()?;

    assert_eq!(encoded.status().as_u16(), 404);
    assert_eq!(encoded.headers()["content-type"], "application/problem+json");
    Ok(())
}
```

The catalog owns the status, headers, JSON shape, code, and type URI. Public
evidence must explicitly implement `PublicEvidence`; private source errors use
the structurally separate `PrivateReport` type.

See the [repository](https://github.com/zsumz/recourse) for the protocol design,
Dispatch reference implementation, conformance fixtures, and generated catalog
documentation. The repository also records the reviewed performance and
allocation boundary.
