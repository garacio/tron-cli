use anyhow::Result;
use tronic::signer::LocalSigner;

use crate::{
    cli::Token,
    config::Network,
    trongrid::TronGridClient,
};

pub async fn run(
    network: Network,
    address: Option<String>,
    token: Token,
    limit: u32,
    wide: bool,
    signer: Option<LocalSigner>,
    api_key: Option<&str>,
) -> Result<()> {
    let addr = match (&address, &signer) {
        (Some(a), _) => {
            crate::error::validate_address(a)?;
            a.clone()
        }
        (None, Some(s)) => s.address().to_string(),
        (None, None) => anyhow::bail!(
            "no address specified and no wallet loaded; use --address or set up a wallet"
        ),
    };

    let client = TronGridClient::new(network, api_key)?;

    match token {
        Token::Trx => show_trx_history(&client, &addr, limit, wide).await,
        Token::Usdt => show_trc20_history(&client, &addr, limit, wide).await,
    }
}

async fn show_trx_history(
    client: &TronGridClient,
    address: &str,
    limit: u32,
    wide: bool,
) -> Result<()> {
    let resp = client.transactions(address, limit, None).await?;

    if resp.data.is_empty() {
        println!("No transactions found.");
        return Ok(());
    }

    if wide {
        println!(
            "{:<19}  {:<10} {:<3} {:>20}   {:<34}  {}",
            "DATE", "STATUS", "DIR", "AMOUNT", "COUNTERPARTY", "TXID"
        );
        println!("{}", "-".repeat(160));
    } else {
        println!(
            "{:<16}  {:<10} {:<3} {:>20}   {:<14}  {}",
            "DATE", "STATUS", "DIR", "AMOUNT", "COUNTERPARTY", "TXID"
        );
        println!("{}", "-".repeat(95));
    }

    for tx in &resp.data {
        let ts = format_timestamp(tx.block_timestamp);
        let status = tx
            .ret
            .as_ref()
            .and_then(|r| r.first())
            .and_then(|r| r.contract_ret.as_deref())
            .unwrap_or("UNKNOWN");

        let (direction, counterparty, amount) = parse_transfer(tx, address, wide);
        let txid = fmt_txid(&tx.tx_id, wide);

        println!(
            "{ts}  {status:<10} {direction} {amount} {counterparty}  {txid}",
        );
    }

    if let Some(meta) = &resp.meta {
        if meta.fingerprint.is_some() {
            println!("  ... more transactions available");
        }
    }

    Ok(())
}

async fn show_trc20_history(
    client: &TronGridClient,
    address: &str,
    limit: u32,
    wide: bool,
) -> Result<()> {
    let resp = client.trc20_transactions(address, limit, None).await?;

    if resp.data.is_empty() {
        println!("No TRC20 transactions found.");
        return Ok(());
    }

    if wide {
        println!(
            "{:<16}  {:<3} {:>14} {:<6} {:<34}  {}",
            "DATE", "DIR", "AMOUNT", "TOKEN", "COUNTERPARTY", "TXID"
        );
        println!("{}", "-".repeat(150));
    } else {
        println!(
            "{:<16}  {:<3} {:>14} {:<6} {:<14}  {}",
            "DATE", "DIR", "AMOUNT", "TOKEN", "COUNTERPARTY", "TXID"
        );
        println!("{}", "-".repeat(80));
    }

    for tx in &resp.data {
        let ts = format_timestamp(tx.block_timestamp);
        let symbol = tx
            .token_info
            .as_ref()
            .and_then(|t| t.symbol.as_deref())
            .unwrap_or("???");
        let decimals = tx
            .token_info
            .as_ref()
            .and_then(|t| t.decimals)
            .unwrap_or(0);

        let direction = if addr_eq(&tx.from, address) {
            "OUT"
        } else {
            " IN"
        };
        let counterparty = if direction == "OUT" { &tx.to } else { &tx.from };
        let amount = format_token_amount(&tx.value, decimals);
        let who = fmt_addr(counterparty, wide);
        let txid = fmt_txid(&tx.transaction_id, wide);

        println!(
            "{ts}  {direction} {amount:>14} {symbol:<6} {who}  {txid}",
        );
    }

    if let Some(meta) = &resp.meta {
        if meta.fingerprint.is_some() {
            println!("  ... more transactions available");
        }
    }

    Ok(())
}

