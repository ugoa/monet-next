mod rt;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Response, StatusCode, body::Incoming};
use hyper::{server::conn::http1, service::service_fn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;

use crate::rt::HyperStream;

async fn action(
    req: Request<Incoming>,
    cache: &RefCell<HashMap<&'static str, f32>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(Full::new(Bytes::from("Hello World!")))),
        (&Method::GET, "/compio") => Ok(Response::new(Full::new(Bytes::from("Hello Compio!")))),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("404 not found")))
            .unwrap()),
    }
}

fn main() {
    let port = 9527;
    println!("Running http server on 0.0.0.0:{}", port);
    let body = async {
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        let listener = compio::net::TcpListener::bind(addr).await.unwrap();

        let cache = RefCell::new(HashMap::from([
            ("Mercury", 0.4),
            ("Venus", 0.7),
            ("Earth", 1.0),
            ("Mars", 1.5),
        ]));

        loop {
            let (io, _) = listener.accept().await.unwrap();

            let io = HyperStream::new(io);

            compio::runtime::spawn(async {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(async |req| action(req, &cache).await))
                    .await
                {
                    println!("Error serving connection: {:?}", err);
                }
            })
            .detach();
        }
    };
    compio::runtime::Runtime::new().unwrap().block_on(body);
}
