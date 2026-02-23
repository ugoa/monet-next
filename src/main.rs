#![allow(clippy::all)]
#![allow(warnings)]

mod rt;

use bytes::Bytes;
use futures::future::select;
use futures::stream::FuturesUnordered;
use http_body_util::Full;
use hyper::{Method, Request, Response, StatusCode, body::Incoming};
use hyper::{server::conn::http1, service::service_fn};
use std::cell::RefCell;
use std::convert::Infallible;
use std::net::{SocketAddr, TcpStream};

use crate::rt::{HyperStream, Listener};

async fn action(
    req: Request<Incoming>,
    cache: &RefCell<i32>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *cache.borrow_mut() += 1;
            Ok(Response::new(Full::new(Bytes::from(format!(
                "Visit Count: {}\n",
                *cache.borrow()
            )))))
        }
        (&Method::GET, "/compio") => Ok(Response::new(Full::new(Bytes::from("Hello Compio!")))),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("404 not found")))
            .unwrap()),
    }
}

use futures::{StreamExt, future::FutureExt, select};

#[compio::main]
async fn main() {
    let port = 9527;
    println!("Running http server on 0.0.0.0:{}", port);
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let mut listener = compio::net::TcpListener::bind(addr).await.unwrap();

    let cache = RefCell::new(0);

    let mut requests = FuturesUnordered::new();

    loop {
        if requests.is_empty() {
            let (io, _) = listener.accepts().await;
            requests.push(handle_request(io, &cache));
        } else {
            select! {
                conn = listener.accepts().fuse() => {
                    let (io, _) = conn;
                    requests.push(handle_request(io, &cache));
                },
                _ = requests.next() => {
                    // one more request finished.
                }
            }
        }
    }
}

async fn handle_request(
    stream: compio::net::TcpStream,
    cache: &RefCell<i32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let io = HyperStream::new(stream);
    if let Err(err) = http1::Builder::new()
        .serve_connection(io, service_fn(async |req| action(req, &cache).await))
        .await
    {
        println!("Error serving connection: {:?}", err);
    }
    Ok(())
}
