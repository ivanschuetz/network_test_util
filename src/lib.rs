use algonaut::{
    algod::v2::Algod,
    core::SuggestedTransactionParams,
    transaction::{
        account::Account, tx_group::TxGroup, AcceptAsset, CreateAsset, TransferAsset, TxnBuilder,
    },
};
use mbase::{
    dependencies::{algod_for_net, network, Network},
    logger::init_logger,
    models::{
        asset_amount::AssetAmount,
        capi_deps::CapiAddress,
        funds::{FundsAmount, FundsAssetId},
    },
    util::algo_helpers::{send_tx_and_wait, send_txs_and_wait},
};
use tests_msig::TestsMsig;

use crate::test_data::{
    capi_owner, creator, customer, funds_asset_creator, investor1, investor2, msig_acc1, msig_acc2,
    msig_acc3,
};
use {
    anyhow::{anyhow, Result},
    dotenv::dotenv,
    fund_accounts_with_algos::fund_accounts_with_algos,
    std::env,
    std::process::Command,
    std::{
        io::{BufRead, BufReader},
        process::Stdio,
    },
};

pub mod fund_accounts_with_algos;
pub mod test_data;
pub mod tests_msig;

/// inits logs and resets the network
pub async fn test_init() -> Result<()> {
    let network = network();
    test_init_with_network(&network).await
}

/// inits logs and resets the network
pub async fn test_init_with_network(network: &Network) -> Result<()> {
    // load vars in .env file
    dotenv().ok();

    if env::var("TESTS_LOGGING")?.parse::<i32>()? == 1 {
        init_logger()?;
        log::debug!("Logging is enabled");
    }

    log::info!("Will reset the network..");
    reset_network(&network)?;

    log::info!("Will fund accounts with algos..");
    fund_accounts_with_algos(&network).await?;

    Ok(())
}

pub fn reset_network(net: &Network) -> Result<()> {
    let mut cmd = Command::new("sh");

    let cmd_with_net_args = match net {
        &Network::SandboxPrivate => cmd
            .current_dir(format!("scripts/sandbox"))
            .arg("./sandbox_dev_reset.sh"),

        // May be restored in the future, not deleting the variant for now.
        Network::Private => {
            panic!("Private network not supported anymore.")
        }
        Network::Test => panic!("Not supported: reseting testnet"),
    };

    log::debug!("Will execute script cmd..");

    // let error_res = cmd_with_net_args.stdout(Stdio::piped()).spawn()?.stderr;
    // log::debug!("Script cmd stderr: {:?}", error_res);

    let reset_res = cmd_with_net_args
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .expect("Couldn't reset network");

    log::debug!("Script cmd stdout: {:?}", reset_res);

    for line in BufReader::new(reset_res)
        .lines()
        .filter_map(|line| line.ok())
    {
        log::debug!("{}", line);

        if line.contains("No active sandbox to reset.") {
            return Err(anyhow!(
                "Please start the sandbox first. e.g: `sandbox up dev -v`"
            ));
        }
    }

    log::debug!("Script finished");

    Ok(())
}

