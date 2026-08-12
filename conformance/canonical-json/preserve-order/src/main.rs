//! Preserve-order external canonical JSON consumer.

#[path = "../../fixture.rs"]
mod fixture;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fixture::run()
}
