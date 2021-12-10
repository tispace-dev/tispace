use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::Server;

use crate::operator::Operator;
use error::*;
use service::HttpService;
use storage::Storage;

mod error;
mod model;
mod operator;
mod service;
mod storage;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:8080".parse().unwrap();

    let s = Storage::from_path("state.json").await?;
    let http_srv = Arc::new(HttpService::from_storage(s.clone()).await?);
    let service = make_service_fn(move |_| {
        let http_srv: Arc<HttpService> = http_srv.clone();
        async move {
            Ok::<_, GenericError>(service_fn(move |req| {
                let http_srv: Arc<HttpService> = http_srv.clone();
                async move { http_srv.serve_http(req).await }
            }))
        }
    });
    let operator = Operator::new(s);
    tokio::spawn(async move { operator.run().await });

    let server = Server::bind(&addr).serve(service);
    println!("Listening on {}", addr);
    server.await?;
    Ok(())
}
