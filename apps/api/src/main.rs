// src/main.rs
use anyhow::Context;

use gigeo_api::build_app; // crate name `api` assumes package name in Cargo.toml

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = build_app().context("Failed to build Axum app")?;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .context("Failed to bind to port 3000")?;

    tracing::info!("HTTP server listening on 0.0.0.0:3000");
    axum::serve(listener, app)
        .await
        .context("HTTP server error")?;

    Ok(())
}
