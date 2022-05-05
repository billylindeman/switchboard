use futures_util::{future, stream, Sink, SinkExt, Stream, StreamExt, TryStreamExt};
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;

use futures_channel::mpsc;

use tokio_tungstenite::{tungstenite, WebSocketStream};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Request {
    id: String,
    method: String,
    params: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Response {
    id: String,
    method: String,
    result: Option<HashMap<String, Value>>,
    error: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Notification {
    method: String,
    params: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
    Disconnected,
}

pub async fn handle_messages(
    stream: WebSocketStream<tokio::net::TcpStream>,
) -> mpsc::UnboundedReceiver<anyhow::Result<Message>> {
    let (write, read) = stream.split();

    let (mut tx, rx) = mpsc::unbounded::<anyhow::Result<Message>>();

    tokio::spawn(async move {
        read.map_err(|err| error!("websocket error: {}", err))
            .try_filter(|msg| future::ready(msg.is_text()))
            .map(|msg| msg.unwrap())
            .map(|msg| serde_json::from_str::<Message>(msg.to_text().unwrap()))
            .err_into()
            .map(Ok)
            .forward(tx)
            .await
            .expect("error in stream");

        error!("disconnected");
    });

    rx
}
