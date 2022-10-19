use std::env;

use switchboard_sfu::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "switchboard=info");
    }
    pretty_env_logger::init();

    let _extip = switchboard_sfu::extip::resolve_external_ip().await?;

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7000".to_string());

    signal::run_server(&addr).await;

    Ok(())
}
