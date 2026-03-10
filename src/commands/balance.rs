use anyhow::Result;
use tronic::{
    contracts::{trc20::{Trc20Calls, Trc20Contract}, token::usdt::Usdt},
    domain::address::TronAddress,
    domain::trx::Trx,
    signer::LocalSigner,
};

use crate::{
    cli::Token,
    client,
    config::Network,
};

pub async fn run(
    network: Network,
    address: Option<String>,
    token: Option<Token>,
    signer: Option<LocalSigner>,
    api_key: Option<&str>,
) -> Result<()> {
    let target: TronAddress = match (&address, &signer) {
        (Some(addr), _) => crate::error::parse_address(addr)?,
        (None, Some(s)) => s.address(),
        (None, None) => anyhow::bail!(
            "no address specified and no wallet loaded; use --address or set up a wallet"
        ),
    };

    let signer = signer.unwrap_or_else(LocalSigner::rand);
    let client = client::build(network, signer, api_key).await?;

    match token {
        Some(Token::Trx) => {
            let balance = client.trx_balance().address(target).get().await?;
            println!("{balance}");
        }
        Some(Token::Usdt) => {
            let balance = get_usdt_balance(&client, network, target).await?;
            println!("{balance}");
        }
        None => {
            let trx = client.trx_balance().address(target).get().await?;
            let usdt = get_usdt_balance(&client, network, target).await?;

            if trx != Trx::default() {
                println!("{trx}");
            }
            if usdt != Usdt::default() {
                println!("{usdt}");
            }
            if trx == Trx::default() && usdt == Usdt::default() {
                println!("No balances");
            }
        }
    }

    Ok(())
}

async fn get_usdt_balance(
    client: &crate::client::TronClient,
    network: Network,
    target: TronAddress,
) -> Result<Usdt> {
    let usdt_addr = crate::error::parse_address(network.usdt_contract())?;
    let contract = Trc20Contract::<Usdt>::new(usdt_addr);
    let balance: Usdt = client
        .trc20_balance_of()
        .contract(contract)
        .owner(target)
        .get()
        .await?;
    Ok(balance)
}
