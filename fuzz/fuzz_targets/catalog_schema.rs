//! Fuzzes the catalog parser's normalized evidence-schema profile.
#![no_main]

use libfuzzer_sys::fuzz_target;
use recourse::catalog::CatalogArtifact;
use serde_json::{Value, json};

fuzz_target!(|body: &[u8]| {
    let Ok(schema) = serde_json::from_slice::<Value>(body) else {
        return;
    };
    let artifact = json!({
        "schema_version": 1,
        "catalog": {
            "name": "fuzz",
            "prefix": "FUZ",
            "type_base": "https://fuzz.invalid/problems/"
        },
        "diagnostics": [{
            "number": 1,
            "code": "FUZ-1",
            "type": "https://fuzz.invalid/problems/FUZ-1",
            "title": "Fuzz fixture",
            "detail": "Schema fuzz fixture.",
            "suggestions": [],
            "documentation_markdown": "Schema fuzz fixture.",
            "evidence_schema": schema,
            "surfaces": {"http": {
                "status": 400,
                "policy": "fixed",
                "required_headers": []
            }}
        }],
        "problem_sets": {}
    });
    let encoded = serde_json::to_vec(&artifact).unwrap_or_default();
    let first = CatalogArtifact::from_slice(&encoded);
    let second = CatalogArtifact::from_slice(&encoded);
    assert_eq!(first.ok(), second.ok());
});
