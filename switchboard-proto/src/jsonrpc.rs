use futures_channel::oneshot;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Id {
    Uuid(String),
    Int(i32),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Request {
    pub id: Id,
    pub method: String,
    pub params: Map<String, Value>,

    #[serde(skip)]
    pub result: Option<oneshot::Sender<Response>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Response {
    pub id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Notification {
    pub method: String,
    pub params: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
/// JsonRPC Event
pub enum Event {
    Request(Request),
    Response(Response),
    Notification(Notification),
}
