use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::Network;

#[derive(Parser)]
#[command(name = "tron-cli", about = "Minimal Tron blockchain CLI")]
pub struct Cli {
    /// Network to connect to
    #[arg(long, value_enum, default_value_t = Network::Mainnet)]
    pub network: Network,

    /// Path to encrypted wallet file
    #[arg(long, env = "TRON_KEY_FILE")]
    pub key_file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    /// Resolve the wallet file path: explicit flag > env > default.
    pub fn wallet_path(&self) -> PathBuf {
        if let Some(ref p) = self.key_file {
            return p.clone();
        }
        dirs_or_default().join("wallet.enc")
    }
}

fn dirs_or_default() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tron-cli")
}

#[derive(Subcommand)]
pub enum Command {
    /// Check TRX or USDT balance (shows all non-zero if token not specified)
    Balance {
        /// Token to query (trx or usdt); omit to show all
        #[arg(value_enum)]
        token: Option<Token>,

        /// Address to check (defaults to own wallet)
        #[arg(long)]
        address: Option<String>,
    },

    /// Transfer TRX or USDT
    Transfer {
        /// Token to transfer (trx or usdt)
        #[arg(value_enum, default_value_t = Token::Trx)]
        token: Token,

        /// Recipient address
        #[arg(long)]
        to: String,

        /// Amount to send (human-readable, e.g. 1.5)
        #[arg(long)]
        amount: String,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Show transaction history
    History {
        /// Token filter (trx or usdt)
        #[arg(value_enum, default_value_t = Token::Trx)]
        token: Token,

        /// Address to query (defaults to own wallet)
        #[arg(long)]
        address: Option<String>,

        /// Number of transactions to show
        #[arg(long, short = 'n', default_value_t = 20)]
        limit: u32,

        /// Show full addresses and txids
        #[arg(long, short = 'w')]
        wide: bool,
    },

    /// Show your wallet address (alias for `wallet show`)
    Recv,

    /// Show transaction details by txid
    Tx {
        /// Transaction hash
        txid: String,
    },

    /// Wallet management
    #[command(subcommand)]
    Wallet(WalletCmd),
}

#[derive(Subcommand)]
pub enum WalletCmd {
    /// Generate a new wallet
    Generate,
    /// Import wallet from hex private key (reads interactively)
    Import,
    /// Show wallet address (same as `recv`)
    Show,
    /// Export private key (hex)
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum Token {
    Trx,
    Usdt,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(args)
    }

    #[test]
    fn balance_no_token_shows_all() {
        let cli = parse(&["tron-cli", "balance"]);
        match cli.command {
            Command::Balance { token, address } => {
                assert!(token.is_none());
                assert!(address.is_none());
            }
            _ => panic!("expected Balance"),
        }
    }

    #[test]
    fn balance_usdt() {
        let cli = parse(&["tron-cli", "balance", "usdt"]);
        match cli.command {
            Command::Balance { token, .. } => assert_eq!(token, Some(Token::Usdt)),
            _ => panic!("expected Balance"),
        }
    }

    #[test]
    fn balance_trx_explicit() {
        let cli = parse(&["tron-cli", "balance", "trx"]);
        match cli.command {
            Command::Balance { token, .. } => assert_eq!(token, Some(Token::Trx)),
            _ => panic!("expected Balance"),
        }
    }

    #[test]
    fn balance_with_address() {
        let cli = parse(&["tron-cli", "balance", "--address", "TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr"]);
        match cli.command {
            Command::Balance { address, .. } => {
                assert_eq!(address.as_deref(), Some("TDvurPRbhTMRaVmSEPfjMDj3rVzif7stgr"));
            }
            _ => panic!("expected Balance"),
        }
    }

    #[test]
    fn transfer_trx() {
        let cli = parse(&["tron-cli", "transfer", "--to", "TNPeeaaFB7K9cmo4uQpcU32zGK8G1NYqeL", "--amount", "10.5"]);
        match cli.command {
            Command::Transfer { token, to, amount, yes } => {
                assert!(matches!(token, Token::Trx));
                assert_eq!(to, "TNPeeaaFB7K9cmo4uQpcU32zGK8G1NYqeL");
                assert_eq!(amount, "10.5");
                assert!(!yes);
            }
            _ => panic!("expected Transfer"),
        }
    }

    #[test]
    fn transfer_usdt_with_yes() {
        let cli = parse(&["tron-cli", "transfer", "usdt", "--to", "TNPeeaaFB7K9cmo4uQpcU32zGK8G1NYqeL", "--amount", "100", "--yes"]);
        match cli.command {
            Command::Transfer { token, yes, .. } => {
                assert!(matches!(token, Token::Usdt));
                assert!(yes);
            }
            _ => panic!("expected Transfer"),
        }
    }

    #[test]
    fn history_defaults() {
        let cli = parse(&["tron-cli", "history"]);
        match cli.command {
            Command::History { token, limit, wide, address } => {
                assert!(matches!(token, Token::Trx));
                assert_eq!(limit, 20);
                assert!(!wide);
                assert!(address.is_none());
            }
            _ => panic!("expected History"),
        }
    }

    #[test]
    fn history_usdt_with_flags() {
        let cli = parse(&["tron-cli", "history", "usdt", "-n", "5", "-w"]);
        match cli.command {
            Command::History { token, limit, wide, .. } => {
                assert!(matches!(token, Token::Usdt));
                assert_eq!(limit, 5);
                assert!(wide);
            }
            _ => panic!("expected History"),
        }
    }

    #[test]
    fn recv_command() {
        let cli = parse(&["tron-cli", "recv"]);
        assert!(matches!(cli.command, Command::Recv));
    }

    #[test]
    fn tx_command() {
        let txid = "0a767ff37ca54efc659199e75c6cb2c197294ff4dadb5f5f72b076f9d20fc4a7";
        let cli = parse(&["tron-cli", "tx", txid]);
        match cli.command {
            Command::Tx { txid: t } => assert_eq!(t, txid),
            _ => panic!("expected Tx"),
        }
    }

    #[test]
    fn wallet_generate() {
        let cli = parse(&["tron-cli", "wallet", "generate"]);
        assert!(matches!(cli.command, Command::Wallet(WalletCmd::Generate)));
    }

    #[test]
    fn wallet_import() {
        let cli = parse(&["tron-cli", "wallet", "import"]);
        assert!(matches!(cli.command, Command::Wallet(WalletCmd::Import)));
    }

    #[test]
    fn network_flag() {
        let cli = parse(&["tron-cli", "--network", "nile", "balance"]);
        assert!(matches!(cli.network, crate::config::Network::Nile));
    }

    #[test]
    fn key_file_flag() {
        let cli = parse(&["tron-cli", "--key-file", "/tmp/my.enc", "balance"]);
        assert_eq!(cli.key_file, Some(std::path::PathBuf::from("/tmp/my.enc")));
        assert_eq!(cli.wallet_path(), std::path::PathBuf::from("/tmp/my.enc"));
    }

    #[test]
    fn default_wallet_path() {
        let cli = parse(&["tron-cli", "balance"]);
        assert!(cli.key_file.is_none());
        let path = cli.wallet_path();
        assert!(path.ends_with("wallet.enc"));
        assert!(path.to_str().unwrap().contains(".tron-cli"));
    }
}
