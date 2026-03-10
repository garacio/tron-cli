mod cli;
mod client;
mod commands;
mod config;
mod error;
#[allow(dead_code)]
mod trongrid;
mod wallet_store;

use anyhow::Result;
use clap::Parser;
use tronic::signer::LocalSigner;
use zeroize::Zeroize;

use cli::{Cli, Command, WalletCmd};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let wallet_path = cli.wallet_path();
    let api_key = std::env::var("TRONGRID_API_KEY").ok();

    match cli.command {
        Command::Wallet(cmd) => match cmd {
            WalletCmd::Generate => commands::wallet::generate(&wallet_path)?,
            WalletCmd::Import => commands::wallet::import(&wallet_path)?,
            WalletCmd::Show => show_address(&wallet_path)?,
            WalletCmd::Export => commands::wallet::export(&wallet_path)?,
        },

        Command::Balance { address, token } => {
            if let Some(ref addr) = address {
                if error::is_txid(addr) {
                    commands::tx::run(cli.network, addr, api_key.as_deref()).await?;
                    return Ok(());
                }
            }
            let signer = if address.is_none() {
                Some(load_signer_required(&wallet_path)?)
            } else {
                None
            };
            commands::balance::run(
                cli.network,
                address,
                token,
                signer,
                api_key.as_deref(),
            )
            .await?;
        }

        Command::History {
            token,
            address,
            limit,
            wide,
        } => {
            // Auto-detect: if --address looks like a txid, show tx info instead.
            if let Some(ref addr) = address {
                if error::is_txid(addr) {
                    commands::tx::run(cli.network, addr, api_key.as_deref()).await?;
                    return Ok(());
                }
            }
            let signer = if address.is_none() {
                Some(load_signer_required(&wallet_path)?)
            } else {
                None
            };
            commands::history::run(
                cli.network,
                address,
                token,
                limit,
                wide,
                signer,
                api_key.as_deref(),
            )
            .await?;
        }

        Command::Recv => show_address(&wallet_path)?,

        Command::Tx { txid } => {
            commands::tx::run(cli.network, &txid, api_key.as_deref()).await?;
        }

        Command::Transfer {
            to,
            amount,
            token,
            yes,
        } => {
            let signer = load_signer_required(&wallet_path)?;
            commands::transfer::run(
                cli.network,
                &to,
                &amount,
                token,
                yes,
                signer,
                api_key.as_deref(),
            )
            .await?;
        }
    }

    Ok(())
}

fn show_address(wallet_path: &std::path::Path) -> Result<()> {
    let signer = load_signer_required(wallet_path)?;
    println!("{}", signer.address());
    Ok(())
}

/// Load signer, failing if no key source is available.
fn load_signer_required(wallet_path: &std::path::Path) -> Result<LocalSigner> {
    if let Ok(hex_key) = std::env::var("TRON_PRIVATE_KEY") {
        let mut bytes = hex::decode(hex_key.trim())?;
        let mut key: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("TRON_PRIVATE_KEY must be 32 bytes hex"))?;
        bytes.zeroize();
        let signer = LocalSigner::from_bytes(&key)?;
        key.zeroize();
        return Ok(signer);
    }
    let password = rpassword::prompt_password("Enter wallet password: ")?;
    let mut key = wallet_store::load(wallet_path, &password)?;
    let signer = LocalSigner::from_bytes(&key)?;
    key.zeroize();
    Ok(signer)
}
