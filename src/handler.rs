use crate::util::{empty, full, host_addr};
use crate::AppState;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use http_body_util::combinators::BoxBody;
use hyper::header::HeaderValue;
use hyper::upgrade::Upgraded;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use reqwest::Url;
use serde_json::Value;
use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::net::TcpStream;
use uuid::Uuid;

pub async fn proxy_requests(
    req: Request<hyper::body::Incoming>,
    state: AppState,
    client_addr: SocketAddr,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let correlation_id = Uuid::new_v4();
    let dt: DateTime<Utc> = SystemTime::now().into();
    let target_host = &(req.headers().clone())["host"]; //removing & messes up, why
    let target_uri = req.uri().to_string();
    println!("Client: {client_addr} ; Request URL: {target_uri:?}; timestamp: {dt:?}; correlationId: {correlation_id}");

    if !is_valid_host(&state, target_host) {
        return error(
            format!("Forbidden host {target_host:?}"),
            StatusCode::FORBIDDEN,
            correlation_id,
        );
    }
    // target is https. tunnel is needed
    if Method::CONNECT == req.method() {
        if let Some(addr) = host_addr(req.uri()) {
            tokio::task::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) = tunnel(upgraded, addr).await {
                            eprintln!("server io error: {}", e);
                        };
                    }
                    Err(e) => eprintln!("upgrade error: {}", e),
                }
            });
            println!("Code: 200 Ok; correlationId: {correlation_id}");
            println!("-------------------------------");
            Ok(Response::new(empty()))
        } else {
            error(
                "CONNECT must be to a socket address".to_string(),
                StatusCode::BAD_REQUEST,
                correlation_id,
            )
        }
    } else {
        // target is https. No tunnel needed
        if let Ok(url) = Url::parse(target_uri.as_str()) {
            let mut json_data: Value = Value::Null;
            let mut headers = req.headers().clone();
            headers.remove("target");
            headers.remove("host");
            headers.insert(
                "X-Forwarded-For",
                client_addr.ip().to_string().parse().unwrap(),
            );
            headers.insert("X-Forwarded-Host", target_host.clone());

            match state
                .http_client
                .request(req.method().clone(), url.clone())
                .headers(headers.clone())
                .send()
                .await
            {
                Ok(val) => match val.status() {
                    StatusCode::OK => match val.json().await {
                        Ok(json_val) => {
                            if !is_response_valid(state.banned_words, &json_val) {
                                return error(
                                    format!("Content from {target_uri} not allowed"),
                                    StatusCode::BAD_REQUEST,
                                    correlation_id,
                                );
                            }
                            json_data = json_val;
                        }
                        Err(err) => {
                            return error(err.to_string(), StatusCode::BAD_REQUEST, correlation_id);
                        }
                    },
                    code => {
                        return error(
                            format!("Error! Got response code {code} from {target_uri}"),
                            code,
                            correlation_id,
                        );
                    }
                },
                Err(err) => {
                    return error(
                        err.to_string(),
                        StatusCode::INTERNAL_SERVER_ERROR,
                        correlation_id,
                    );
                }
            };

            println!("Code: 200 Ok; correlationId: {correlation_id}");
            println!("-------------------------------");

            // Create Hyper response with JSON body
            let json = serde_json::to_string(&json_data).unwrap();
            let response = Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(full(json))
                .unwrap();
            Ok(response)
        } else {
            error("".to_string(), StatusCode::BAD_REQUEST, correlation_id)
        }
    }
}

fn error(
    err: String,
    code: StatusCode,
    correlation_id: Uuid,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    println!("Code: {code} ; correlationId: {correlation_id}");
    println!("-------------------------------");
    let mut resp = Response::new(full(err));
    *resp.status_mut() = code;
    Ok(resp)
}

fn is_valid_host(state: &AppState, target_host: &HeaderValue) -> bool {
    for fh in &state.forbidden_hosts {
        if let Ok(target_host_str) = target_host.to_str() {
            let parts: Vec<_> = target_host_str.split(':').collect();
            if fh == parts[0] || fh.contains(parts[0]) {
                return false;
            }
        }
    }
    true
}

fn is_response_valid(banned_words: Vec<String>, res: &Value) -> bool {
    for bw in banned_words {
        if serde_json::to_string(res).unwrap().contains(&bw.clone()) {
            return false;
        }
    }
    true
}

async fn tunnel(upgraded: Upgraded, addr: String) -> std::io::Result<()> {
    // Connect to remote server
    let mut server = TcpStream::connect(addr).await?;
    let mut upgraded = TokioIo::new(upgraded);
    // Proxying data
    let (from_client, from_server) =
        tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;
    // Print message when done
    println!(
        "client wrote {} bytes and received {} bytes",
        from_client, from_server
    );

    Ok(())
}
