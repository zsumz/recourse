//! Explicit deterministic Dispatch catalog exporter.

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    dispatch_diagnostics::catalog()?
        .artifact()
        .write_pretty(std::io::stdout())?;
    Ok(())
}
