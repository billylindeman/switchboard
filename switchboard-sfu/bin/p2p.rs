use futures::StreamExt;
use std::env;
use std::error::Error;
use tokio::io::{self, AsyncBufReadExt};

use libp2p::{floodsub, swarm::SwarmEvent};

use switchboard_sfu::p2p;
use switchboard_sfu::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "switchboard=info");
    }
    pretty_env_logger::init();

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7000".to_string());

    tokio::spawn(async move {
        signal::run_server(&addr).await;
    });

    let topic = floodsub::Topic::new("switchboard-announce");

    let mut swarm = p2p::node::build_swarm(topic.clone()).await?;

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
