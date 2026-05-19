# xgen

**Multi-chain HD Wallet CLI** — Generate xprv/xpub, private keys and addresses for EVM, Bitcoin, and Solana from a single BIP39 mnemonic, with EIP-55 compliant checksums and SLIP-0010 compatible Ed25519 derivation.

[![CI](https://github.com/maxylev/xgen/actions/workflows/ci.yml/badge.svg)](https://github.com/maxylev/xgen/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/xgen.svg)](https://crates.io/crates/xgen)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Supports: **EVM, Bitcoin, Solana** (cryptographically verified)

---

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Commands](#commands)
- [Supported Chains](#supported-chains)
- [Exchange Workflow](#exchange-workflow)
- [Solana Modes](#solana-modes)
- [Encryption](#encryption)
- [Batch Derivation](#batch-derivation)
- [Library Usage](#library-usage)
- [Development](#development)
- [Security](#security)

---

## Quick Start

```bash
# Generate a new EVM wallet (default: 1 address, account 0)
xgen gen

# Generate 5 Solana addresses with QR codes
xgen gen --chain solana --num 5 --qr

# Import mnemonic, get specific Bitcoin address
xgen gen --mnemonic "your twelve words here" --chain btc --index 7

# Generate 100 Solana addresses (cold-export mode — recommended)
xgen gen --chain solana --solana-mode cold-export --num 100 --output solana-keys.json

# Derive child keys from an extended private key (xpriv)
xgen gen --xpriv "xprv9wTYmMFdV23N2TdNG573QoEsfRr..." --chain evm --num 5
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
| `--chain` | Target chain: `evm`, `btc`, `solana` | `evm` |
| `--mnemonic` | Import existing BIP39 mnemonic | Generate new |
| `--passphrase` / `-s` | BIP39 passphrase | empty |
| `--strength` | Mnemonic strength: 12 or 24 words | `12` |
| `--json` | Output JSON instead of terminal display | |
| `--output` / `-o` | Save output to file | stdout |
| `--qr` | Show QR code for each address | |
| `--encrypt` | Encrypt JSON output (use with `--password` or prompt) | |
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

#### xpriv derivation options (all chains)

| Option | Description |
|--------|-------------|
| `--xpriv` | Generate keys from BIP32 xpriv (EVM/BTC) or 64-byte hex (Solana) |
| `--xpriv-path` | Base derivation path for xpriv (default: account path) |

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

| Chain | Curve | Coin Type | BIP44 Path | Address Format | xpub watch-only | xpriv |
|-------|-------|:---------:|------------|----------------|:---------------:|:-----:|
| EVM | secp256k1 | 60 | `m/44'/60'/{account}'/{change}/{index}` | `0x...` EIP-55 checksummed | ✅ | ✅ |
| Bitcoin | secp256k1 | 0 | `m/44'/0'/{account}'/{change}/{index}` | `bc1q...` P2WPKH | ✅ | ✅ |
| Solana | Ed25519 (SLIP-0010) | 501 | `m/44'/501'/{account}'/{change}'` | Base58 | ❌ (hardened) | ✅ |

### Address Generation Correctness

- **EVM:** Uses EIP-55 checksums — computes Keccak256 of the **address hex string** (not the public key) to produce mixed-case checksums compatible with all wallets and exchanges.
- **Bitcoin:** Uses Native SegWit P2WPKH (Bech32) addresses starting with `bc1q` with WIF private key format. Standard compressed public key derivation (BIP32 compliance).
- **Solana:** Uses SLIP-0010 compliant Ed25519 derivation with all-hardened path segments (no modulo order reduction) — compatible with Phantom and Solflare wallets when using the same mnemonic.

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

#### Solana (Ed25519 — xpriv and seed-based derivation)

```bash
# === Option A: Cold export (recommended) ===
xgen gen --chain solana --mnemonic "your phrase" --solana-mode cold-export --num 10000 --json

# === Option B: Dynamic derivation from seed ===
xgen gen --chain solana --mnemonic "your phrase" --account 1 --index 0   # user 1
xgen gen --chain solana --mnemonic "your phrase" --account 2 --index 0   # user 2

# === Option C: xpriv derivation (64-byte hex: key + chain code) ===
xgen gen --chain solana --mnemonic "your phrase" --index 0 --json  # get master_xpub
# Construct 64-byte xpriv: priv_key(32 bytes) + chain_code(32 bytes from master_xpub)
xgen gen --xpriv "<64-byte-hex>" --chain solana --num 100 --json \
  --solana-mode cold-export

# === Option D: PDA addresses (receive-only) ===
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
# Encrypt wallet output (password from command line)
xgen gen --chain solana --password "mypassword" --output wallet.enc

# Encrypt with interactive prompt (secure — no password in shell history)
xgen gen --chain solana --encrypt --output wallet.enc
# → Enter encryption password: 
# → Confirm encryption password:

# Decrypt
xgen decrypt wallet.enc
xgen decrypt wallet.enc --output wallet.json
xgen decrypt wallet.enc --password "mypassword"   # non-interactive
```

Uses AES-256-GCM with scrypt key derivation (N=2^16, r=8, p=1). Salt and nonce are generated via OS-level CSPRNG (`OsRng`). Wallet version field is enforced during decryption.

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

## Library Usage

`xgen` exposes a public library interface. Add it to your `Cargo.toml`:

```toml
[dependencies]
xgen = "1.1"
bip39 = "2.2"
```

### Programmatic Address Generation

```rust
use xgen::{generate_for_chain, KeyInfo, WalletOutput};
use bip39::Mnemonic;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let mnemonic = Mnemonic::parse(phrase)?;
    let seed = mnemonic.to_seed("");

    // Generate an EVM address at index 0
    let wallet: WalletOutput = generate_for_chain(
        &seed,
        "m/44'/60'/0'/0/0",
        Some(0),       // index
        1,             // num
        &mnemonic,
        "",            // passphrase
        "evm",
        "full",        // solana_mode (ignored for evm)
        "",            // program_id (ignored for evm)
        &None,         // explicit indexes
    )?;

    for key in &wallet.keys {
        println!("Path: {}, Address: {}", key.path, key.address);
    }
    Ok(())
}
```

### xpub Watch-Only Mode

```rust
use xgen::generate_from_xpub;

let wallet = generate_from_xpub(
    "xpub6DCoCpSuQZB2...",
    "m/44'/60'/0'/0",
    None,           // index (None = use num)
    10,             // num
    "evm",
)?;
```

### xpriv Derivation Mode

```rust
use xgen::generate_from_xpriv;

// secp256k1 (EVM/BTC): standard BIP32 xpriv base58
let wallet = generate_from_xpriv(
    "xprv9wTYmMFdV23N2TdNG573QoEsfRr...",
    "m/44'/60'/0'/0",
    None,            // index (None = use num)
    5,               // num
    "evm",
    "full",          // solana_mode (ignored for evm)
    "",              // program_id (ignored for evm)
    &None,           // explicit indexes
)?;

// Ed25519 (Solana): 64-byte hex xpriv (private_key || chain_code)
let wallet = generate_from_xpriv(
    "a1b2c3...64_byte_hex...",
    "m/44'/501'/0'/0'",
    Some(0),         // specific index
    1,               // num
    "solana",
    "cold-export",   // solana_mode
    "",              // program_id
    &None,           // explicit indexes
)?;
```

### Encryption / Decryption

```rust
use xgen::{encrypt_data, decrypt_data, EncryptedWallet};

// Encrypt a wallet JSON string
let encrypted = encrypt_data(wallet_json, "password")?;

// Decrypt
let enc: EncryptedWallet = serde_json::from_str(&encrypted)?;
let decrypted = decrypt_data(&enc, "password")?;
```

### Public API

| Function / Type | Description |
|-----------------|-------------|
| `generate_for_chain(...)` | Generate HD wallet addresses for a chain |
| `generate_from_xpub(...)` | Watch-only address generation from xpub |
| `generate_from_xpriv(...)` | Key derivation from extended private key (BIP32 or SLIP-0010) |
| `derive_slip10_ed25519_child(...)` | SLIP-0010 child derivation from parent 64-byte state |
| `generate_evm(...)` / `generate_bitcoin(...)` / `generate_solana(...)` | Single-chain generators |
| `encrypt_data(...)` / `decrypt_data(...)` | AES-256-GCM encrypt/decrypt |
| `eth_address(&[u8])` | EIP-55 checksummed EVM address from pubkey |
| `derive_slip10_ed25519(...)` | SLIP-0010 Ed25519 key derivation |
| `parse_path(...)` | Parse BIP44 derivation path string |
| `get_default_path(...)` | Get default BIP44 path per chain |
| `KeyInfo` / `WalletOutput` / `EncryptedWallet` | Serializable data structures |
| `HARDENED` | Hardened index constant (`0x80000000`) |

---

## Development

```bash
cargo build
cargo test          # 62 integration tests
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
├── src/
│   ├── lib.rs               # Core library (types, crypto, generators)
│   └── main.rs             # CLI wrapper (arg parsing, terminal output)
├── tests/
│   ├── integration.rs       # 62 integration tests
│   └── e2e-exchange.sh     # E2E on Anvil + Solana + bitcoind
├── Cargo.toml               # lib + bin targets
├── .github/workflows/
│   ├── ci.yml               # fmt + clippy + test on push/PR
│   └── publish.yml          # Publish to crates.io on release
├── README.md
└── LICENSE
```

---

## Security

- **EIP-55 compliance:** EVM addresses use correct checksum (Keccak256 of address hex string)
- **SLIP-0010:** Solana Ed25519 derivation enforces all-hardened path segments for security and Phantom/Solfare compatibility
- **BIP32 compression:** Bitcoin public keys enforce standard compressed format (33 bytes)
- **CSPRNG:** Salt and nonce generated via `OsRng` (OS-level entropy)
- **Version enforcement:** Encrypted wallets validate version on decrypt
- **Interactive password prompt:** Use `--encrypt` without `--password` for secure, non-history-leaking password entry
- **Memory hygiene:** Sensitive keys zeroized on drop via `zeroize` crate
- **Never share your mnemonic or private keys**
- Use `--encrypt` when saving to disk
- Always prefer the interactive `--encrypt` prompt over `--password` on the command line
- Generate addresses offline when possible
- For Solana, prefer `cold-export` or `pda` mode on hot servers
- Hardware wallets are always safer for production use
- The `full` mode on Solana exposes private keys on the hot server

---

## License

MIT — see [LICENSE](LICENSE)

