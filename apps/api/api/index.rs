use axum::Router;
use gigeo_api::build_app;
use serde_json::Value;
use vercel_runtime::{Error, Request, run, service_fn};

async fn handler(_req: Request) -> Result<Value, Error> {
    // Build your Axum app (side effects like env/config/clients happen here)
    let app: Router = build_app().map_err(|e| Error::from(e.to_string()))?;

    // For now, just return a simple JSON response proving the function works.
    // Later you can move specific logic into separate, smaller handlers.
    Ok(serde_json::json!({
        "status": "ok",
        "routes_mounted": true
    }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}
