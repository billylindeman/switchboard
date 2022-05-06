use std::net::SocketAddr;

use futures_util::{future, Sink, SinkExt, StreamExt, TryStreamExt};
use log::info;
use tokio::net::{TcpListener, TcpStream};

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

    while let Some(Ok(evt)) = rx.next().await {
        info!("got jsonrpc evt: {:#?}", evt);

        match evt {
            jsonrpc::Message::Request(r) => {
                let r = jsonrpc::Response {
                    method: r.method,
                    id: r.id,
                    result: Some(r.params),
                    error: None,
                };

                tx.send(Ok(jsonrpc::Message::Response(r))).await;
            }
            _ => {}
        }
    }

    info!("client disconnected");
}
