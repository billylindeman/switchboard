use futures_util::{SinkExt, StreamExt, TryStreamExt};
use log::*;
use tokio::net::{TcpListener, TcpStream};

use super::*;
use crate::sfu;

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

    let mut peer = sfu::peer::Peer::new().await.expect("error creating peer");

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (rpc_rx, rpc_tx) = jsonrpc::handle_messages(ws_stream).await;
    let (sig_rx, mut sig_tx) = signal::handle_messages(rpc_rx, rpc_tx).await;

    peer.event_loop(sig_rx, sig_tx.clone()).await;

    error!("event loop closed");
    sig_tx.close().await.expect("closed signal tx");
    peer.close().await;

    drop(peer);

    info!("client disconnected");
}
