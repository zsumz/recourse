# recourse-axum

`recourse-axum` is the Axum/Tower adapter for Recourse's framework-neutral,
versioned failure contracts.

```toml
[dependencies]
axum = "0.8"
recourse = "=0.0.1-rc.3"
recourse-axum = "=0.0.1-rc.3"
```

The layer owns request correlation, request-scoped `ProblemContext`, panic and
service-error fallback, response translation, and observation timing. Catalog
identity, status selection, required headers, JSON encoding, compatibility, and
private/public separation remain in `recourse`.

The panic boundary covers request scoping, Tower readiness and synchronous
`call`, and the returned service future while the response can still be
replaced. Observer and reporter panics are isolated per invocation so telemetry
cannot suppress a caller response. These callbacks still execute synchronously:
keep them fast, nonblocking, nonpanicking, and internally bounded, preferably by
using a nonblocking send into an application-owned bounded channel.

Applications build the layer from an explicit catalog, an explicitly registered
internal diagnostic, and an explicit fault-reporting choice: either a
`FaultReporter` or the deliberate `discard_faults()` opt-out. Handlers return
the concrete `HandlerResult<T>` and construct expected Problems or unexpected
private faults through the request-scoped context.

Given application-owned `ServiceCatalog`, `ResourceMissing`, and
`InternalError` declarations, the complete Axum boundary is:

```rust
use axum::{Router, routing::get};
use recourse::{
    catalog::Catalog,
    diagnostic::NoEvidence,
};
use recourse_axum::{HandlerResult, ProblemContext, RecourseLayer};

async fn missing(
    problems: ProblemContext<ServiceCatalog>,
) -> HandlerResult<&'static str> {
    Err(problems.problem::<ResourceMissing>(NoEvidence))
}

fn router() -> Result<Router, Box<dyn std::error::Error>> {
    let catalog = Catalog::<ServiceCatalog>::builder()
        .problem::<ResourceMissing>()
        .problem::<InternalError>()
        .build()?;
    let layer = RecourseLayer::builder(catalog)
        .internal::<InternalError>()
        .discard_faults()
        .build()?;

    Ok(Router::new()
        .route("/resources/{id}", get(missing))
        .layer(layer))
}
```

`discard_faults()` is an explicit opt-out for applications that already record
private failures elsewhere. Otherwise provide an application-owned
`FaultReporter`; layer construction fails closed if neither choice is made.

Recourse core produces the canonical status, headers, and JSON bytes.
`recourse-axum` owns request context and lifecycle translation, then places
those bytes into an Axum response without redefining the protocol.

See the [repository](https://github.com/zsumz/recourse) for the Dispatch Axum
reference API and executable lifecycle tests.
