use async_tungstenite::tokio::TokioAdapter;
use async_tungstenite::{tokio::connect_async, WebSocketStream};
use async_tungstenite::stream::Stream;
use async_tungstenite::tungstenite::protocol::Message;
use anyhow::{Result, Context};
use futures::stream::SplitSink;
use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_util::sync::{CancellationToken, DropGuard};
use tokio_rustls::client::TlsStream;

use switchboard_proto as proto;

pub trait Signal {

}


type WriteStream = SplitSink<WebSocketStream<Stream<TokioAdapter<TcpStream>, TokioAdapter<TlsStream<TcpStream>>>>, Message>;

pub struct JsonRPCWebsocket {
    guard: DropGuard,
    writer: WriteStream,
}
impl Signal for JsonRPCWebsocket;

impl JsonRPCWebsocket {
    pub async fn connect(url: String) -> Result<JsonRPCWebsocket> {
        let (ws_stream, _) = connect_async(&url).await.context("Failed to connect")?;
        println!("WebSocket handshake has been successfully completed");

        let token = CancellationToken::new();
        let cancel = token.clone();
        let (write, mut read) = ws_stream.split();


        tokio::spawn(async move {
            tokio::select! {
                next = read.next() => {
                    if let Some(Ok(msg)) = next {
                        println!("msg {}", msg);
                    }
                }

                _ = cancel.cancelled() => {
                    println!("token dropped, event loop exiting");
                }

            }
        });

        Ok(Self {
            guard: token.drop_guard(),
            writer: write,
        })
    }
}
