use futures_util::{future, Stream, StreamExt, TryStreamExt};
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;

use anyhow::format_err;
use tokio_tungstenite::WebSocketStream;

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
enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

fn parse_msg_type(msg_text: String) -> anyhow::Result<Message> {
    let msg: Message = serde_json::from_str(&msg_text)?;
    Ok(msg)
}

pub async fn handle_messages(stream: WebSocketStream<tokio::net::TcpStream>) {
    let (write, read) = stream.split();

    read.try_filter(|msg| future::ready(msg.is_text()))
        .map(|msg| msg.unwrap().to_string())
        .map(parse_msg_type)
        .filter(|msg| {
            if let Err(e) = msg {
                error!("error parsing message: {}", e);
                return future::ready(false);
            }
            future::ready(true)
        })
        .map(|msg| info!("got message: {:#?}", msg))
        .collect()
        .await
}
