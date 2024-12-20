use futures::select;
use futures_util::{future, StreamExt, TryStreamExt};
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use futures_channel::{mpsc, oneshot};

use switchboard_proto::jsonrpc;
use tokio_tungstenite::{tungstenite, WebSocketStream};

pub type ReadStream = mpsc::UnboundedReceiver<anyhow::Result<jsonrpc::Event>>;
pub type WriteStream = mpsc::UnboundedSender<anyhow::Result<jsonrpc::Event>>;

/// This function processes the websocket stream into
/// a writer and reader for jsonrpc::Event's
pub async fn handle_messages(
    stream: WebSocketStream<tokio::net::TcpStream>,
) -> (ReadStream, WriteStream) {
    let (write, read) = stream.split();

    let (read_tx, read_rx) = mpsc::unbounded::<anyhow::Result<jsonrpc::Event>>();
    let (write_tx, write_rx) = mpsc::unbounded::<anyhow::Result<jsonrpc::Event>>();

    // Inbound message loop
    tokio::spawn(async move {
        let mut incoming_fut = read
            .map_err(|err| error!("websocket error: {}", err))
            .try_filter(|msg| future::ready(msg.is_text()))
            .map_ok(|msg| {
                trace!("websocket got msg {}", msg);
                msg
            })
            .map(|msg| serde_json::from_str::<jsonrpc::Event>(msg.unwrap().to_text().unwrap()))
            .map_err(|err| {
                error!("error parsing json: {}", err);
                err.into()
            })
            .map_ok(|msg| {
                trace!("jsonrpc got event: {:#?}", msg);
                msg
            })
            .filter(|r| future::ready(r.is_ok()))
            .map(Ok)
            .forward(read_tx);

        let mut outgoing_fut = write_rx
            .map_ok(|evt| serde_json::to_value(&evt).unwrap())
            .map_ok(|v| match v {
                Value::Object(m) => {
                    let mut m = m.clone();
                    m.insert("jsonrpc".to_owned(), Value::String("2.0".to_owned()));
                    Value::Object(m)
                }
                v => v.clone(),
            })
            .map_ok(|v| serde_json::to_string(&v).unwrap())
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
