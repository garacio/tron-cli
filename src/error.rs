use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("wallet not found at {path}")]
    WalletNotFound { path: PathBuf },

    #[error("decryption failed — wrong password or corrupted file")]
    DecryptionFailed,

    #[error("invalid Tron address: {0}")]
    InvalidAddress(String),

    #[error("insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: String, need: String },

    #[error("transaction failed: {0}")]
    TransactionFailed(String),
}

use tronic::domain::address::TronAddress;

/// Parse a Tron address string, converting eyre::Report to anyhow::Error.
pub fn parse_address(s: &str) -> anyhow::Result<TronAddress> {
    validate_address(s)?;
    s.parse()
        .map_err(|e: eyre::Report| anyhow::anyhow!("{e:#}"))
}

/// Check if a string looks like a transaction hash (64 hex chars).
pub fn is_txid(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Validate that a string looks like a Tron address, not a txid or random hex.
pub fn validate_address(s: &str) -> anyhow::Result<()> {
    if s.starts_with('T') && s.len() == 34 {
        return Ok(());
    }
    // Could be a hex-encoded txid (64 hex chars).
    if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        anyhow::bail!(
            "'{s}' looks like a transaction hash, not an address. \
             Did you mean: tron-cli tx {s}"
        );
    }
    anyhow::bail!(
        "invalid Tron address: '{s}'. Tron addresses start with 'T' and are 34 characters long"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_txid_valid() {
        let txid = "0a767ff37ca54efc659199e75c6cb2c197294ff4dadb5f5f72b076f9d20fc4a7";
        assert!(is_txid(txid));
    }

    #[test]
    fn is_txid_too_short() {
        assert!(!is_txid("0a767ff37ca54efc"));
    }

    #[test]
    fn is_txid_not_hex() {
        let s = "zz767ff37ca54efc659199e75c6cb2c197294ff4dadb5f5f72b076f9d20fc4a7";
        assert!(!is_txid(s));
    }

    #[test]
    fn is_txid_tron_address() {
        assert!(!is_txid("TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr"));
    }

    #[test]
    fn validate_address_valid() {
        assert!(validate_address("TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr").is_ok());
    }

    #[test]
    fn validate_address_txid_suggests_tx_command() {
        let txid = "0a767ff37ca54efc659199e75c6cb2c197294ff4dadb5f5f72b076f9d20fc4a7";
        let err = validate_address(txid).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("transaction hash"), "msg: {msg}");
        assert!(msg.contains("tron-cli tx"), "msg: {msg}");
    }

    #[test]
    fn validate_address_garbage() {
        assert!(validate_address("hello").is_err());
        assert!(validate_address("").is_err());
        assert!(validate_address("T123").is_err());
    }
}
