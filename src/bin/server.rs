use std::{net::SocketAddr, time::Duration};

use axum::{error_handling::HandleErrorLayer, routing::get, Router};
use kube::Client;
use tower::ServiceBuilder;
use tower_http::cors::{any, CorsLayer, Origin};
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

use tispace::auth::authorized;
use tispace::error::handle_error;
use tispace::operator::Operator;
use tispace::service::protected_routes;
use tispace::storage::Storage;

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "tispace=debug,tower_http=debug")
    }
    tracing_subscriber::fmt::init();

    let s: Storage = Storage::load("state.json").await.unwrap();

    let app = Router::new()
        .route("/authorized", get(authorized))
        .merge(protected_routes())
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                // Handle errors from middleware
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(1024)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(AddExtensionLayer::new(s.clone()))
                .into_inner(),
        )
        .layer(
            // see https://docs.rs/tower-http/latest/tower_http/cors/index.html
            // for more details
            CorsLayer::new()
                .allow_origin(Origin::exact("http://localhost:3000".parse().unwrap()))
                .allow_methods(any())
                .allow_headers(any()),
        );

    let client = Client::try_default().await.unwrap();
    let operator = Operator::new(client, s);
    tokio::spawn(async move { operator.run().await });

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
