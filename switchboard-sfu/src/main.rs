use std::net::SocketAddr;

use jsonrpsee::core::client::ClientT;
use jsonrpsee::ws_client::WsClientBuilder;
use jsonrpsee::ws_server::{RpcModule, WsServerBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .expect("setting default subscriber failed");

    let addr = run_server().await?;
    let url = format!("ws://{}", addr);

    let client = WsClientBuilder::default().build(&url).await?;
    let response: String = client.request("say_hello", None).await?;
    tracing::info!("response: {:?}", response);

    Ok(())
}

async fn run_server() -> anyhow::Result<SocketAddr> {
    let server = WsServerBuilder::default().build("127.0.0.1:0").await?;
    let mut module = RpcModule::new(());
    module.register_method("say_hello", |_, _| Ok("lo"))?;
    let addr = server.local_addr()?;
    server.start(module)?;
    Ok(addr)
}
