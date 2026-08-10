//! Runnable local server for the thin Dispatch Axum reference adapter.

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, dispatch_api_axum::router()?).await?;
    Ok(())
}
