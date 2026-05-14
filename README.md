# xgen

**Multi-chain HD Wallet CLI** — Generate xprv/xpub, private keys and addresses for 8 blockchains from a single mnemonic.

[![CI](https://github.com/maxylev/xgen/actions/workflows/ci.yml/badge.svg)](https://github.com/maxylev/xgen/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/xgen.svg)](https://crates.io/crates/xgen)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Supports: **EVM (Ethereum), Bitcoin, Solana, TON, Dogecoin, XRP, Cardano, Monero**

---

## Features

- Generate new BIP39 mnemonic (12 or 24 words) or import existing one
- Derive addresses for **8 blockchains** from the same seed
- Specific index derivation (`--index`) or batch (`--num`)
- Hardware wallet simulation (`--hw-sim`)
- Encrypted JSON export (`--encrypt`) with AES-256-GCM + scrypt
- Decrypt command
- QR codes for addresses
- Colored terminal output
- JSON output for scripting

## Installation

### From crates.io

```bash
cargo install xgen
```

### From source

```bash
git clone https://github.com/maxylev/xgen.git
cd xgen
cargo build --release
sudo cp target/release/xgen /usr/local/bin/
```

## Usage

### Generate a new wallet

```bash
# Default: EVM, 1 address, account 0
xgen gen

# TON blockchain
xgen gen --chain ton --num 5 --qr

# Bitcoin with WIF private keys
xgen gen --chain btc --num 3

# 24-word mnemonic
xgen gen --strength 24
```

### Import existing mnemonic

```bash
xgen gen --mnemonic "witch collapse practice feed shame open despair creek road again ice least" --chain evm --index 7
```

### Specific derivation

```bash
# Specific index (single address)
xgen gen --chain solana --index 42

# Hardware wallet style derivation
xgen gen --chain solana --hw-sim --account 1

# Custom account and change
xgen gen --chain evm --account 2 --change 1 --num 3
```

### Encrypted output

```bash
# Encrypt and save to file
xgen gen --chain ton --encrypt "mystrongpassword" --output myton.json

# Decrypt
xgen decrypt myton.json

# Decrypt to file
xgen decrypt myton.json --output decrypted.json

# Decrypt with password flag (non-interactive)
xgen decrypt myton.json --password "mystrongpassword"
```

### JSON output (for scripting)

```bash
xgen gen --chain evm --mnemonic "your twelve words here" --index 0 --json

# Save to file
xgen gen --chain btc --num 5 --json --output wallet.json
```

## Options

### `gen` command

| Option              | Description |
|---------------------|-------------|
| `--chain`           | Target chain: `evm`, `btc`, `solana`, `ton`, `doge`, `xrp`, `cardano`, `monero` (default: `evm`) |
| `--mnemonic`        | Import existing BIP39 mnemonic |
| `--passphrase`      | BIP39 passphrase (default: empty) |
| `--index`           | Derive a single specific index |
| `--num`             | Number of addresses to derive (default: 1) |
| `--account`         | Account index for BIP44 derivation (default: 0) |
| `--change`          | Change index (default: 0) |
| `--strength`        | Mnemonic strength: 12 or 24 words (default: 12) |
| `--hw-sim`          | Use hardware wallet compatible derivation paths |
| `--qr`              | Show QR code for each address |
| `--encrypt <pass>`  | Encrypt output JSON with password |
| `--password <pass>` | Provide password non-interactively |
| `--json`            | Output JSON instead of colored table |
| `--output <file>`   | Save output to file |
| `-s` / `--passphrase` | BIP39 passphrase |
| `-h` / `--help`     | Print help |

### `decrypt` command

| Option              | Description |
|---------------------|-------------|
| `--output <file>`   | Save decrypted wallet to file (default: stdout) |
| `--password <pass>` | Provide password non-interactively |

## Supported Chains

| Chain       | Coin Type | BIP44 Path Example              | Address Format      |
|-------------|-----------|----------------------------------|---------------------|
| EVM         | 60        | `m/44'/60'/0'/0/0`              | `0x...` (checksummed) |
| Bitcoin     | 0         | `m/44'/0'/0'/0/0`               | Legacy `1...`       |
| Solana      | 501       | `m/44'/501'/0'/0'`              | Base58              |
| TON         | 607       | `m/44'/607'/0'/0'`              | `EQ...`             |
| Dogecoin    | 3         | `m/44'/3'/0'/0/0`               | `D...`              |
| XRP         | 144       | `m/44'/144'/0'/0/0`             | `r...`              |
| Cardano     | 1815      | `m/1852'/1815'/0'/0/0`          | `addr1...`          |
| Monero      | 128       | `m/44'/128'/0'/0/0`             | `4...`              |

## Examples

### Deterministic output

The same mnemonic + index always produces the same keys:

```bash
xgen gen --chain evm --mnemonic "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about" --index 0
# Address: 0x83aEa22E17D39c533B3Bc3BbA8737D7B72574EbF
```

### Full wallet backup

```bash
# Generate encrypted backup
xgen gen --chain evm --num 5 --encrypt "strong-password" --output evm_backup.json

# Later, restore
xgen decrypt evm_backup.json --output evm_restored.json
```

### Multi-chain from single mnemonic

```bash
# One mnemonic, multiple chains
MNEMONIC="your twelve words here"

xgen gen --chain evm    --mnemonic "$MNEMONIC" --index 0
xgen gen --chain btc    --mnemonic "$MNEMONIC" --index 0
xgen gen --chain solana --mnemonic "$MNEMONIC" --index 0
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Build release
cargo build --release
```

### Project structure

```
xgen/
├── src/main.rs          # CLI implementation
├── tests/integration.rs # 68 e2e tests
├── Cargo.toml
├── .github/workflows/
│   ├── ci.yml           # CI (build + test on push/PR)
│   └── publish.yml      # Publish to crates.io on release
└── LICENSE
```

### Exchange / Watch-Only Mode (xpub)

Generate **unlimited deposit addresses** from an xpub **without private keys** — the standard approach used by crypto exchanges:

```bash
# Step 1 (COLD WALLET — offline): Generate the account xpub
xgen gen --chain evm --mnemonic "your twelve words here" --index 0 --json
# → master_xpub: xpub6DCoCpSuQZB2ja...

# Step 2 (HOT SERVER): Generate 1000 deposit addresses from xpub (no private keys)
xgen gen --xpub "xpub6DCoCpSuQZB2ja..." --num 1000 --chain evm --json

# Step 3: Generate a specific user's deposit address
xgen gen --xpub "xpub6DCoCpSuQZB2ja..." --index 42 --chain evm

# Step 4 (COLD WALLET — offline): Sign withdrawal with private key
xgen gen --chain evm --mnemonic "your twelve words here" --index 42
# Use the private_key field to sign the withdrawal transaction
```

**Architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│                  CRYPTO EXCHANGE SETUP                       │
├─────────────────────────┬───────────────────────────────────┤
│   COLD WALLET (offline) │   HOT SERVER (online)              │
│                         │                                     │
│   mnemonic phrase       │   xpub (BIP32 extended public key) │
│       │                 │       │                             │
│       ▼                 │       ▼                             │
│   master_xprv           │   derive_pub(0) → User A deposit   │
│       │                 │   derive_pub(1) → User B deposit   │
│       ▼                 │   derive_pub(2) → User C deposit   │
│   account xpub ───────► │   derive_pub(3) → ...              │
│   (m/44'/60'/0')       │       │                             │
│                         │   BALANCE CHECK via RPC            │
│   No network access     │   No private keys on hot server    │
└─────────────────────────┴───────────────────────────────────┘
```

**Verified with real blockchains (Anvil, Solana local validator):**

```bash
# 1. Cold wallet generates account xpub
# 2. Hot server derives 5 deposit addresses from xpub
# 3. User deposits 10 ETH to address[3]
# 4. Cold wallet signs withdrawal of 3 ETH from address[3]
# 5. All addresses MATCH between xpub and private key derivation ✓
```

**Important:** Only non-hardened derivation (`/0`, `/1`, `/2`) works with xpub. Hardened paths (`/0'`, `/1'`) require the private key. The `master_xpub` field in JSON output contains the account-level xpub (`m/44'/60'/0'`).

### Options

| Option              | Description |
|---------------------|-------------|
| `--xpub <string>`   | Generate watch-only addresses from xpub (no private keys needed) |
| `--xpub-path <path>` | Base path for xpub derivation (default: derived from chain defaults) |

## Security Notes

- Never share your mnemonic or private keys
- Use `--encrypt` when saving to disk
- Generate offline when possible
- Hardware wallets are always safer for production use

## License

MIT — see [LICENSE](LICENSE)
