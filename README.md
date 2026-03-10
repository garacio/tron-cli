# tron-cli

CLI for interacting with the Tron blockchain: wallet management, balances, transfers, transaction history.

## Installation

```bash
cargo build --release
cp target/release/tron-cli ~/.local/bin/
```

## Global Options

```
tron-cli [--network <mainnet|shasta|nile>] [--key-file <path>] <COMMAND>
```

| Option | Description | Default |
|--------|-------------|---------|
| `--network` | Tron network | `mainnet` |
| `--key-file` | Path to encrypted wallet file | `~/.tron-cli/wallet.enc` |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `TRONGRID_API_KEY` | TronGrid API key (recommended for mainnet) |
| `TRON_PRIVATE_KEY` | Private key in hex (alternative to wallet file, for CI/scripts) |
| `TRON_KEY_FILE` | Wallet path (alternative to `--key-file`) |

---

## Commands

### recv — show your address for receiving transfers

```bash
tron-cli recv
```

Prints the wallet address. Alias for `wallet show`.

---

### balance — check balance

```bash
tron-cli balance [trx|usdt] [--address <ADDR>]
```

If no token is specified, shows all non-zero balances (TRX and USDT).

**Examples:**

```bash
# All non-zero balances of your wallet
tron-cli balance

# TRX only
tron-cli balance trx

# USDT only
tron-cli balance usdt

# Balance of any address (no password required)
tron-cli balance --address <ADDR>

# USDT balance of any address
tron-cli balance usdt --address <ADDR>
```

**Output without specifying token:**
```
1995.38 TRX
999.99 USDT
```

---

### transfer — send TRX/USDT

```bash
tron-cli transfer [trx|usdt] --to <ADDR> --amount <AMOUNT> [--yes]
```

Prompts for confirmation (y/N) before sending. Use `--yes` to skip.

**Examples:**

```bash
# Send 10 TRX
tron-cli transfer --to <ADDR> --amount 10

# Send 100 USDT
tron-cli transfer usdt --to <ADDR> --amount 100

# Skip confirmation (for scripts)
tron-cli transfer --to <ADDR> --amount 5 --yes
```

---

### history — transaction history

```bash
tron-cli history [trx|usdt] [--address <ADDR>] [-n <LIMIT>] [-w]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--address` | Address to query | own wallet |
| `-n`, `--limit` | Number of entries | 20 |
| `-w`, `--wide` | Show full addresses and txids | off |

**Examples:**

```bash
# Last 20 TRX transactions
tron-cli history

# Last 5 USDT transfers
tron-cli history usdt -n 5

# With full addresses and txids
tron-cli history -w

# History of any address (no password required)
tron-cli history --address <ADDR>
```

**Compact output:**
```
DATE              STATUS     DIR               AMOUNT   COUNTERPARTY    TXID
-----------------------------------------------------------------------------------------------
2025-01-15 14:32  SUCCESS     IN      10.000000 TRX    TKbQ2m...9xYz  a1b2c3d4...e5f6
2025-01-15 10:05  SUCCESS    OUT       5.500000 TRX    TJnR4p...3wVq  d7e8f9a0...b1c2
```

**Wide output (`-w`):**
```
DATE              STATUS     DIR               AMOUNT   COUNTERPARTY                        TXID
-------------------------------------------------------------------------------------------------------------
2025-01-15 14:32  SUCCESS     IN      10.000000 TRX    TKbQ2mXxR7pN5sLfYjW8cD4gH6kV9xYz  a1b2c3d4e5f6...
```

---

### tx — transaction details

```bash
tron-cli tx <TXID>
```

Shows full transaction info: block, time, status, operation type, sender, recipient, amount, fee, energy, bandwidth.

**Example:**

```bash
tron-cli tx <TXID>
```

```
TX:     a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
Block:  65443951
Time:   2025-01-15 14:32:07 UTC
Status: SUCCESS
Type:   TransferContract
From:   TKbQ2mXxR7pN5sLfYjW8cD4gH6kV9xYz
To:     TJnR4pQw6tM3vBzX8yK2fA5hN7sL3wVq
Amount: 10 TRX
BW:     268
```

If a txid is passed to `--address`, it will automatically show transaction info instead.

---

### wallet — wallet management

#### Generate a new wallet

```bash
tron-cli wallet generate
```

Generates a random private key, prompts for a password, and saves the encrypted file to `~/.tron-cli/wallet.enc`.

#### Import an existing key

```bash
tron-cli wallet import --private-key <HEX>
```

#### Show address

```bash
tron-cli wallet show
```

Same as `tron-cli recv`.

#### Export private key

```bash
tron-cli wallet export
```

Prints the address and private key in hex. Requires password.

---

## Wallet Encryption

The file `~/.tron-cli/wallet.enc` is a JSON with the encrypted private key:

- **KDF:** Argon2id (19 MiB memory, 2 iterations)
- **Cipher:** AES-256-GCM (12-byte nonce)
- **Format:** `{"version":1, "salt":"<hex>", "nonce":"<hex>", "ciphertext":"<hex>"}`

## Networks

| Network | gRPC endpoint | Purpose |
|---------|---------------|---------|
| `mainnet` | grpc.trongrid.io:50051 | Production |
| `shasta` | grpc.shasta.trongrid.io:50051 | Testnet |
| `nile` | grpc.nile.trongrid.io:50051 | Testnet (recommended for TRC20) |
