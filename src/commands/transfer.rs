use std::io::{self, Write};

use anyhow::Result;
use tronic::{
    client::pending::ManualSigning,
    contracts::{trc20::{Trc20Calls, Trc20Contract}, token::usdt::Usdt},
    domain::{estimate::ResourceState, trx::Trx},
    signer::LocalSigner,
};

use crate::{
    cli::Token,
    client,
    config::Network,
    trongrid::TronGridClient,
};

/// Size of the PTX1 serialization header before activation checks.
const PTX1_HEADER: usize = 4 + 32 + 21 + 8 + 1 + 1; // magic + txid + owner + base_trx + can_spend + count
/// Size of each activation check entry.
const PTX1_CHECK: usize = 21 + 8; // address + fee

/// Extract protobuf Transaction bytes from PendingTransaction::serialize() output.
fn extract_proto_bytes(serialized: &[u8]) -> &[u8] {
    let count = serialized[PTX1_HEADER - 1] as usize;
    let offset = PTX1_HEADER + count * PTX1_CHECK;
    &serialized[offset..]
}

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
    let client = client::build(network, signer.clone(), api_key).await?;

    match token {
        Token::Trx => {
            let trx_amount: Trx = amount
                .parse::<f64>()
                .map_err(|_| anyhow::anyhow!("invalid amount: {amount}"))?
                .into();

            let mut pending = client
                .send_trx()
                .to(recipient)
                .amount(trx_amount)
                .can_spend_trx_for_fee(true)
                .build::<ManualSigning>()
                .await?;

            let estimate = pending.estimate_transaction().await?;

            if !skip_confirm {
                let trx_price = fetch_trx_price_usd().await;
                print_summary(&network, &format!("{trx_amount}"), &from, &recipient, &estimate, trx_price);
                confirm()?;
            }

            // Sign and broadcast via REST (TronGrid blocks gRPC BroadcastTransaction)
            pending.sign(&signer, &()).await?;
            let txid = pending.txid();
            let proto_hex = hex::encode(extract_proto_bytes(&pending.serialize()));
            let tg = TronGridClient::new(network, api_key)?;
            tg.broadcast_hex(&proto_hex).await?;
            println!("TX sent: {txid:?}");
        }
        Token::Usdt => {
            let usdt_amount = Usdt::from_decimal(
                amount
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("invalid amount: {amount}"))?,
            )
            .map_err(|e| anyhow::anyhow!("invalid USDT amount: {e}"))?;

            let usdt_addr = crate::error::parse_address(network.usdt_contract())?;
            let contract = Trc20Contract::<Usdt>::new(usdt_addr);

            let mut pending = client
                .trc20_transfer()
                .contract(contract)
                .to(recipient)
                .amount(usdt_amount)
                .can_spend_trx_for_fee(true)
                .build::<ManualSigning>()
                .await?;

            let estimate = pending.estimate_transaction().await?;

            if !skip_confirm {
                let trx_price = fetch_trx_price_usd().await;
                print_summary(&network, &format!("{usdt_amount}"), &from, &recipient, &estimate, trx_price);
                confirm()?;
            }

            // Sign and broadcast via REST (TronGrid blocks gRPC BroadcastTransaction)
            pending.sign(&signer, &()).await?;
            let txid = pending.txid();
            let proto_hex = hex::encode(extract_proto_bytes(&pending.serialize()));
            let tg = TronGridClient::new(network, api_key)?;
            tg.broadcast_hex(&proto_hex).await?;
            println!("TX sent: {txid:?}");
        }
    }

    Ok(())
}

/// Fetch current TRX/USD price from CoinGecko. Returns None on any error.
async fn fetch_trx_price_usd() -> Option<f64> {
    #[derive(serde::Deserialize)]
    struct CoinGeckoResponse {
        tron: Option<TronPrice>,
    }
    #[derive(serde::Deserialize)]
    struct TronPrice {
        usd: Option<f64>,
    }

    let client = reqwest::Client::builder()
        .user_agent("tron-cli")
        .build()
        .ok()?;
    let resp = client
        .get("https://api.coingecko.com/api/v3/simple/price?ids=tron&vs_currencies=usd")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?
        .json::<CoinGeckoResponse>()
        .await
        .ok()?;
    resp.tron?.usd
}

fn parse_trx_display(trx: &impl std::fmt::Display) -> f64 {
    let s = format!("{trx}");
    s.trim_end_matches(" TRX").parse::<f64>().unwrap_or(0.0)
}

/// Extract TRX fee as f64 from the estimate.
fn extract_fee_trx(estimate: &ResourceState) -> Option<f64> {
    if let Some(ref insufficient) = estimate.insufficient {
        let fee: f64 = insufficient
            .suggested_trx_topup
            .iter()
            .map(|(_, trx)| parse_trx_display(trx))
            .sum();
        Some(fee)
    } else {
        let consume = &estimate.will_consume;
        if consume.energy == 0 && consume.bandwidth == 0 {
            None // free
        } else {
            let trx_val = parse_trx_display(&consume.trx);
            if trx_val > 0.0 { Some(trx_val) } else { None }
        }
    }
}

fn format_fee(estimate: &ResourceState, trx_price: Option<f64>) -> String {
    let usd_suffix = |fee_trx: f64| -> String {
        match trx_price {
            Some(price) => format!(" (~${:.2})", fee_trx * price),
            None => String::new(),
        }
    };

    if let Some(ref insufficient) = estimate.insufficient {
        let fee: f64 = insufficient
            .suggested_trx_topup
            .iter()
            .map(|(_, trx)| parse_trx_display(trx))
            .sum();
        let balance = parse_trx_display(&insufficient.account_balance);
        if balance >= fee {
            format!("~{fee:.4} TRX{} (from balance)", usd_suffix(fee))
        } else {
            let deficit = fee - balance;
            format!(
                "~{fee:.4} TRX{} (INSUFFICIENT — need ~{deficit:.4} TRX more)",
                usd_suffix(fee),
            )
        }
    } else {
        match extract_fee_trx(estimate) {
            Some(trx_val) => format!("~{trx_val:.4} TRX{}", usd_suffix(trx_val)),
            None => "free (covered by staked resources)".to_string(),
        }
    }
}

fn print_summary(
    network: &Network,
    amount: &str,
    from: &impl std::fmt::Display,
    to: &impl std::fmt::Display,
    estimate: &ResourceState,
    trx_price: Option<f64>,
) {
    let consume = &estimate.will_consume;
    println!();
    println!("  Network:   {network}");
    println!("  From:      {from}");
    println!("  To:        {to}");
    println!("  Amount:    {amount}");
    println!("  Energy:    {}", consume.energy);
    println!("  Bandwidth: {}", consume.bandwidth);
    println!("  Fee:       {}", format_fee(estimate, trx_price));
    println!();
}

fn confirm() -> Result<()> {
    print!("Confirm transfer? [y/N] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        anyhow::bail!("aborted by user");
    }
    Ok(())
}
