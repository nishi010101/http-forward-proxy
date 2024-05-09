mod handler;
mod state;
mod util;

use crate::state::AppState;
use hyper::http;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;

use crate::handler::proxy_requests;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    loop {
        let (stream, addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let service = service_fn(move |req| proxy_requests(req, AppState::new(), addr));

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
