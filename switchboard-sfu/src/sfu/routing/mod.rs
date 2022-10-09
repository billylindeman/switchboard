mod router;
mod subscriber;

pub use router::*;
pub use subscriber::*;

// Layer
// Used by router & subscriber to determine which video stream to send
pub enum Layer {
    // Mute Stream
    None,
    // Single video stream
    Unicast,
    // Simulcast layer w/RID
    Rid(String),
}
