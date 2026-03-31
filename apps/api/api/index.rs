use axum::Router;
use gigeo_api::build_app;
use tower::ServiceBuilder;

use vercel_runtime::{Error, axum::VercelLayer};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Build your Axum app (side effects like env/config/clients happen here)
    let router: Router = build_app().map_err(|e| Error::from(e.to_string()))?;
    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router);

    vercel_runtime::run(app).await
}
