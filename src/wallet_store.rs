use std::fs;
use std::io::Write;
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result};
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::error::AppError;

const ARGON2_MEM_COST: u32 = 19 * 1024; // 19 MiB
const ARGON2_TIME_COST: u32 = 2;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

#[derive(Serialize, Deserialize)]
struct WalletFile {
    version: u32,
    salt: String,
    nonce: String,
    ciphertext: String,
}

/// Encrypt a 32-byte private key and save to `path`.
pub fn save(private_key: &[u8; 32], path: &Path, password: &str) -> Result<()> {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let mut derived = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&derived)
        .map_err(|e| anyhow::anyhow!("cipher init: {e}"))?;
    derived.zeroize();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, private_key.as_ref())
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;

    let file = WalletFile {
        version: 1,
        salt: hex::encode(salt),
        nonce: hex::encode(nonce_bytes),
        ciphertext: hex::encode(ciphertext),
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&file)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .with_context(|| format!("writing wallet to {}", path.display()))?;
        f.write_all(json.as_bytes())
            .with_context(|| format!("writing wallet to {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, json)
            .with_context(|| format!("writing wallet to {}", path.display()))?;
    }

    Ok(())
}

/// Load and decrypt a private key from `path`.
pub fn load(path: &Path, password: &str) -> Result<[u8; 32]> {
    if !path.exists() {
        return Err(AppError::WalletNotFound {
            path: path.to_path_buf(),
        }
        .into());
    }

    let data = fs::read_to_string(path)
        .with_context(|| format!("reading wallet from {}", path.display()))?;
    let file: WalletFile = serde_json::from_str(&data)
        .with_context(|| "parsing wallet file")?;

    if file.version != 1 {
        anyhow::bail!(
            "unsupported wallet version {} (expected 1)",
            file.version
        );
    }

    let salt = hex::decode(&file.salt).context("decoding salt")?;
    let nonce_bytes = hex::decode(&file.nonce).context("decoding nonce")?;
    let ciphertext = hex::decode(&file.ciphertext).context("decoding ciphertext")?;

    let mut derived = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&derived)
        .map_err(|e| anyhow::anyhow!("cipher init: {e}"))?;
    derived.zeroize();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| AppError::DecryptionFailed)?;

    let key: [u8; 32] = plaintext
        .as_slice()
        .try_into()
        .map_err(|_| AppError::DecryptionFailed)?;
    plaintext.zeroize();
    Ok(key)
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let params = argon2::Params::new(ARGON2_MEM_COST, ARGON2_TIME_COST, 1, Some(32))
        .map_err(|e| anyhow::anyhow!("argon2 params: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow::anyhow!("argon2 hash: {e}"))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_wallet_path() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tron-cli-test-{}", rand::random::<u64>()));
        dir.join("wallet.enc")
    }

    #[test]
    fn round_trip_save_load() {
        let path = tmp_wallet_path();
        let key: [u8; 32] = [42u8; 32];
        let password = "test-password-123";

        save(&key, &path, password).unwrap();
        assert!(path.exists());

        let loaded = load(&path, password).unwrap();
        assert_eq!(key, loaded);

        // Cleanup.
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn wrong_password_fails() {
        let path = tmp_wallet_path();
        let key: [u8; 32] = [7u8; 32];

        save(&key, &path, "correct").unwrap();
        let result = load(&path, "wrong");
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_file_fails() {
        let path = PathBuf::from("/tmp/nonexistent-tron-wallet-xyz/wallet.enc");
        let result = load(&path, "any");
        assert!(result.is_err());
    }

    #[test]
    fn wallet_file_is_valid_json() {
        let path = tmp_wallet_path();
        let key: [u8; 32] = [1u8; 32];

        save(&key, &path, "pw").unwrap();
        let data = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&data).unwrap();

        assert_eq!(parsed["version"], 1);
        assert!(parsed["salt"].is_string());
        assert!(parsed["nonce"].is_string());
        assert!(parsed["ciphertext"].is_string());

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn derive_key_deterministic() {
        let salt = [0u8; 16];
        let k1 = derive_key("password", &salt).unwrap();
        let k2 = derive_key("password", &salt).unwrap();
        assert_eq!(k1, k2);

        let k3 = derive_key("other", &salt).unwrap();
        assert_ne!(k1, k3);
    }
}