/// Parse a TRX transfer entry into (direction, counterparty, formatted amount).
fn parse_transfer(
    tx: &crate::trongrid::TxEntry,
    my_address: &str,
    wide: bool,
) -> (&'static str, String, String) {
    let contract = tx
        .raw_data
        .as_ref()
        .and_then(|r| r.contract.as_ref())
        .and_then(|c| c.first());

    let contract_type = contract
        .and_then(|c| c.contract_type.as_deref())
        .unwrap_or("Unknown");

    let value = contract.and_then(|c| c.parameter.as_ref()).and_then(|p| p.value.as_ref());

    match contract_type {
        "TransferContract" => {
            let owner = value
                .and_then(|v| v.get("owner_address"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let to = value
                .and_then(|v| v.get("to_address"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let amount_sun = value
                .and_then(|v| v.get("amount"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let amount_trx = amount_sun as f64 / 1_000_000.0;

            let (dir, counterparty) = if addr_eq(owner, my_address) {
                ("OUT", to)
            } else {
                (" IN", owner)
            };

            (dir, fmt_addr(counterparty, wide), format!("{amount_trx:>14.6} TRX   "))
        }
        "TriggerSmartContract" => {
            let owner = value
                .and_then(|v| v.get("owner_address"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let contract_addr = value
                .and_then(|v| v.get("contract_address"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let dir = if addr_eq(owner, my_address) { "OUT" } else { " IN" };
            (dir, fmt_addr(contract_addr, wide), "    (contract)    ".to_string())
        }
        _ => {
            ("   ", "".to_string(), format!("    ({contract_type})"))
        }
    }
}

fn fmt_addr(addr: &str, wide: bool) -> String {
    let base58 = to_base58(addr);
    if wide || base58.len() <= 14 {
        base58
    } else {
        format!("{}...{}", &base58[..6], &base58[base58.len() - 4..])
    }
}

fn fmt_txid(txid: &str, wide: bool) -> String {
    if wide || txid.len() <= 16 {
        txid.to_string()
    } else {
        format!("{}...{}", &txid[..8], &txid[txid.len() - 4..])
    }
}

/// Compare addresses that may be in different formats (hex vs base58).
fn addr_eq(a: &str, b: &str) -> bool {
    to_base58(a) == to_base58(b)
}

/// Convert 41-prefixed hex address to base58 Tron address.
fn to_base58(hex_addr: &str) -> String {
    let Ok(bytes) = hex::decode(hex_addr) else {
        return hex_addr.to_string();
    };
    if bytes.len() == 21 && bytes[0] == 0x41 {
        bs58::encode(&bytes).with_check().into_string()
    } else {
        hex_addr.to_string()
    }
}

fn format_timestamp(ms: i64) -> String {
    let secs = ms / 1000;
    let dt = chrono::DateTime::from_timestamp(secs, 0);
    match dt {
        Some(d) => d.format("%Y-%m-%d %H:%M").to_string(),
        None => format!("{ms}"),
    }
}

fn format_token_amount(raw: &str, decimals: u8) -> String {
    let value: f64 = raw.parse().unwrap_or(0.0);
    let divisor = 10f64.powi(decimals as i32);
    format!("{:.prec$}", value / divisor, prec = decimals.min(6) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── to_base58 ──

    #[test]
    fn to_base58_valid_hex_address() {
        // 41 + 20 bytes = known Tron address
        let hex = "41a614f803b6fd780986a42c78ec9c7f77e6ded13c";
        let result = to_base58(hex);
        assert!(result.starts_with('T'), "result: {result}");
        assert_eq!(result.len(), 34);
    }

    #[test]
    fn to_base58_already_base58() {
        let addr = "TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr";
        assert_eq!(to_base58(addr), addr);
    }

    #[test]
    fn to_base58_invalid_hex() {
        assert_eq!(to_base58("not-hex"), "not-hex");
    }

    #[test]
    fn to_base58_wrong_length() {
        assert_eq!(to_base58("41abcd"), "41abcd");
    }

    // ── addr_eq ──

    #[test]
    fn addr_eq_same_base58() {
        let a = "TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr";
        assert!(addr_eq(a, a));
    }

    #[test]
    fn addr_eq_hex_vs_base58() {
        let hex = "41a614f803b6fd780986a42c78ec9c7f77e6ded13c";
        let base58 = to_base58(hex);
        assert!(addr_eq(hex, &base58));
    }

    #[test]
    fn addr_eq_different() {
        assert!(!addr_eq(
            "TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr",
            "TNPeeaaFB7K9cmo4uQpcU32zGK8G1NYqeL"
        ));
    }

    // ── fmt_addr ──

    #[test]
    fn fmt_addr_compact() {
        let addr = "TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr";
        let short = fmt_addr(addr, false);
        assert!(short.contains("..."), "short: {short}");
        assert!(short.len() < addr.len());
    }

    #[test]
    fn fmt_addr_wide() {
        let addr = "TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr";
        assert_eq!(fmt_addr(addr, true), addr);
    }

    // ── fmt_txid ──

    #[test]
    fn fmt_txid_compact() {
        let txid = "0a767ff37ca54efc659199e75c6cb2c197294ff4dadb5f5f72b076f9d20fc4a7";
        let short = fmt_txid(txid, false);
        assert!(short.contains("..."), "short: {short}");
        assert!(short.starts_with("0a767ff3"));
    }

    #[test]
    fn fmt_txid_wide() {
        let txid = "0a767ff37ca54efc659199e75c6cb2c197294ff4dadb5f5f72b076f9d20fc4a7";
        assert_eq!(fmt_txid(txid, true), txid);
    }

    #[test]
    fn fmt_txid_short_passthrough() {
        let short = "abcdef1234567890";
        assert_eq!(fmt_txid(short, false), short);
    }

    // ── format_timestamp ──

    #[test]
    fn format_timestamp_valid() {
        // 2024-01-01 00:00:00 UTC = 1704067200 sec
        let ts = format_timestamp(1704067200_000);
        assert_eq!(ts, "2024-01-01 00:00");
    }

    #[test]
    fn format_timestamp_zero() {
        let ts = format_timestamp(0);
        assert_eq!(ts, "1970-01-01 00:00");
    }

    // ── format_token_amount ──

    #[test]
    fn format_token_amount_usdt() {
        // 100 USDT = 100_000_000 raw with 6 decimals
        assert_eq!(format_token_amount("100000000", 6), "100.000000");
    }

    #[test]
    fn format_token_amount_small() {
        assert_eq!(format_token_amount("1", 6), "0.000001");
    }

    #[test]
    fn format_token_amount_zero_decimals() {
        assert_eq!(format_token_amount("42", 0), "42");
    }

    #[test]
    fn format_token_amount_invalid_raw() {
        assert_eq!(format_token_amount("not-a-number", 6), "0.000000");
    }
}
