use anyhow::Result;
use network_test_util::wasm::reset_and_fund_local_network;

#[tokio::main]
async fn main() -> Result<()> {
    reset_and_fund_local_network().await?;
    log::info!("Network reset and configured.");
    Ok(())
}
