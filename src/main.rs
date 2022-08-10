use anyhow::Result;
use mbase::dependencies::network;
use network_test_util::{do_setup_on_chain_deps, test_init_with_network};

#[tokio::main]
async fn main() -> Result<()> {
    let network = network();

    test_init_with_network(&network).await?;

    log::info!("Will setup on chain deps..");
    do_setup_on_chain_deps(&network).await?;

    log::info!("✅ Network reset and configured.");
    Ok(())
}
