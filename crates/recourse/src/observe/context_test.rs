//! Focused tests for bounded normalized request metadata.

use http::Method;

use super::{HttpEventContext, NormalizedRoute, NormalizedRouteError};

#[test]
fn context_carries_method_and_route_template() {
    let route = NormalizedRoute::new("/jobs/{job_id}");
    let Some(route) = route.ok() else {
        return;
    };
    let context = HttpEventContext::new()
        .with_method(Method::GET)
        .with_route(route);

    assert_eq!(context.method(), Some(&Method::GET));
    assert_eq!(
        context.route().map(NormalizedRoute::as_str),
        Some("/jobs/{job_id}")
    );
}

#[test]
fn unsafe_or_unnormalized_routes_are_rejected() {
    assert_eq!(
        NormalizedRoute::new("jobs/123"),
        Err(NormalizedRouteError::MissingRoot)
    );
    assert!(matches!(
        NormalizedRoute::new("/jobs\n"),
        Err(NormalizedRouteError::ControlCharacter { .. })
    ));
}
