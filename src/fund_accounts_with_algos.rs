use algonaut::{
    algod::v2::Algod,
    core::{Address, MicroAlgos},
    kmd::v1::Kmd,
    model::indexer::v2::QueryAccount,
    transaction::{Pay, TxnBuilder},
};
use anyhow::{anyhow, Result};
use mbase::{
    dependencies::{algod_for_net, indexer_for_net, Network},
    util::algo_helpers::wait_for_p_tx_with_id,
};

// note that this was part initially of the shell script that was called in "reset_network"
// we had to move it to rust due to issues in a WSL2 environment
pub async fn fund_accounts_with_algos(net: &Network) -> Result<()> {
    let funder = choose_a_funder_address(net).await?;

    let algod = algod_for_net(net);
    let kmd = sandbox_private_network_kmd();

    // this was in the original shell script, not sure what for. might be outdated
    // // import additional account
    // 7ZLNWP5YP5DCCCLHAYYETZQLFH4GTBEKTBFQDHA723I7BBZ2FKCOZCBE4I
    // sandbox goal account import -m "group slush snack cram emotion echo cousin viable fan all pupil solar total boss deny under master rely wine help trick mechanic glance abstract clever"

    // kmd.import_key(wallet_handle, private_key)

    // fund the "funds asset" source - this is an account dedicated solely to mint the funds asset and distribute it to the test accounts
    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("DNQPINWK4K5QZYLCK7DVJFEWRUXPXGW36TEUIHNSNOFYI2RMPG2BZPQ7DE")?,
    )
    .await?;

    // fund our test accounts
    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("STOUDMINSIPP7JMJMGXVJYVS6HHD3TT5UODCDPYGV6KBGP7UYNTLJVJJME")?,
    )
    .await?;

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("7XSZQUQ2GJB25W37LVM5R4CMKKVC4VNSMIPCIWJYWM5ORA5VA4JRCNOJ4Y")?,
    )
    .await?;

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("PGCS3D5JL4AIFGTBPDGGMMCT3ODKUUFEFG336MJO25CGBG7ORKVOE3AHSU")?,
    )
    .await?;

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("7ZLNWP5YP5DCCCLHAYYETZQLFH4GTBEKTBFQDHA723I7BBZ2FKCOZCBE4I")?,
    )
    .await?;

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("NIKGABIQLRCPJYCNCFZWR7GUIC3NA66EBVR65JKHKLGLIYQ4KO3YYPV67Q")?,
    )
    .await?;

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("KPV7XSMNSRSQ44QCDQY7I6MORAB4GGT3GRY4WUNTCZZNRKSL4UEBPTJP2U")?,
    )
    .await?;

    // multisig accounts

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("DN7MBMCL5JQ3PFUQS7TMX5AH4EEKOBJVDUF4TCV6WERATKFLQF4MQUPZTA")?,
    )
    .await?;

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("GIZTTA56FAJNAN7ACK3T6YG34FH32ETDULBZ6ENC4UV7EEHPXJGGSPCMVU")?,
    )
    .await?;

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("BFRTECKTOOE7A5LHCF3TTEOH2A7BW46IYT2SX5VP6ANKEXHZYJY77SJTVM")?,
    )
    .await?;

    // multisig address

    send_algos(
        &algod,
        &kmd,
        &funder,
        &to_address("BSAWQANNI3VWCQH3RCJLDHR27XEYTQYVBLTQ3C2MW5GRULCKFQBEWPDV6E")?,
    )
    .await?;

    Ok(())
}

fn to_address(str: &str) -> Result<Address> {
    Ok(str
        .parse()
        .map_err(|e| anyhow!("Couldn't parse: {str} to address, error: {e:?}"))?)
}

async fn send_algos(algod: &Algod, kmd: &Kmd, sender: &Address, receiver: &Address) -> Result<()> {
    let amount = MicroAlgos(10_000_000_000);

    let params = algod.suggested_transaction_params().await?;
    let tx = TxnBuilder::with(&params, Pay::new(*sender, *receiver, amount).build()).build()?;

    let wallet_password = "";
    let wallet_handle_token = wallet_handle_token(&kmd, "").await?;

    let sign_response = kmd
        .sign_transaction(&wallet_handle_token, wallet_password, &tx)
        .await?;

    let send_response = algod
        .broadcast_raw_transaction(&sign_response.signed_transaction)
        .await?;

    wait_for_p_tx_with_id(&algod, &send_response.tx_id.parse()?).await?;

    Ok(())
}

async fn wallet_handle_token(kmd: &Kmd, wallet_password: &str) -> Result<String> {
    let list_response = kmd.list_wallets().await?;
    let wallet_id = match list_response
        .wallets
        .into_iter()
        .find(|wallet| wallet.name == "unencrypted-default-wallet")
    {
        Some(wallet) => wallet.id,
        None => return Err(anyhow!("Wallet not found")),
    };
    let init_response = kmd.init_wallet_handle(&wallet_id, wallet_password).await?;
    let wallet_handle_token = init_response.wallet_handle_token;

    Ok(wallet_handle_token)
}

/// Chooses an arbitrary account from the network, assumed to be sufficiently funded.
/// Usually this is called with a new sandbox, where the initial accounts have quadrillions.
async fn choose_a_funder_address(net: &Network) -> Result<Address> {
    let indexer = indexer_for_net(net);
    let accounts = indexer.accounts(&QueryAccount::default()).await?.accounts;

    for a in &accounts {
        log::debug!("{} -> {:?}", a.address, a.amount);

        // sum copy pasted from one sandbox account. it can be lowered
        if a.amount >= MicroAlgos(2_000_000_000_000_000) {
            return Ok(a.address);
        }
    }

    Err(anyhow!(
        "Environment: {net:?} has no accounts or accounts with sufficient funds. Please check that it has been setup correctly."
    ))
}

// TODO for net, like algod
fn sandbox_private_network_kmd() -> Kmd {
    Kmd::new(
        "http://127.0.0.1:4002",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .expect("Couldn't initialize sandbox kmd")
}
