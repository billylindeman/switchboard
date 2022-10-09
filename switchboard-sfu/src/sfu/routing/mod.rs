mod router;
mod subscriber;

pub use router::*;
pub use subscriber::*;

// Layer
// Used by router & subscriber to determine which video stream to send
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum Layer {
    None,
    // Single video stream
    Unicast,
    // Simulcast layer w/RID
    Rid(String),
}

impl Layer {
    pub fn from(rid: &str) -> Layer {
        match rid {
            "" => Layer::Unicast,
            layer => Layer::Rid(layer.to_owned()),
        }
    }
}
