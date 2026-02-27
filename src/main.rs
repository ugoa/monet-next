#![allow(clippy::all)]
#![allow(warnings)]
mod rt;

use bytes::Bytes;
use compio::net::{TcpListener, TcpStream};
use futures::{
    // StreamExt,
    future::{self, FutureExt},
    pin_mut,
    select,
    stream::FuturesUnordered,
};
use http_body_util::Full;
use hyper::{
    Method, Request, Response, StatusCode, body::Incoming, server::conn::http1,
    service::service_fn, upgrade::on,
};
use std::cell::RefCell;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::pin;

use crate::rt::{HyperStream, Listener};

async fn action(
    req: Request<Incoming>,
    cache: &RefCell<i32>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    compio::runtime::time::sleep(std::time::Duration::from_millis(5000)).await;

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

use futures::stream::{self, StreamExt};
use futures_concurrency::future::FutureGroup;
use futures_concurrency::prelude::*;
use futures_concurrency::prelude::*;

use async_channel::{self as channel, Receiver, Sender};

type Unit = Result<(), Box<dyn std::error::Error + Send + Sync>>;

enum Message {
    Incoming((TcpStream, SocketAddr)),
    Completed(Option<()>),
}

type MSG = ();

type Handle = Sender<MSG>;

struct Actor(Receiver<MSG>);

impl Actor {
    fn new() -> (Self, Handle) {
        let (sender, receiver) = channel::unbounded();
        (Self(receiver), sender)
    }

    async fn run(&mut self) -> Result<(), Infallible> {
        self.0
            .clone()
            .co()
            .try_for_each(|msg| async move {
                compio::runtime::time::sleep(std::time::Duration::from_millis(1000)).await;
                println!("background job finished successfully\n");
                Ok(())
            })
            .await
    }
}

fn main() {
    let rt = compio::runtime::Runtime::new().expect("cannot create runtime");

    rt.clone().block_on(async move {
        let port = 9527;
        println!("Running http server on 0.0.0.0:{}", port);
        let addr: SocketAddr = ([0, 0, 0, 0], port).into();
        let mut listener = compio::net::TcpListener::bind(addr).await.unwrap();

        let cache = RefCell::new(0);

        // let mut requests = FuturesUnordered::new();

        loop {
            let (io, _) = listener.accepts().await;
            unsafe {
                rt.clone()
                    .spawn_unchecked(async { handle_request(io, &cache).await });
            }
        }
    });

    // loop {
    //     if requests.is_empty() {
    //         let (io, _) = listener.accepts().await;
    //         requests.push(handle_request(io, &cache));
    //     } else {
    //         // let fut1 = pin!(async { listener.accepts().await });
    //         // let fut2 = pin!(async { group.borrow_mut().next().await });
    //
    //         futures::select! {
    //             conn = listener.accepts().fuse() => {
    //                 let (io, _) = conn;
    //                  requests.push(handle_request(io, &cache));
    //             },
    //             _ = requests.next() => {
    //                 // completed
    //             }
    //         }
    //     }
    // }

    // let mut group = RefCell::new(FutureGroup::new());
    // loop {
    //     if group.borrow().is_empty() {
    //         let (io, _) = listener.accepts().await;
    //         group.borrow_mut().insert(handle_request(io, &cache));
    //     } else {
    //         let fut1 = pin!(async { listener.accepts().await });
    //         let fut2 = pin!(async { group.borrow_mut().next().await });
    //
    //         let st1 = stream::once(fut1).map(Message::Incoming);
    //         let st2 = stream::once(fut2).map(Message::Completed);
    //
    //         let mut async_iter = (st1, st2).merge();
    //         while let Some(msg) = async_iter.next().await {
    //             match msg {
    //                 Message::Incoming((io, addr)) => {
    //                     group.borrow_mut().insert(handle_request(io, &cache));
    //                 }
    //                 _ => (),
    //             }
    //         }
    //     }
    // }

    // loop {
    //     let fut1 = pin!(async { listener.accepts().await });
    //     let fut2 = pin!(async { group.borrow_mut().next().await });
    //
    //     let st1 = stream::once(fut1).map(Message::Incoming);
    //     let st2 = stream::once(fut2).map(Message::Completed);
    //
    //     let mut async_iter = (st1, st2).merge();
    //     while let Some(msg) = async_iter.next().await {
    //         match msg {
    //             Message::Incoming((io, addr)) => {
    //                 group.borrow_mut().insert(handle_request(io, &cache));
    //             }
    //             _ => (),
    //         }
    //     }
    // }
}

async fn handle_request(stream: compio::net::TcpStream, cache: &RefCell<i32>) -> () {
    http1::Builder::new()
        .serve_connection(
            HyperStream::new(stream),
            service_fn(async |req| action(req, &cache).await),
        )
        .await
        .expect("Should handle request successfully");
    ()
}
