//! Fuzzes bounded catalog parsing, validation, and canonical re-encoding.
#![no_main]

use libfuzzer_sys::fuzz_target;
use recourse::catalog::CatalogArtifact;

fuzz_target!(|body: &[u8]| {
    let first = CatalogArtifact::from_slice(body);
    let second = CatalogArtifact::from_slice(body);
    match (first, second) {
        (Ok(first), Ok(second)) => {
            assert_eq!(first, second);
            let mut encoded = Vec::new();
            if let Err(error) = first.write_pretty(&mut encoded) {
                panic!("validated artifact failed to encode: {error}");
            }
            assert_eq!(CatalogArtifact::from_slice(&encoded).ok(), Some(first));
        }
        (Err(first), Err(second)) => assert_eq!(first.to_string(), second.to_string()),
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => panic!("identical input parsed inconsistently"),
    }
});
