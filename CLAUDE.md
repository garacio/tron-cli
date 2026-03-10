# tron-cli

Minimal CLI for Tron blockchain: TRX/USDT balance, transfers, wallet management, transaction history.

## Build & Run

```bash
cargo build --release
cargo run -- --help
```

## Tests

```bash
# Unit tests (59+, no network needed)
cargo test --lib

# Integration tests on Nile testnet (requires .env with TRON_TEST_KEY)
cargo test --test testnet -- --ignored
```

## Environment (.env)

All keys and tokens live in `.env` (gitignored). Required vars:
- `TRONGRID_API_KEY` — TronGrid API key (for mainnet rate limits)
- `TRON_TEST_KEY` — hex private key funded on Nile (for integration tests)
- `TRON_PRIVATE_KEY` — hex private key (runtime alternative to wallet file, for CI/scripts)

Integration tests load `.env` automatically via `dotenvy`.

## Architecture

- **Rust 2021**, async with tokio
- **tronic v0.5** — gRPC client for Tron (balance, transfers). Requires `tonic-tls` feature.
- **TronGrid REST API** (reqwest) — transaction history, tx info. tronic has no history methods.
- **clap v4** (derive) — CLI parsing
- **anyhow/thiserror** — error handling
- **AES-256-GCM + Argon2id** — wallet encryption (`~/.tron-cli/wallet.enc`)

## File Structure

```
src/
  main.rs           — entry point, clap dispatch, load_signer_required()
  lib.rs            — pub re-exports for integration tests
  cli.rs            — Clap structs: Cli, Command, WalletCmd, Token
  client.rs         — tronic gRPC client factory (with optional API key auth)
  config.rs         — Network enum, gRPC/REST endpoints, USDT contract addresses
  error.rs          — AppError, validate_address(), is_txid(), parse_address()
  trongrid.rs       — TronGrid REST client + response types
  wallet_store.rs   — save/load encrypted wallet (Argon2id + AES-256-GCM)
  commands/
    balance.rs      — TRX/USDT balance (single or all non-zero)
    transfer.rs     — TRX/USDT transfer with confirmation prompt
    wallet.rs       — generate, import, export (requires TTY for password)
    history.rs      — tx history with compact/wide modes, hex-to-base58 conversion
    tx.rs           — full tx info, TRC20 data decoding
tests/
  testnet.rs        — integration tests against Nile testnet
examples/
  gen_test_key.rs   — generate a test keypair for Nile
```

## Key tronic Gotchas

- `TronAddress::from_str()` returns `eyre::Report`, not std Error — use `parse_address()` helper with `.map_err()`
- `Client` always requires a `PrehashSigner` — use `LocalSigner::rand()` as dummy for read-only queries
- TRC20 methods need `use tronic::contracts::trc20::Trc20Calls` trait import
- `Trx`/`Usdt` Display already includes unit suffix ("TRX"/"USDT") — don't duplicate
- `Usdt::from_decimal()` returns `TokenError` which doesn't impl std Error — wrap with `anyhow!`
- `GrpcProvider::builder().auth()` uses type-state — can't conditionally call, use match branches
- gRPC endpoints use `http://` (not https), port 50051

## Networks

| Network | gRPC | REST API |
|---------|------|----------|
| mainnet | `http://grpc.trongrid.io:50051` | `https://api.trongrid.io` |
| shasta  | `http://grpc.shasta.trongrid.io:50051` | `https://api.shasta.trongrid.io` |
| nile    | `http://grpc.nile.trongrid.io:50051` | `https://nile.trongrid.io` |

## Tron-Specific Notes

- Tron does NOT allow transferring TRX to yourself — tests use random recipient
- TronGrid REST returns hex addresses (41-prefixed) for TRX history, base58 for TRC20
- USDT has 6 decimals on all networks
- Nile faucet: https://nileex.io/join/getJoinPage

## CLI Commands

```
tron-cli balance [trx|usdt]           # omit token to show all non-zero
tron-cli transfer [trx|usdt] --to ADDR --amount N [--yes]
tron-cli history [trx|usdt] [--address ADDR] [-n LIMIT] [-w]
tron-cli tx TXID
tron-cli recv                          # show your address (alias for wallet show)
tron-cli wallet generate|import|show|export
```
