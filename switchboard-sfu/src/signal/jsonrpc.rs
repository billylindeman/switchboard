use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request<T> {
    id: String,
    method: String,
    params: T,
}

#[derive(Serialize, Deserialize)]
pub struct Response<T> {
    id: String,
    method: String,
    params: T,
}

#[derive(Serialize, Deserialize)]
pub struct Notification<T> {
    method: String,
    params: T,
}
