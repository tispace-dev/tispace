use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use error::*;
use operator::Operator;
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

    let s = Storage::load("state.json").await?;
    let mut secret = String::new();
    s.read_write(|state| {
        if state.secret.is_empty() {
            secret = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();
            state.secret = secret.clone();
            true
        } else {
            false
        }
    })
    .await?;

    let http_svc = Arc::new(HttpService::new(s.clone(), secret));
    let service = make_service_fn(move |_| {
        let http_srv: Arc<HttpService> = http_svc.clone();
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
