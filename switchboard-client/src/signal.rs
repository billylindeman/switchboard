use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use futures::StreamExt;
use switchboard_proto as proto;
use tokio_util::sync::{CancellationToken, DropGuard};

pub struct JsonRPCWebsocket {
    guard: DropGuard,
}

impl JsonRPCWebsocket {
    pub async fn connect(url: String) -> JsonRPCWebsocket {
        let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
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

        Self {
            guard: token.drop_guard(),
        }
    }
}
