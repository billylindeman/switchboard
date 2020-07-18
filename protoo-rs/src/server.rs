use async_std::net::{SocketAddr, TcpListener, TcpStream};
use async_tungstenite::{accept_async, tungstenite::Error};
use futures::prelude::*;
use tungstenite::Result;

use log::*;

pub struct WebsocketServer {
    pub addr: &'static str 
}

impl WebsocketServer {
    async fn accept_connection(peer: SocketAddr, stream: TcpStream) {
        if let Err(e) = WebsocketServer::handle_connection(peer, stream).await {
            match e {
                Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
                err => error!("Error processing connection: {}", err),
            }
        }
    }
    
    async fn handle_connection(peer: SocketAddr, stream: TcpStream) -> Result<()> {
        let mut ws_stream = accept_async(stream).await.expect("Failed to accept");
    
        info!("New WebSocket connection: {}", peer);
    
        while let Some(msg) = ws_stream.next().await {
            let msg = msg?;
            if msg.is_text() || msg.is_binary() {
                ws_stream.send(msg).await?;
            }
        }
    
        Ok(())
    }
    
    async fn run_async(&self) {
        let listener = TcpListener::bind(&self.addr).await.expect("Can't listen");
        info!("Listening on: {}", &self.addr);
    
        while let Ok((stream, _)) = listener.accept().await {
            let peer = stream
                .peer_addr()
                .expect("connected streams should have a peer address");
            info!("Peer address: {}", peer);
    
            async_std::task::spawn(WebsocketServer::accept_connection(peer, stream));
        }
    }

    pub fn run(&self) {
        async_std::task::block_on(self.run_async());
    }
}


