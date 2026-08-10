//! Fuzzes deterministic compatibility analysis against accepted Dispatch history.
#![no_main]

use libfuzzer_sys::fuzz_target;
use recourse::catalog::{CatalogArtifact, CatalogLock};

const LOCK: &[u8] = include_bytes!("../../diagnostics/catalog.lock");

fuzz_target!(|body: &[u8]| {
    let Ok(current) = CatalogArtifact::from_slice(body) else {
        return;
    };
    let Ok(lock) = CatalogLock::from_slice(LOCK) else {
        panic!("reviewed Dispatch lock must remain valid");
    };
    assert_eq!(lock.check(&current), lock.check(&current));
});