/// calls setup_on_chain_deps with default dependencies
pub async fn do_setup_on_chain_deps(net: &Network) -> Result<OnChainDeps> {
    let algod = algod_for_net(net);
    let capi_owner = capi_owner();

    let deps = setup_on_chain_deps(&algod, &capi_owner).await?;
    log::info!("ℹ️ Capi deps: {deps:?}");

    Ok(deps)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnChainDeps {
    pub funds_asset_id: FundsAssetId,
    // the address to which the platform fees are directed
    pub capi_address: CapiAddress,
}

/// Creates the funds asset and capi dependencies, and funds the default accounts
pub async fn setup_on_chain_deps(algod: &Algod, capi_owner: &Account) -> Result<OnChainDeps> {
    let params = algod.suggested_transaction_params().await?;

    let funds_asset_id = create_and_distribute_funds_asset(algod).await?;

    log::info!("Will optin and send funds asset to msig test account..");

    optin_and_send_asset_to_msig(
        algod,
        &params,
        funds_asset_id.0,
        test_accounts_initial_funds().0,
        &funds_asset_creator(),
        &msig()?,
    )
    .await?;

    Ok(OnChainDeps {
        funds_asset_id,
        capi_address: CapiAddress(capi_owner.address()),
    })
}

pub async fn create_and_distribute_funds_asset(algod: &Algod) -> Result<FundsAssetId> {
    let params = algod.suggested_transaction_params().await?;

    let asset_creator = funds_asset_creator();
    let asset_id = create_funds_asset(algod, &params, &asset_creator).await?;

    log::info!("Will optin and send fund asset to test accounts..");

    // we want to only opt-in, not fund the capi owner. the capi owner is assumed to start without any funding
    // no reason other than backwards compatibility with tests
    optin_and_submit(algod, &params, asset_id.0, &capi_owner()).await?;

    let accounts = &[
        creator(),
        investor1(),
        investor2(),
        customer(),
        msig_acc1(),
        msig_acc2(),
        msig_acc3(),
    ];

    let initial_funds = test_accounts_initial_funds();

    // Log the funded addresses
    let addresses_str = accounts
        .into_iter()
        .map(|a| a.address().to_string())
        .collect::<Vec<String>>()
        .join(", ");
    log::debug!(
        "Funding accounts: {addresses_str} with: {} of funds asset: {}",
        initial_funds.0,
        asset_id.0
    );

    optin_and_fund_accounts_with_asset(
        algod,
        &params,
        asset_id.0,
        initial_funds,
        &asset_creator,
        accounts,
    )
    .await?;

    Ok(asset_id)
}

async fn create_funds_asset(
    algod: &Algod,
    params: &SuggestedTransactionParams,
    creator: &Account,
) -> Result<FundsAssetId> {
    log::info!("Will create funds asset..");

    let t = TxnBuilder::with(
        params,
        // 10 quintillions
        CreateAsset::new(creator.address(), 10_000_000_000_000_000_000, 6, false)
            .unit_name("TEST".to_owned())
            .asset_name("Test".to_owned())
            .build(),
    )
    .build()?;

    let signed_t = creator.sign_transaction(t)?;

    let p_tx = send_tx_and_wait(&algod, &signed_t).await?;
    let asset_id = p_tx
        .asset_index
        .ok_or_else(|| anyhow!("Couldn't retrieve asset id from pending tx"))?;

    log::info!("Created funds asset: {}", asset_id);

    Ok(FundsAssetId(asset_id))
}

/// Note that sending of algos to the msig address is done in fund_accounts_sandbox.sh. This flow could be improved (TODO low prio)
async fn optin_and_send_asset_to_msig(
    algod: &Algod,
    params: &SuggestedTransactionParams,
    asset_id: u64,
    amount: AssetAmount,
    sender: &Account,
    receiver: &TestsMsig,
) -> Result<()> {
    // optin the receiver to the asset
    let mut optin_tx = TxnBuilder::with(
        params,
        AcceptAsset::new(receiver.address().address(), asset_id).build(),
    )
    .build()?;

    let mut fund_tx = TxnBuilder::with(
        params,
        TransferAsset::new(
            sender.address(),
            asset_id,
            amount.0,
            receiver.address().address(),
        )
        .build(),
    )
    .build()?;

    TxGroup::assign_group_id(&mut [&mut optin_tx, &mut fund_tx])?;

    let optin_tx_signed = receiver.sign(optin_tx)?;
    let fund_tx_signed = sender.sign_transaction(fund_tx)?;

    send_txs_and_wait(&algod, &[optin_tx_signed, fund_tx_signed]).await?;

    log::debug!(
        "Opted in and funded (funds asset): {}",
        receiver.address().address()
    );

    Ok(())
}

fn test_accounts_initial_funds() -> FundsAmount {
    FundsAmount::new(100_000_000_000)
}

pub fn msig() -> Result<TestsMsig> {
    Ok(TestsMsig::new(vec![msig_acc1(), msig_acc2(), msig_acc3()])?)
}

async fn optin_and_submit(
    algod: &Algod,
    params: &SuggestedTransactionParams,
    asset_id: u64,
    account: &Account,
) -> Result<()> {
    // optin the receiver to the asset
    let optin_tx = TxnBuilder::with(
        params,
        AcceptAsset::new(account.address(), asset_id).build(),
    )
    .build()?;

    let optin_tx_signed = account.sign_transaction(optin_tx)?;

    send_txs_and_wait(&algod, &[optin_tx_signed]).await?;

    log::debug!("Opted in: {}, to asset: {asset_id}", account.address());

    Ok(())
}

pub async fn optin_and_fund_accounts_with_asset(
    algod: &Algod,
    params: &SuggestedTransactionParams,
    asset_id: u64,
    amount: FundsAmount,
    sender: &Account,
    accounts: &[Account],
) -> Result<()> {
    for account in accounts {
        optin_and_send_asset_to_account(algod, params, asset_id, amount.0, sender, &account)
            .await?;
    }
    Ok(())
}

pub async fn optin_and_send_asset_to_account(
    algod: &Algod,
    params: &SuggestedTransactionParams,
    asset_id: u64,
    amount: AssetAmount,
    sender: &Account,
    receiver: &Account,
) -> Result<()> {
    // optin the receiver to the asset
    let mut optin_tx = TxnBuilder::with(
        params,
        AcceptAsset::new(receiver.address(), asset_id).build(),
    )
    .build()?;

    let mut fund_tx = TxnBuilder::with(
        params,
        TransferAsset::new(sender.address(), asset_id, amount.0, receiver.address()).build(),
    )
    .build()?;

    TxGroup::assign_group_id(&mut [&mut optin_tx, &mut fund_tx])?;

    let optin_tx_signed = receiver.sign_transaction(optin_tx)?;
    let fund_tx_signed = sender.sign_transaction(fund_tx)?;

    send_txs_and_wait(&algod, &[optin_tx_signed, fund_tx_signed]).await?;

    log::debug!(
        "Opted in and funded: {}, asset: {asset_id}",
        receiver.address()
    );

    Ok(())
}
