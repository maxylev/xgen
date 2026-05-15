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

## Exchange Best Practices (per Chain)

Not all chains support xpub watch-only mode. Here is the recommended strategy for each chain:

| Chain | Curve | xpub watch-only? | Best practice for many deposit addresses | Hot server safety |
|-------|-------|:----------------:|------------------------------------------|:-----------------:|
| **EVM** | secp256k1 | ✅ Excellent | xpub at account level (`m/44'/60'/0'`) | 🔒 High |
| **Bitcoin** | secp256k1 | ✅ Excellent | xpub at account level (`m/44'/0'/0'`) | 🔒 High |
| **Dogecoin** | secp256k1 | ✅ Good | xpub at account level (`m/44'/3'/0'`) | 🔒 High |
| **Cardano** | Ed25519 | ⚠️ Limited | Use extended public keys (native support) | 🔒 High |
| **XRP** | Ed25519 | ❌ No | Pre-generate from seed + export pubkeys | 🔒 High |
| **Solana** | Ed25519 | ❌ No | **Pre-generate many keypairs** or use PDAs | 🔒 High |
| **TON** | Ed25519 | ❌ No | Pre-generate from seed + export pubkeys | 🔒 Medium |
| **Monero** | Ed25519 | ❌ No | Use subaddresses (built-in stealth addresses) | 🔒 High |

### For xpub-supported chains (EVM, Bitcoin, Dogecoin):

```bash
# Cold wallet: generate account xpub once
xgen gen --chain evm --mnemonic "your phrase" --index 0 --json
# → master_xpub: xpub6DCoCpSuQZB2...

# Hot server: generate unlimited deposit addresses (no private keys!)
xgen gen --xpub "xpub6DCoCpSuQZB2..." --num 10000 --chain evm --json
xgen gen --xpub "xpub6DCoCpSuQZB2..." --index 42 --chain evm --json
```

### For Ed25519 chains (Solana, TON, XRP, Cardano, Monero):

These chains use **hardened derivation** — xpub-only derivation is mathematically impossible.
You have two options:

**Option A: Pre-generate (most secure — recommended)**

```bash
# Cold wallet: pre-generate 10000 addresses, export JSON (no private keys)
xgen gen --chain solana --mnemonic "your phrase" --num 10000 --json > public-keys.json

# Hot server: use the exported public keys to monitor deposits
# No private keys ever touch the hot server
```

**Option B: Dynamic derivation from seed (common for exchanges)**

Generate addresses on-demand using the `--account` flag (each user = separate account):

```bash
# User 1 → m/44'/501'/0'/0'
xgen gen --chain solana --mnemonic "your phrase" --account 0 --index 0

# User 2 → m/44'/501'/1'/0'  
xgen gen --chain solana --mnemonic "your phrase" --account 1 --index 0

# User 10000 → m/44'/501'/9999'/0'
xgen gen --chain solana --mnemonic "your phrase" --account 9999 --index 0
```

Or with simple indices (simpler, each user = consecutive index):

```bash
# Generate 1000 deposit addresses dynamically
xgen gen --chain solana --mnemonic "your phrase" --num 1000
```

## Real-World Exchange Workflow (EVM + Bitcoin + Solana)

The complete deposit → sweep cycle using xgen, verified on real local nodes. The standard model used by crypto exchanges:

### Architecture

```
COLD WALLET (offline)          HOT SERVER (online)                BLOCKCHAIN
┌─────────────────────┐        ┌──────────────────────────┐       ┌──────────────┐
│ mnemonic (BIP39)    │        │ xpub (watch-only)        │       │              │
│         │           │        │   ├─ User 1 addr         │◄──────┤  DEPOSIT     │
│         ▼           │  xpub  │   ├─ User 2 addr         │        │  10 users    │
│ master_xprv         │───────►│   ├─ User 3 addr         │        │  1 ETH/BTC   │
│   │                 │        │   ├─ ...                 │        │  1 SOL each  │
│   ├─ xpub (acc)  ──►│        │   └─ User N addr         │◄───────┤              │
│   │                 │        │                          │        └──────┬───────┘
│   ▼                 │        │   No private keys        │               │
│ derive_priv(i)      │◄───────┤                          │               │
│ sign sweep tx       │ sign   │   Request sweep          │               │
└─────────────────────┘        └──────────────────────────┘               │
                                                                          │ SWEEP
                                                                          │ (all funds
                                                                          │ to hot wallet)
```

