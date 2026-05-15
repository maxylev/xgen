# xgen

**Multi-chain HD Wallet CLI** — Generate xprv/xpub, private keys and addresses for 8 blockchains from a single BIP39 mnemonic.

[![CI](https://github.com/maxylev/xgen/actions/workflows/ci.yml/badge.svg)](https://github.com/maxylev/xgen/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/xgen.svg)](https://crates.io/crates/xgen)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Supports: **EVM, Bitcoin, Solana, TON, Dogecoin, XRP, Cardano, Monero**

---

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Commands](#commands)
- [Supported Chains](#supported-chains)
- [Exchange Workflow](#exchange-workflow)
- [Solana Modes](#solana-modes)
- [Encryption](#encryption)
- [Development](#development)
- [Security](#security)

---

## Quick Start

```bash
# Generate a new EVM wallet (default: 1 address, account 0)
xgen gen

# Generate 5 TON addresses with QR codes
xgen gen --chain ton --num 5 --qr

# Import mnemonic, get specific Bitcoin address
xgen gen --mnemonic "your twelve words here" --chain btc --index 7

# Generate 100 Solana addresses (cold-export mode — recommended)
xgen gen --chain solana --solana-mode cold-export --num 100 --output solana-keys.json
```

---

## Installation

```bash
# From crates.io
cargo install xgen

# From source
git clone https://github.com/maxylev/xgen.git
cd xgen
cargo build --release
sudo cp target/release/xgen /usr/local/bin/
```

---

## Commands

### `xgen gen` — Generate wallets and addresses

#### Basic options

| Option | Description | Default |
|--------|-------------|---------|
| `--chain` | Target chain: `evm`, `btc`, `solana`, `ton`, `doge`, `xrp`, `cardano`, `monero` | `evm` |
| `--mnemonic` | Import existing BIP39 mnemonic | Generate new |
| `--passphrase` / `-s` | BIP39 passphrase | empty |
| `--strength` | Mnemonic strength: 12 or 24 words | `12` |
| `--json` | Output JSON instead of terminal display | |
| `--output` / `-o` | Save output to file | stdout |
| `--qr` | Show QR code for each address | |
| `--encrypt` | Encrypt JSON output with password | |
| `--password` | Provide encryption/decryption password non-interactively | |

#### Derivation options

| Option | Description | Default |
|--------|-------------|---------|
| `--index` | Derive a single specific address index | — |
| `--indexes` | Comma-separated list of indices (e.g. `0,5,10,42`). Overrides `--index` and `--num` | — |
| `--num` | Number of addresses to generate sequentially | `1` |
| `--account` | Account index for BIP44 derivation | `0` |
| `--change` | Change index | `0` |
| `--hw-sim` | Hardware wallet compatible derivation paths | |

#### Watch-only / xpub options (secp256k1 chains only)

| Option | Description |
|--------|-------------|
| `--xpub` | Generate addresses from BIP32 xpub (no private key needed) |
| `--xpub-path` | Base derivation path for xpub (default: account path) |

#### Solana-specific options

| Option | Description | Default |
|--------|-------------|---------|
| `--solana-mode` | Solana security mode: `full`, `cold-export`, `hsm-sim`, `pda` | `full` |
| `--program-id` | Program ID for PDA mode (base58) | Token Program |

### `xgen decrypt` — Decrypt encrypted wallets

| Option | Description |
|--------|-------------|
| `<file>` | Encrypted wallet file (required) |
| `--output` | Save decrypted wallet to file |
| `--password` | Provide password non-interactively |

---

## Supported Chains

| Chain | Curve | Coin Type | BIP44 Path | Address Format | xpub watch-only |
|-------|-------|:---------:|------------|----------------|:---------------:|
| EVM | secp256k1 | 60 | `m/44'/60'/{account}'/{change}/{index}` | `0x...` checksummed | ✅ |
| Bitcoin | secp256k1 | 0 | `m/44'/0'/{account}'/{change}/{index}` | `1...` P2PKH | ✅ |
| Dogecoin | secp256k1 | 3 | `m/44'/3'/{account}'/{change}/{index}` | `D...` | ✅ |
| Solana | Ed25519 | 501 | `m/44'/501'/{account}'/{change}'` | Base58 | ❌ (hardened) |
| TON | Ed25519 | 607 | `m/44'/607'/{account}'/{change}'` | `EQ...` | ❌ (hardened) |
| XRP | Ed25519 | 144 | `m/44'/144'/{account}'/{index}` | `r...` | ❌ (hardened) |
| Cardano | Ed25519 | 1815 | `m/1852'/1815'/{account}'/0/{index}` | `addr1...` | ❌ (hardened) |
| Monero | Ed25519 | 128 | `m/44'/128'/{account}'/0/{index}` | `4...` | ❌ (hardened) |

---

## Exchange Workflow

### Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                     CRYPTO EXCHANGE SETUP                         │
├──────────────────────────┬───────────────────────────────────────┤
│  COLD WALLET (offline)  │  HOT SERVER (online)                   │
│                          │                                        │
│  mnemonic phrase         │  xpub (BIP32 extended public key)      │
│      │                   │      │                                 │
│      ▼                   │      ▼                                 │
│  master_xprv             │  derive_pub(0) → User A deposit addr   │
│      │                   │  derive_pub(1) → User B deposit addr   │
│      ▼                   │  derive_pub(2) → User C deposit addr   │
│  account xpub ──────────►│  ...                                    │
│  (hardened path)         │                                        │
│                          │  No private keys on hot server         │
│  sign withdraw tx ◄──────┤  Cold wallet signs on request          │
│  with private_key[i]     │                                        │
└──────────────────────────┴───────────────────────────────────────┘
```

### Full Cycle (verified on Anvil + bitcoind regtest + Solana validator)

#### EVM / Bitcoin (xpub chains)

```bash
# === PHASE 1: Cold Wallet — Generate account xpub ===
xgen gen --chain evm --mnemonic "your phrase" --index 0 --json
# master_xpub: xpub6DCoCpSuQZB2...

# === PHASE 2: Hot Server — Generate deposit addresses (watch-only) ===
xgen gen --xpub "xpub6DCoCpSuQZB2..." --num 10000 --chain evm --json
xgen gen --xpub "xpub6DCoCpSuQZB2..." --index 42 --chain evm --json

# === PHASE 3: Sweep — Cold wallet signs withdrawal ===
for i in {0..9}; do
  PRIV=$(xgen gen --chain evm --mnemonic "your phrase" --index $i --json | jq -r '.keys[0].private_key')
  cast send --private-key "${PRIV#0x}" --value "$BAL" "$HOT_WALLET"
done
```

#### Solana (Ed25519 — no xpub support)

```bash
# === Option A: Cold export (recommended) ===
xgen gen --chain solana --mnemonic "your phrase" --solana-mode cold-export --num 10000 --json

# === Option B: Dynamic derivation from seed ===
xgen gen --chain solana --mnemonic "your phrase" --account 1 --index 0   # user 1
xgen gen --chain solana --mnemonic "your phrase" --account 2 --index 0   # user 2

# === Option C: PDA addresses (receive-only) ===
xgen gen --chain solana --solana-mode pda --mnemonic "your phrase" --num 100 --json
```

---

## Solana Modes

Solana uses Ed25519 with **hardened derivation** — all child indices require the private key. xgen provides 4 security modes:

| Mode | Private key exposed? | Can sweep? | Use case |
|------|:-------------------:|:----------:|----------|
| `full` | ✅ Visible | ✅ Yes | Testing, small amounts on hot server |
| `hsm-sim` | ✅ Visible (simulated HSM) | ✅ Yes | When using HSM/secure enclave |
| `cold-export` | ❌ `HIDDEN_FOR_SECURITY` | ✅ Yes (with xgen priv key) | **🔒 Recommended for exchanges** |
| `pda` | ❌ `PDA_CONTROLLED_BY_PROGRAM` | ❌ No (needs on-chain program) | Receive-only monitoring |

### PDA Mode

Program Derived Addresses are controlled by a Solana program, not by a private key:

```bash
# PDA with default program (Token Program)
xgen gen --chain solana --solana-mode pda --index 0
# → 3zJqUDFX2mJvcXUBpCkhM18E3TdvWhR6HJ4uMbXHXR8N

# PDA with custom program ID
xgen gen --chain solana --solana-mode pda --index 42 --program-id "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
```

**PDA addresses can receive SOL** but cannot sweep without a deployed program with `invoke_signed`. Use PDA mode for monitoring-only deposit addresses.

### Solana: How to Sweep

1. Generate address with `full` or `cold-export` mode
2. Extract private key from JSON output
3. Create a [priv(32) + pub(32)] keypair file
4. Use `solana transfer --keypair keypair.json` to sweep

```bash
# Generate keypair from xgen output
PRIV=$(xgen gen --chain solana --mnemonic "..." --index 0 --json | jq -r '.keys[0].private_key')
PUB=$(xgen gen --chain solana --mnemonic "..." --index 0 --json | jq -r '.keys[0].public_key')
python3 -c "import json; priv=bytes.fromhex('$PRIV'); pub=bytes.fromhex('$PUB'); json.dump(list(priv)+list(pub),open('key.json','w'))"

# Sweep to hot wallet
solana transfer --keypair key.json --allow-unfunded-recipient "$HOT_WALLET" ALL
```

---

## Encryption

```bash
# Encrypt wallet output
xgen gen --chain ton --encrypt "mypassword" --output wallet.enc

# Decrypt
xgen decrypt wallet.enc
xgen decrypt wallet.enc --output wallet.json
xgen decrypt wallet.enc --password "mypassword"   # non-interactive
```

Uses AES-256-GCM with scrypt key derivation (N=2^15, r=8, p=1).

---

## Batch Derivation

```bash
# Generate specific indices (comma-separated)
xgen gen --chain evm --indexes "0,5,10,42"
xgen gen --chain solana --solana-mode cold-export --indexes "100,200,300"
xgen gen --chain btc --xpub "xpub6..." --indexes "1000,2000,3000"

# Sequential range (consecutive)
xgen gen --chain evm --num 100
```

---

## Development

```bash
cargo build
cargo test          # 68 integration tests
cargo clippy -- -D warnings
cargo build --release
```

### Local blockchain testing

```bash
# Start local nodes
anvil --silent &                                    # EVM
solana-test-validator --reset &                     # Solana
bitcoind -regtest -daemon -fallbackfee=0.0001 &     # Bitcoin

# Run exchange E2E (generates 10 users, deposits, sweeps)
bash tests/e2e-exchange.sh
```

### Project structure

```
xgen/
├── src/main.rs              # 950+ lines of Rust
├── tests/integration.rs     # 1200+ lines, 68 tests
├── Cargo.toml               # 40+ deps at latest versions
├── .github/workflows/
│   ├── ci.yml               # fmt + clippy + test on push/PR
│   └── publish.yml          # Publish to crates.io on release
├── README.md
└── LICENSE
```

---

## Security

- **Never share your mnemonic or private keys**
- Use `--encrypt` when saving to disk
- Generate addresses offline when possible
- For Solana, prefer `cold-export` or `pda` mode on hot servers
- Hardware wallets are always safer for production use
- The `full` mode on Solana exposes private keys on the hot server

---

## License

MIT — see [LICENSE](LICENSE)
