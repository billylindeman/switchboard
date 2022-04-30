use futures::StreamExt;
use std::error::Error;
use tokio::io::{self, AsyncBufReadExt};

use libp2p::{floodsub, swarm::SwarmEvent};
use switchboard::p2p::{self};

/// The `tokio::main` attribute sets up a tokio runtime.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let topic = floodsub::Topic::new("chat-123");

    let mut swarm = p2p::build_swarm(topic.clone()).await?;

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0/ws".parse()?)?;

    // Kick it off
    loop {
        tokio::select! {
            line = stdin.next_line() => {
                let line = line?.expect("stdin closed");
                swarm.behaviour_mut().floodsub.publish(topic.clone(), line.as_bytes());
            }
            event = swarm.select_next_some() => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    println!("Listening on {:?}", address);
                }
            }
        }
    }
}
