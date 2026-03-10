use std::path::Path;

use anyhow::{Context, Result};
use tronic::signer::LocalSigner;
use zeroize::Zeroize;

use crate::wallet_store;

/// Generate a new random wallet, encrypt and save to disk.
pub fn generate(wallet_path: &Path) -> Result<()> {
    check_no_overwrite(wallet_path)?;

    let password = prompt_new_password()?;
    let signer = LocalSigner::rand();
    let mut key = signer.secret_key();

    wallet_store::save(&key, wallet_path, &password)?;
    key.zeroize();

    println!("Wallet created: {}", wallet_path.display());
    println!("Address: {}", signer.address());
    Ok(())
}

/// Import a wallet from a hex-encoded private key (read interactively).
pub fn import(wallet_path: &Path) -> Result<()> {
    check_no_overwrite(wallet_path)?;

    let hex_key = rpassword::prompt_password("Enter private key (hex): ")
        .context("failed to read private key")?;
    let mut bytes = hex::decode(hex_key.trim())
        .context("invalid hex private key")?;
    let mut key: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("private key must be exactly 32 bytes"))?;
    bytes.zeroize();

    // Verify the key is valid by constructing a signer.
    let signer = LocalSigner::from_bytes(&key)?;

    let password = prompt_new_password()?;
    wallet_store::save(&key, wallet_path, &password)?;
    key.zeroize();

    println!("Wallet imported: {}", wallet_path.display());
    println!("Address: {}", signer.address());
    Ok(())
}

/// Export the private key in hex.
pub fn export(wallet_path: &Path) -> Result<()> {
    eprintln!("WARNING: private key will be printed to stdout. Ensure no one can see your screen.");
    let password = prompt_password("Enter wallet password: ")?;
    let mut key = wallet_store::load(wallet_path, &password)?;

    // Verify before displaying.
    let signer = LocalSigner::from_bytes(&key)?;
    println!("Address: {}", signer.address());
    println!("Private key: {}", hex::encode(key));
    key.zeroize();
    Ok(())
}

fn check_no_overwrite(path: &Path) -> Result<()> {
    if path.exists() {
        anyhow::bail!(
            "wallet already exists at {}. Remove it first or use a different --key-file.",
            path.display()
        );
    }
    Ok(())
}

fn prompt_password(msg: &str) -> Result<String> {
    let pw = rpassword::prompt_password(msg)
        .context("failed to read password")?;
    if pw.is_empty() {
        anyhow::bail!("password cannot be empty");
    }
    Ok(pw)
}

fn prompt_new_password() -> Result<String> {
    let pw = prompt_password("Enter new password: ")?;
    let confirm = prompt_password("Confirm password: ")?;
    if pw != confirm {
        anyhow::bail!("passwords do not match");
    }
    Ok(pw)
}
