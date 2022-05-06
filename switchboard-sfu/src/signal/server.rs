use futures_util::{future, Sink, SinkExt, StreamExt, TryStreamExt};
use log::*;
use std::collections::HashMap;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use super::*;

pub async fn run_server(addr: &str) {
    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }
}

async fn accept_connection(stream: TcpStream) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (mut rx, mut tx) = jsonrpc::handle_messages(ws_stream).await;

    while let Some(e) = rx.next().await {
        match e {
            Ok(evt) => {
                info!("got jsonrpc evt: {:#?}", evt);

                match evt {
                    jsonrpc::Event::Request(r) => {
                        let r = jsonrpc::Response {
                            method: r.method,
                            id: r.id,
                            result: Some(r.params),
                            error: None,
                        };

                        match r.method.as_str() {
                            "close" => {
                                tx.close().await.expect("failed to close socket");
                            }
                            _ => {
                                tx.send(Ok(jsonrpc::Event::Response(r)))
                                    .await
                                    .expect("couldn't send message");
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(err) => {
                error!("got error: {}", err);
            }
        }
    }

    info!("client disconnected");
}
