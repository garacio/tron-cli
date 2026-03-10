use std::io::{self, Write};

use anyhow::Result;
use tronic::{
    client::pending::AutoSigning,
    contracts::{trc20::{Trc20Calls, Trc20Contract}, token::usdt::Usdt},
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
    to: &str,
    amount: &str,
    token: Token,
    skip_confirm: bool,
    signer: LocalSigner,
    api_key: Option<&str>,
) -> Result<()> {
    let recipient = crate::error::parse_address(to)?;
    let from = signer.address();

    match token {
        Token::Trx => {
            let trx_amount: Trx = amount
                .parse::<f64>()
                .map_err(|_| anyhow::anyhow!("invalid amount: {amount}"))?
                .into();

            if !skip_confirm {
                confirm(&format!(
                    "Send {trx_amount} TRX from {from} to {recipient}?"
                ))?;
            }

            let client = client::build(network, signer, api_key).await?;
            let txid = client
                .send_trx()
                .to(recipient)
                .amount(trx_amount)
                .can_spend_trx_for_fee(true)
                .build::<AutoSigning>()
                .await?
                .broadcast(&())
                .await?;

            println!("TX sent: {txid:?}");
        }
        Token::Usdt => {
            let usdt_amount = Usdt::from_decimal(
                amount
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("invalid amount: {amount}"))?,
            )
            .map_err(|e| anyhow::anyhow!("invalid USDT amount: {e}"))?;

            if !skip_confirm {
                confirm(&format!(
                    "Send {usdt_amount} USDT from {from} to {recipient}?"
                ))?;
            }

            let usdt_addr = crate::error::parse_address(network.usdt_contract())?;
            let contract = Trc20Contract::<Usdt>::new(usdt_addr);

            let client = client::build(network, signer, api_key).await?;
            let txid = client
                .trc20_transfer()
                .contract(contract)
                .to(recipient)
                .amount(usdt_amount)
                .can_spend_trx_for_fee(true)
                .build::<AutoSigning>()
                .await?
                .broadcast(&())
                .await?;

            println!("TX sent: {txid:?}");
        }
    }

    Ok(())
}

fn confirm(msg: &str) -> Result<()> {
    print!("{msg} [y/N] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        anyhow::bail!("aborted by user");
    }
    Ok(())
}
