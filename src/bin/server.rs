use std::{net::SocketAddr, time::Duration};

use axum::{error_handling::HandleErrorLayer, Router};
use reqwest::{Client as ReqwestClient, Identity};
use std::fs::File;
use std::io::Read;
use tower::ServiceBuilder;
use tower_http::cors::{any, CorsLayer, Origin};
use tower_http::{add_extension::AddExtensionLayer, trace::TraceLayer};
use tracing::{info, warn};

use tispace::collector::Collector;
use tispace::env::LXD_CLIENT_CERT;
use tispace::error::handle_error;
use tispace::operator_lxd::Operator as LxdOperator;
use tispace::scheduler::Scheduler;
use tispace::service::{metrics_routes, protected_routes};
use tispace::storage::Storage;

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "tispace=debug,tower_http=debug,server=debug")
    }
    tracing_subscriber::fmt::init();

    let s: Storage = Storage::open("state.json").await.unwrap();

    let mut lxd_client = None;
    if !LXD_CLIENT_CERT.is_empty() {
        let mut buf = Vec::new();
        File::open(LXD_CLIENT_CERT.as_str())
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
        let id = Identity::from_pkcs12_der(&buf, "").unwrap();
        let client = ReqwestClient::builder()
            .danger_accept_invalid_certs(true)
            .identity(id)
            .build()
            .unwrap();
        let lxd_operator = LxdOperator::new(client.clone(), s.clone());
        tokio::spawn(async move { lxd_operator.run().await });
        lxd_client = Some(client);
        info!("lxd operator started");
    } else {
        warn!("lxd client cert not provided, will not start lxd operator");
    }

    let collector = Collector::new(s.clone(), None, lxd_client);
    tokio::spawn(async move { collector.run().await });
    info!("collector started");

    let scheduler = Scheduler::new(s.clone());
    tokio::spawn(async move { scheduler.run().await });
    info!("scheduler started");

    let app = Router::new()
        .merge(protected_routes())
        .merge(metrics_routes())
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                // Handle errors from middleware
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(1024)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(AddExtensionLayer::new(s))
                .into_inner(),
        )
        .layer(
            // see https://docs.rs/tower-http/latest/tower_http/cors/index.html
            // for more details
            CorsLayer::new()
                .allow_origin(Origin::list([
                    "http://localhost:3000".parse().unwrap(),
                    "https://tispace.dev".parse().unwrap(),
                ]))
                .allow_methods(any())
                .allow_headers(any()),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
