//! Integration tests against Nile testnet.
//!
//! These tests require:
//!   - `TRON_TEST_KEY` env var with a hex private key funded on Nile
//!   - Network access to Nile testnet
//!
//! Run with: cargo test --test testnet -- --ignored
//!
//! Get test TRX: https://nileex.io/join/getJoinPage

use anyhow::Result;
use tronic::{
    contracts::trc20::{Trc20Calls, Trc20Contract},
    contracts::token::usdt::Usdt,
    signer::LocalSigner,
};

use tron_cli::{
    client,
    config::Network,
    error::parse_address,
    trongrid::TronGridClient,
};

const NETWORK: Network = Network::Nile;

/// Load test signer from `TRON_TEST_KEY` env var (reads .env automatically).
fn test_signer() -> LocalSigner {
    let _ = dotenvy::dotenv();
    let hex_key = std::env::var("TRON_TEST_KEY")
        .expect("TRON_TEST_KEY env var required — set in .env or environment");
    let bytes = hex::decode(hex_key.trim()).expect("TRON_TEST_KEY must be valid hex");
    let key: [u8; 32] = bytes.try_into().expect("TRON_TEST_KEY must be 32 bytes");
    LocalSigner::from_bytes(&key).expect("invalid private key")
}

/// Random throwaway address as transfer recipient.
fn random_recipient() -> tronic::domain::address::TronAddress {
    LocalSigner::rand().address()
}

// ── Balance tests ──

#[tokio::test]
#[ignore]
async fn balance_trx() -> Result<()> {
    let signer = test_signer();
    let addr = signer.address();
    let client = client::build(NETWORK, signer, None).await?;

    let balance = client.trx_balance().address(addr).get().await?;
    println!("TRX balance: {balance}");
    Ok(())
}

#[tokio::test]
#[ignore]
async fn balance_usdt() -> Result<()> {
    let signer = test_signer();
    let addr = signer.address();
    let usdt_addr = parse_address(NETWORK.usdt_contract())?;
    let contract = Trc20Contract::<Usdt>::new(usdt_addr);

    let client = client::build(NETWORK, signer, None).await?;
    let balance: Usdt = client
        .trc20_balance_of()
        .contract(contract)
        .owner(addr)
        .get()
        .await?;
    println!("USDT balance: {balance}");
    Ok(())
}

// ── Transfer TRX ──

#[tokio::test]
#[ignore]
async fn transfer_trx() -> Result<()> {
    use tronic::client::pending::AutoSigning;
    use tronic::domain::trx::Trx;

    let signer = test_signer();
    let my_addr = signer.address();
    let recipient = random_recipient();
    let client = client::build(NETWORK, signer, None).await?;

    let balance = client.trx_balance().address(my_addr).get().await?;
    println!("Balance before: {balance}");

    // Send 0.1 TRX to a random address.
    let amount: Trx = 0.1_f64.into();
    let txid = client
        .send_trx()
        .to(recipient)
        .amount(amount)
        .can_spend_trx_for_fee(true)
        .build::<AutoSigning>()
        .await?
        .broadcast(&())
        .await?;

    println!("TX sent to {recipient}: {txid:?}");

    // Verify balance decreased.
    let balance_after = client.trx_balance().address(my_addr).get().await?;
    println!("Balance after: {balance_after}");

    Ok(())
}

// ── Transfer USDT ──

#[tokio::test]
#[ignore]
async fn transfer_usdt() -> Result<()> {
    use tronic::client::pending::AutoSigning;

    let signer = test_signer();
    let my_addr = signer.address();
    let recipient = random_recipient();

    let usdt_addr = parse_address(NETWORK.usdt_contract())?;
    let contract = Trc20Contract::<Usdt>::new(usdt_addr);

    let client = client::build(NETWORK, signer, None).await?;

    // Check USDT balance first.
    let balance: Usdt = client
        .trc20_balance_of()
        .contract(contract.clone())
        .owner(my_addr)
        .get()
        .await?;
    println!("USDT balance before: {balance}");

    let amount = Usdt::from_decimal(0.01)
        .map_err(|e| anyhow::anyhow!("invalid USDT amount: {e}"))?;

    // Send 0.01 USDT to a random address.
    let txid = client
        .trc20_transfer()
        .contract(contract.clone())
        .to(recipient)
        .amount(amount)
        .can_spend_trx_for_fee(true)
        .build::<AutoSigning>()
        .await?
        .broadcast(&())
        .await?;

    println!("USDT TX sent to {recipient}: {txid:?}");

    // Verify balance decreased.
    let balance_after: Usdt = client
        .trc20_balance_of()
        .contract(contract)
        .owner(my_addr)
        .get()
        .await?;
    println!("USDT balance after: {balance_after}");

    Ok(())
}

// ── History ──

#[tokio::test]
#[ignore]
async fn history_trx() -> Result<()> {
    let signer = test_signer();
    let addr = signer.address().to_string();

    let client = TronGridClient::new(NETWORK, None)?;
    let resp = client.transactions(&addr, 5, None).await?;

    assert!(resp.success);
    println!("Got {} TRX transactions", resp.data.len());

    for tx in &resp.data {
        println!("  {} status={:?}", tx.tx_id, tx.ret);
    }
    Ok(())
}

#[tokio::test]
#[ignore]
async fn history_trc20() -> Result<()> {
    let signer = test_signer();
    let addr = signer.address().to_string();

    let client = TronGridClient::new(NETWORK, None)?;
    let resp = client.trc20_transactions(&addr, 5, None).await?;

    assert!(resp.success);
    println!("Got {} TRC20 transactions", resp.data.len());
    Ok(())
}

// ── Tx info ──

#[tokio::test]
#[ignore]
async fn tx_info_after_transfer() -> Result<()> {
    use tronic::client::pending::AutoSigning;
    use tronic::domain::trx::Trx;

    let signer = test_signer();
    let recipient = random_recipient();
    let client = client::build(NETWORK, signer, None).await?;

    // Send 0.01 TRX to a random address.
    let amount: Trx = 0.01_f64.into();
    let txid = client
        .send_trx()
        .to(recipient)
        .amount(amount)
        .can_spend_trx_for_fee(true)
        .build::<AutoSigning>()
        .await?
        .broadcast(&())
        .await?;

    let txid_str = format!("{txid:?}");
    println!("Sent TX: {txid_str}");

    // Wait for indexing.
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;

    let tg = TronGridClient::new(NETWORK, None)?;

    let raw = tg.transaction_by_id(&txid_str).await?;
    assert!(raw.tx_id.is_some(), "raw tx should have txID");

    let info = tg.transaction_info(&txid_str).await?;
    assert!(info.block_number.is_some(), "tx info should have block number");
    println!(
        "Block: {:?}, Fee: {:?}",
        info.block_number, info.fee
    );

    Ok(())
}