### Full Exchange Cycle (10 users, verified)

```bash
# --- PHASE 1: Cold Wallet — Generate account xpub ---
xgen gen --chain evm --mnemonic "your twelve words here" --index 0 --json
# master_xpub: xpub6DCoCpSuQZB2...

# --- PHASE 2: Hot Server — Generate 10,000 deposit addresses (watch-only) ---
xgen gen --xpub "xpub6DCoCpSuQZB2..." --num 10000 --chain evm --json

# --- PHASE 3: Users deposit funds (monitor blockchain for each address) ---
# Deposit 1 ETH to each of 10 addresses
for i in {0..9}; do
  cast send --value 1ether "$(xgen gen --xpub "$XPUB" --index $i --chain evm --json | jq -r '.keys[0].address')"
done

# --- PHASE 4: Cold Wallet — Sweep all funds to hot wallet ---
HOT="0xRecipient..."
for i in {0..9}; do
  # Get private key from cold wallet
  PRIV=$(xgen gen --chain evm --mnemonic "your twelve words here" --index $i --json | jq -r '.keys[0].private_key')
  # Sign and send all balance
  cast send --private-key "${PRIV#0x}" --value "$(cast balance "$ADDR")" "$HOT"
done
```

### Verified Results (Anvil + bitcoind regtest + Solana validator)

| Step | EVM | Bitcoin | Solana |
|------|-----|---------|--------|
| 1. Generate xpub | `xpub6DCoCpSuQZB2...` | `xpub6BosfCnifzxc...` | hex xpub (reference) |
| 2. Generate 10 addresses | `0xb8fd...` → `0xAAF0f2...` | `13KE6T...` → `1CGZnV...` | `2zhZ9...` → `3ZNcL...` |
| 3. Deposit 1 each | 10 ETH sent ✓ | 10 BTC sent ✓ | 10 SOL airdropped ✓ |
| 4. Sweep all to hot | 10/10 signed with xgen priv key ✓ | WIF import + send ✓ | Keypair signed ✓ |
| 5. Verify | Hot wallet received ~10 ETH | Exchange wallet credited | Hot wallet received SOL |

### Command Reference

```bash
# Generate account xpub (cold wallet)
xgen gen --chain evm --mnemonic "your phrase" --index 0
# → use master_xpub for hot server

# Generate deposit addresses (hot server, watch-only)
xgen gen --xpub "xpub6..." --num 100 --chain evm
xgen gen --xpub "xpub6..." --index 42 --chain evm

# Sweep: get private key for each index (cold wallet)
PRIV=$(xgen gen --chain evm --mnemonic "your phrase" --index 42 --json | jq -r '.keys[0].private_key')
# Sign and broadcast with any tool (cast, ethers, web3.js, etc.)
```

**Solana note**: Ed25519 chains use hardened derivation and cannot derive from xpub alone. Use `--mnemonic` and `--index` for Solana/TON/XRP/Cardano/Monero.

### Options

| Option              | Description |
|---------------------|-------------|
| `--xpub <string>`   | Generate watch-only addresses from xpub (no private keys needed) |
| `--xpub-path <path>` | Base path for xpub derivation (default: derived from chain defaults) |

**Important:** Only non-hardened derivation (`/0`, `/1`, `/2`) works with xpub. Hardened paths (`/0'`, `/1'`) require the private key. The `master_xpub` field in JSON output contains the account-level xpub (`m/44'/60'/0'`).

## Security Notes

- Never share your mnemonic or private keys
- Use `--encrypt` when saving to disk
- Generate offline when possible
- Hardware wallets are always safer for production use

## License

MIT — see [LICENSE](LICENSE)
