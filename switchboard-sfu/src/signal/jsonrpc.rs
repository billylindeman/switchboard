use futures::select;
use futures_util::{future, StreamExt, TryStreamExt};
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;

use futures_channel::mpsc;

use tokio_tungstenite::{tungstenite, WebSocketStream};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Request {
    pub id: String,
    pub method: String,
    pub params: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Response {
    pub id: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Notification {
    method: String,
    params: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Event {
    Request(Request),
    Response(Response),
    Notification(Notification),
    Disconnected,
}

pub type ReadStream = mpsc::UnboundedReceiver<anyhow::Result<Event>>;
pub type WriteStream = mpsc::UnboundedSender<anyhow::Result<Event>>;

pub async fn handle_messages(
    stream: WebSocketStream<tokio::net::TcpStream>,
) -> (ReadStream, WriteStream) {
    let (write, read) = stream.split();

    let (read_tx, read_rx) = mpsc::unbounded::<anyhow::Result<Event>>();
    let (write_tx, write_rx) = mpsc::unbounded::<anyhow::Result<Event>>();

    // Inbound message loop
    tokio::spawn(async move {
        let mut incoming_fut = read
            .map_err(|err| error!("websocket error: {}", err))
            .try_filter(|msg| future::ready(msg.is_text()))
            .map(|msg| msg.unwrap())
            .map(|msg| serde_json::from_str::<Event>(msg.to_text().unwrap()))
            .map_err(|err| {
                error!("error parsing json: {}", err);
                err.into()
            })
            .filter(|r| future::ready(r.is_ok()))
            .map(Ok)
            .forward(read_tx);

        let mut outgoing_fut = write_rx
            .map_ok(|evt| serde_json::to_string(&evt).unwrap())
            .map_ok(|msg| tungstenite::Message::from(msg))
            .map_err(|_| tungstenite::error::Error::ConnectionClosed)
            .forward(write);

        select! {
            _ = incoming_fut => info!("websocket closed"),
            _ = outgoing_fut => info!("client dropped websocket"),
        };
    });

    (read_rx, write_tx)
}
