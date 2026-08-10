//! Fuzzes bounded catalog-lock parsing and canonical re-encoding.
#![no_main]

use libfuzzer_sys::fuzz_target;
use recourse::catalog::CatalogLock;

fuzz_target!(|body: &[u8]| {
    let first = CatalogLock::from_slice(body);
    let second = CatalogLock::from_slice(body);
    match (first, second) {
        (Ok(first), Ok(second)) => {
            assert_eq!(first, second);
            let mut encoded = Vec::new();
            if let Err(error) = first.write_pretty(&mut encoded) {
                panic!("validated lock failed to encode: {error}");
            }
            assert_eq!(CatalogLock::from_slice(&encoded).ok(), Some(first));
        }
        (Err(first), Err(second)) => assert_eq!(first.to_string(), second.to_string()),
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => panic!("identical input parsed inconsistently"),
    }
});
