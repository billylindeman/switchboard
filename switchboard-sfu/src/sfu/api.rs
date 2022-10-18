use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct SetRemoteMedia {
    #[serde(rename = "streamId")]
    pub stream_id: String,
    #[serde(rename = "video")]
    pub rid: String,
}
