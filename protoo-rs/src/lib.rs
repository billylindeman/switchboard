use serde::{Serialize,Deserialize};

pub mod server;

#[derive(Serialize,Deserialize,Debug)]
#[serde(untagged)]
pub enum Message {
    Request{ id: i32, method: String},
    Response{ id: i32, method: String},
    Error{},
    Notification{ id: i32, method: String}
}

pub struct Request<T> {
    pub id: i32,
    pub method: String,
    pub data: T
}

pub struct Response<T> {
    pub id: i32,
    pub method: String,
    pub data: T
}


