use anyhow::Result;

use crate::{
    config::Network,
    trongrid::TronGridClient,
};

pub async fn run(
    network: Network,
    txid: &str,
    api_key: Option<&str>,
) -> Result<()> {
    let client = TronGridClient::new(network, api_key)?;

    let (raw, info) = tokio::try_join!(
        client.transaction_by_id(txid),
        client.transaction_info(txid),
    )?;

    // Basic tx info.
    let tx_id = info
        .id
        .as_deref()
        .or(raw.tx_id.as_deref())
        .unwrap_or(txid);
    println!("TX:     {tx_id}");

    if let Some(block) = info.block_number {
        println!("Block:  {block}");
    }

    if let Some(ts) = info.block_timestamp {
        println!("Time:   {}", format_timestamp(ts));
    }

    // Status.
    let status = raw
        .ret
        .as_ref()
        .and_then(|r| r.first())
        .and_then(|r| r.contract_ret.as_deref())
        .unwrap_or("UNKNOWN");
    println!("Status: {status}");

    // Contract type & details.
    if let Some(ref rd) = raw.raw_data {
        if let Some(ref contracts) = rd.contract {
            for c in contracts {
                let ctype = c.contract_type.as_deref().unwrap_or("Unknown");
                println!("Type:   {ctype}");

                if let Some(ref param) = c.parameter {
                    if let Some(ref val) = param.value {
                        print_contract_details(ctype, val);
                    }
                }
            }
        }
    }

    // Fees.
    if let Some(fee) = info.fee {
        println!("Fee:    {} TRX", fee as f64 / 1_000_000.0);
    }

    if let Some(ref receipt) = info.receipt {
        if let Some(energy) = receipt.energy_usage_total {
            println!("Energy: {energy}");
        }
        if let Some(bw) = receipt.net_usage {
            println!("BW:     {bw}");
        }
    }

    // Result message (error details).
    if let Some(ref msg) = info.res_message {
        if !msg.is_empty() {
            let decoded = hex_to_string(msg);
            println!("Msg:    {decoded}");
        }
    }

    Ok(())
}

fn print_contract_details(contract_type: &str, val: &serde_json::Value) {
    match contract_type {
        "TransferContract" => {
            if let Some(from) = val.get("owner_address").and_then(|v| v.as_str()) {
                println!("From:   {}", to_base58(from));
            }
            if let Some(to) = val.get("to_address").and_then(|v| v.as_str()) {
                println!("To:     {}", to_base58(to));
            }
            if let Some(amount) = val.get("amount").and_then(|v| v.as_i64()) {
                println!("Amount: {} TRX", amount as f64 / 1_000_000.0);
            }
        }
        "TriggerSmartContract" => {
            if let Some(from) = val.get("owner_address").and_then(|v| v.as_str()) {
                println!("From:   {}", to_base58(from));
            }
            if let Some(contract) = val.get("contract_address").and_then(|v| v.as_str()) {
                println!("Ctrct:  {}", to_base58(contract));
            }
            if let Some(data) = val.get("data").and_then(|v| v.as_str()) {
                // TRC20 transfer: a9059cbb + address(32B) + amount(32B)
                if data.starts_with("a9059cbb") && data.len() >= 136 {
                    let to_hex = &data[32..72];
                    let amount_hex = &data[72..136];
                    if let Ok(to_addr) = hex_to_tron_address(to_hex) {
                        println!("To:     {to_addr}");
                    }
                    if let Ok(amount) = u128::from_str_radix(amount_hex.trim_start_matches('0'), 16) {
                        // Assume 6 decimals (USDT/USDC).
                        let human = amount as f64 / 1_000_000.0;
                        println!("Amount: {human} (raw: {amount})");
                    }
                } else {
                    let preview = if data.len() > 20 { &data[..20] } else { data };
                    println!("Data:   {preview}...");
                }
            }
        }
        _ => {
            let s = serde_json::to_string_pretty(val).unwrap_or_default();
            if s.len() <= 500 {
                println!("{s}");
            }
        }
    }
}

fn format_timestamp(ms: i64) -> String {
    let secs = ms / 1000;
    let dt = chrono::DateTime::from_timestamp(secs, 0);
    match dt {
        Some(d) => d.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        None => format!("{ms}"),
    }
}

/// Convert hex address (41-prefixed) to base58 Tron address. Falls back to raw.
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

/// Try to decode hex-encoded error message to UTF-8.
fn hex_to_string(hex_str: &str) -> String {
    match hex::decode(hex_str) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => hex_str.to_string(),
    }
}

/// Convert 20-byte hex (no 41 prefix) to base58 Tron address.
fn hex_to_tron_address(hex20: &str) -> Result<String, ()> {
    let clean = hex20.trim_start_matches('0');
    // Pad to 40 hex chars (20 bytes).
    let padded = format!("{:0>40}", clean);
    let mut bytes = vec![0x41u8];
    let addr_bytes = hex::decode(&padded).map_err(|_| ())?;
    bytes.extend_from_slice(&addr_bytes);
    Ok(bs58::encode(&bytes).with_check().into_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── to_base58 ──

    #[test]
    fn to_base58_valid_41_prefix() {
        let hex = "41a614f803b6fd780986a42c78ec9c7f77e6ded13c";
        let result = to_base58(hex);
        assert!(result.starts_with('T'));
        assert_eq!(result.len(), 34);
    }

    #[test]
    fn to_base58_passthrough_base58() {
        let addr = "TNPeeaaFB7K9cmo4uQpcU32zGK8G1NYqeL";
        assert_eq!(to_base58(addr), addr);
    }

    #[test]
    fn to_base58_passthrough_invalid() {
        assert_eq!(to_base58("garbage"), "garbage");
    }

    // ── hex_to_string ──

    #[test]
    fn hex_to_string_valid_utf8() {
        let hex = hex::encode("Hello Tron");
        assert_eq!(hex_to_string(&hex), "Hello Tron");
    }

    #[test]
    fn hex_to_string_invalid_hex() {
        assert_eq!(hex_to_string("zzzz"), "zzzz");
    }

    // ── hex_to_tron_address ──

    #[test]
    fn hex_to_tron_address_valid() {
        // 20 bytes = 40 hex chars
        let hex20 = "a614f803b6fd780986a42c78ec9c7f77e6ded13c";
        let result = hex_to_tron_address(hex20).unwrap();
        assert!(result.starts_with('T'));
        assert_eq!(result.len(), 34);

        // Should match the same address as 41-prefixed
        let full_hex = format!("41{hex20}");
        assert_eq!(result, to_base58(&full_hex));
    }

    #[test]
    fn hex_to_tron_address_with_leading_zeros() {
        // Address with leading zeros in the 20-byte part
        let hex20 = "00000000b6fd780986a42c78ec9c7f77e6ded13c";
        let result = hex_to_tron_address(hex20);
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with('T'));
    }

    #[test]
    fn hex_to_tron_address_invalid_hex() {
        assert!(hex_to_tron_address("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    }

    // ── format_timestamp ──

    #[test]
    fn format_timestamp_known_date() {
        let ts = format_timestamp(1709726787_000);
        assert!(ts.contains("2024-03-06"), "ts: {ts}");
        assert!(ts.ends_with("UTC"));
    }

    #[test]
    fn format_timestamp_epoch() {
        let ts = format_timestamp(0);
        assert_eq!(ts, "1970-01-01 00:00:00 UTC");
    }
}
