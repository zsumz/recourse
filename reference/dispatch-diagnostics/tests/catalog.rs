//! Public-API conformance test for the first Dispatch catalog snapshot.

use dispatch_diagnostics::catalog;

#[test]
fn artifact_matches_the_reviewed_snapshot() {
    let catalog = catalog();
    assert!(catalog.is_ok());
    let Some(catalog) = catalog.ok() else {
        return;
    };
    let mut output = Vec::new();
    assert!(catalog.artifact().write_pretty(&mut output).is_ok());

    assert_eq!(output, include_bytes!("../../../diagnostics/catalog.json"));
}

#[test]
fn initial_diagnostics_keep_their_permanent_statuses() {
    let catalog = catalog();
    let Some(catalog) = catalog.ok() else {
        return;
    };
    let identities = catalog
        .artifact()
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| {
            diagnostic
                .http_status()
                .map(|status| (diagnostic.code().to_string(), status))
        })
        .collect::<Vec<_>>();

    assert_eq!(
        identities,
        [
            ("DSP-1001".to_owned(), 400),
            ("DSP-1002".to_owned(), 422),
            ("DSP-1003".to_owned(), 404),
            ("DSP-1004".to_owned(), 409),
            ("DSP-1005".to_owned(), 401),
            ("DSP-1006".to_owned(), 405),
            ("DSP-1007".to_owned(), 503),
            ("DSP-1008".to_owned(), 500),
            ("DSP-1010".to_owned(), 503),
            ("DSP-1011".to_owned(), 415),
        ]
    );
}

#[test]
fn durable_and_health_surfaces_are_explicit() {
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let artifact = catalog.artifact();
    let operation = artifact
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code().to_string() == "DSP-1009");
    let health = artifact
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code().to_string() == "DSP-1010");

    assert!(matches!(
        operation,
        Some(diagnostic) if diagnostic.impact_schema().is_some()
    ));
    assert!(matches!(
        health,
        Some(diagnostic) if diagnostic.supports_health()
    ));
}

#[test]
fn api_operations_publish_their_declared_problem_sets() {
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let artifact = catalog.artifact();
    let create_job = artifact.problem_sets().get("createJob");
    let get_job = artifact.problem_sets().get("getJob");

    assert_eq!(create_job.map(Vec::len), Some(8));
    assert_eq!(get_job.map(Vec::len), Some(4));
}
