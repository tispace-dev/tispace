use std::{net::SocketAddr, time::Duration};

use axum::{error_handling::HandleErrorLayer, Router};
use kube::Client;
use tower::ServiceBuilder;
use tower_http::cors::{any, CorsLayer, Origin};
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};

use tispace::error::handle_error;
use tispace::operator_k8s::Operator as K8sOperator;
use tispace::operator_lxd::Operator as LxdOperator;
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
                .allow_origin(Origin::list([
                    "http://localhost:3000".parse().unwrap(),
                    "http://tispace.herokuapp.com".parse().unwrap(),
                    "https://tispace.dev".parse().unwrap(),
                ]))
                .allow_methods(any())
                .allow_headers(any()),
        );

    let client = Client::try_default().await.unwrap();
    let k8s_operator = K8sOperator::new(client, s.clone());
    tokio::spawn(async move { k8s_operator.run().await });
    let lxd_operator = LxdOperator::new(s);
    tokio::spawn(async move { lxd_operator.run().await });

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
