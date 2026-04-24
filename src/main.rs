#![allow(clippy::all)]
#![allow(warnings)]
mod rt;

use crate::rt::{HyperStream, Listener};
use bytes::Bytes;
use compio::net::{TcpListener, TcpStream};
use compio::runtime::time::sleep;
use futures::stream::{self, StreamExt};
use futures::{
    // StreamExt,
    future::FutureExt,
    select,
    stream::FuturesUnordered,
};
use futures_concurrency::future::FutureGroup;
use futures_concurrency::prelude::*;
use http_body_util::Full;
use hyper::{
    Method, Request, Response, StatusCode, body::Incoming, server::conn::http1, service::service_fn,
};
use std::cell::RefCell;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::pin;
use std::time::Duration;

type Unit = Result<(), Box<dyn std::error::Error + Send + Sync>>;

enum Message {
    Incoming((TcpStream, SocketAddr)),
    Completed(Option<()>),
}

#[compio::main]
async fn main() {
    let port = 9527;
    println!("Running http server on 0.0.0.0:{}", port);
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let mut listener = compio::net::TcpListener::bind(addr).await.unwrap();

    let cache = RefCell::new(0);

    let mut group = FutureGroup::new();
    loop {
        tokio::select! {
            biased;
            stream = listener.accepts() => {
                println!("Received at {}", jiff::Timestamp::now());
                group.insert(handle_request(stream.0, &cache));
            },
            res =  group.next(), if !group.is_empty()  => (),
        }
    }
}

async fn handle_request(stream: compio::net::TcpStream, cache: &RefCell<i32>) {
    http1::Builder::new()
        .serve_connection(
            HyperStream::new(stream),
            service_fn(async |req| action(req, &cache).await),
        )
        .await
        .expect("Should handle request successfully");
}

async fn action(
    req: Request<Incoming>,
    cache: &RefCell<i32>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            compio::runtime::time::sleep(std::time::Duration::from_millis(2000)).await;
            *cache.borrow_mut() += 1;

            use jiff::Zoned;

            Ok(Response::new(Full::new(Bytes::from(format!(
                "Visit Count: {} at {} \n",
                *cache.borrow(),
                Zoned::now()
            )))))
        }
        (&Method::GET, "/compio") => Ok(Response::new(Full::new(Bytes::from("Hello Compio!")))),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("404 not found")))
            .unwrap()),
    }
}
