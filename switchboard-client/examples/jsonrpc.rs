#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let server = "ws://localhost:7000";

    let jsonrpc = switchboard_client::signal::JsonRPCWebsocket::connect(server.to_owned()).await;

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    drop(jsonrpc);
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}
