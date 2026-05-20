---
name: xgen
description: Use xgen — a Rust library and CLI for multi-chain HD wallet generation — to generate BIP39 mnemonics, derive EVM/Bitcoin/Solana addresses, manage xpubs/xprivs, or encrypt wallets in any Rust project. Also use when running xgen CLI commands for wallet generation. Make sure to use this skill whenever the user mentions xgen, wallet generation, crypto addresses, mnemonics, HD wallets, BIP39, BIP44, BIP32, xpub, xpriv, Solana key derivation, exchange deposit address generation, or needs to integrate multi-chain wallet functionality into a Rust project.
---

# xgen — Multi-chain HD Wallet for Rust

xgen is a Rust crate and CLI that generates HD wallets from BIP39 mnemonics across EVM, Bitcoin, and Solana.

## Quick start — adding to a Rust project

Add to `Cargo.toml`:

```toml
[dependencies]
xgen = "1.1"
bip39 = "2.2"
```

Basic wallet generation:

```rust
use xgen::{generate_for_chain, WalletOutput};
use bip39::Mnemonic;

let mnemonic = Mnemonic::parse("abandon abandon abandon ... about")?;
let seed = mnemonic.to_seed("");

let wallet: WalletOutput = generate_for_chain(
    &seed,
    "m/44'/60'/0'/0/0",
    Some(0),
    1,
    &mnemonic,
    "",
    "evm",
    "full",
    "",
    &None,
)?;

for key in &wallet.keys {
    println!("{} -> {}", key.path, key.address);
}
```

## API reference

All functions and types are exported from the `xgen` crate. Import what you need:

```rust
use xgen::{
    generate_for_chain, generate_from_xpub, generate_from_xpriv,
    generate_evm, generate_bitcoin, generate_solana,
    derive_slip10_ed25519, derive_slip10_ed25519_child,
    encrypt_data, decrypt_data,
    eth_address,
    parse_path, parse_indexes, parse_xpub,
    get_default_path, get_or_generate_mnemonic,
    KeyInfo, WalletOutput, EncryptedWallet,
    HARDENED,
};
```

### Core generation functions

**`generate_for_chain`** — Generate keys from a BIP39 seed and mnemonic.

```rust
pub fn generate_for_chain(
    seed: &[u8],                  // BIP39 seed bytes (64 bytes)
    base_path: &str,              // BIP44 path template, e.g. "m/44'/60'/0'/0/0"
    specific_index: Option<u32>,  // Single index (Some(42)) or None for range
    num: u32,                     // Number of sequential indices (used if specific_index is None and indexes is None)
    mnemonic: &Mnemonic,          // BIP39 mnemonic object
    bip39_pass: &str,             // BIP39 passphrase (usually "")
    chain: &str,                  // "evm", "btc", "solana"
    solana_mode: &str,            // Solana mode: "full", "cold-export", "hsm-sim", "pda" (ignored for EVM/BTC)
    program_id: &str,             // Solana PDA program ID (ignored otherwise)
    indexes: &Option<String>,     // Comma-separated indices like "0,5,10" — overrides specific_index and num
) -> Result<WalletOutput>
```

Returns a `WalletOutput` with mnemonic, chain, master keys, and a `Vec<KeyInfo>`.

**`generate_from_xpub`** — Watch-only address derivation (EVM/BTC only, not Solana).

```rust
pub fn generate_from_xpub(
    xpub_str: &str,               // BIP32 xpub string (base58)
    base_path: &str,              // Base derivation path, e.g. "m/44'/60'/0'/0"
    specific_index: Option<u32>,  // Single index or None for range
    num: u32,                     // Number of addresses
    chain: &str,                  // "evm" or "btc"
) -> Result<WalletOutput>
```

Private keys show as `"WATCH-ONLY"`. Safe for hot servers.

**`generate_from_xpriv`** — Derive child keys from an extended private key.

```rust
pub fn generate_from_xpriv(
    xpriv_str: &str,              // BIP32 xpriv base58 (EVM/BTC) or 64-byte hex (Solana)
    base_path: &str,              // Base derivation path
    specific_index: Option<u32>,  // Single index or None
    num: u32,                     // Number of keys
    chain: &str,                  // "evm", "btc", "solana"
    solana_mode: &str,            // Solana mode (ignored for EVM/BTC)
    program_id: &str,             // Solana PDA program ID
    indexes: &Option<String>,     // Comma-separated indices
) -> Result<WalletOutput>
```

For Solana, xpriv is 64-byte hex: `private_key(32 bytes) || chain_code(32 bytes)`.

### Single-chain generators

These are lower-level, used internally by `generate_for_chain`:

```rust
pub fn generate_evm(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo>
pub fn generate_bitcoin(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo>
pub fn generate_solana(seed: &[u8], path: &str, idx: u32, mode: &str, program_id: &str) -> Result<KeyInfo>
```

### EIP-55 address

```rust
pub fn eth_address(pubkey_bytes: &[u8]) -> String
```

Takes an uncompressed 65-byte secp256k1 public key (starting with `0x04`), returns EIP-55 checksummed address like `"0xAbCd..."`.

### SLIP-0010 Ed25519 derivation (Solana)

```rust
pub fn derive_slip10_ed25519(seed: &[u8], path: &[u32]) -> Result<[u8; 64]>
```

Derives a 64-byte Solana key from a BIP39 seed and hardened path. All path segments must be hardened (>= `HARDENED`). Returns `[private_key(32) || chain_code(32)]`.

```rust
pub fn derive_slip10_ed25519_child(parent: &[u8; 64], child_index: u32) -> Result<[u8; 64]>
```

Derives a child from a 64-byte parent. `child_index` must be hardened.

### Encryption

```rust
pub fn encrypt_data(data: &str, password: &str) -> Result<String>
pub fn decrypt_data(enc: &EncryptedWallet, password: &str) -> Result<String>
```

AES-256-GCM with scrypt key derivation (N=2^16, r=8, p=1). Returns/accepts JSON-serialized `EncryptedWallet`.

### Utility functions

```rust
// Get default BIP44 path for a chain
pub fn get_default_path(chain: &str, account: u32, change: u32, hw_sim: bool) -> String
// "evm" -> "m/44'/60'/{account}'/{change}/0"
// "btc" -> "m/44'/0'/{account}'/{change}/0"
// "solana" -> "m/44'/501'/{account}'/{change}'"

// Parse a BIP44 path string to Vec<u32>
pub fn parse_path(path_str: &str) -> Result<Vec<u32>>

// Parse comma-separated index string to Vec<u32>
pub fn parse_indexes(indexes_str: &str) -> Result<Vec<u32>>

// Parse xpub string (base58 BIP32 or hex)
pub fn parse_xpub(xpub_str: &str) -> Result<bitcoin::bip32::Xpub>

// Generate or import a BIP39 mnemonic
pub fn get_or_generate_mnemonic(mnemonic: Option<String>, strength: u32) -> Result<Mnemonic>

// Check if chain uses Ed25519 (Solana)
pub fn is_ed25519_chain(chain: &str) -> bool
```

## Data types

### `KeyInfo`

```rust
pub struct KeyInfo {
    pub index: u32,              // Derivation index
    pub path: String,            // Full BIP44 path
    pub xprv: Option<String>,    // Extended private key (base58 or hex)
    pub xpub: Option<String>,    // Extended public key (base58 or hex)
    pub private_key: String,     // Hex (EVM), WIF (BTC), hex (Solana full/hsm), or "HIDDEN_FOR_SECURITY"
    pub public_key: String,      // Hex-encoded public key
    pub address: String,         // EIP-55 hex (EVM), bech32 (BTC), base58 (Solana)
    pub wif: Option<String>,     // WIF format (BTC only)
}
```

Fields vary by chain and mode:
- **EVM**: `private_key` is `0x`-prefixed hex, `wif` is `None`
- **Bitcoin**: `private_key` is WIF, `wif` is `Some(wif)`, `public_key` is compressed (33 bytes)
- **Solana full/hsm**: `private_key` is hex, `xprv` is hex, `xpub` is hex
- **Solana cold-export**: `private_key` is `"HIDDEN_FOR_SECURITY"`, `xprv`/`xpub` are `None`
- **Solana PDA**: `private_key` is `"PDA_CAN_RECEIVE_ONLY_SWEEP_NEEDS_PROGRAM"`, `address` is the PDA
- **Watch-only (xpub)**: `private_key` is `"WATCH-ONLY"`, `xprv` is `None`

### `WalletOutput`

```rust
pub struct WalletOutput {
    pub mnemonic: String,            // BIP39 phrase
    pub passphrase: String,          // BIP39 passphrase
    pub chain: String,               // Chain name
    pub master_xprv: Option<String>, // Usually "Master key hidden for security"
    pub master_xpub: Option<String>, // BIP32 xpub (secp256k1) or hex (Solana: chain_code + pubkey)
    pub keys: Vec<KeyInfo>,          // Derived keys
}
```

### `EncryptedWallet`

```rust
pub struct EncryptedWallet {
    pub version: u32,     // Must be 1
    pub salt: String,     // Base64-encoded 16-byte salt
    pub nonce: String,    // Base64-encoded 12-byte nonce
    pub ciphertext: String, // Base64-encoded AES-256-GCM ciphertext
}
```

## Common patterns

### Generate a wallet for a single user

```rust
use xgen::{generate_for_chain, get_default_path};
use bip39::Mnemonic;

fn user_deposit_address(mnemonic: &Mnemonic, user_index: u32, chain: &str) -> Result<String> {
    let seed = mnemonic.to_seed("");
    let path = get_default_path(chain, 0, 0, false); // account 0, change 0
    let wallet = generate_for_chain(&seed, &path, Some(user_index), 1, mnemonic, "", chain, "full", "", &None)?;
    Ok(wallet.keys[0].address.clone())
}
```

### Exchange cold/hot wallet pattern

```rust
use xgen::{generate_for_chain, generate_from_xpub};
use bip39::Mnemonic;

// COLD WALLET: Generate account xpub from mnemonic
let mnemonic = Mnemonic::parse("twelve word phrase")?;
let seed = mnemonic.to_seed("");
let wallet = generate_for_chain(&seed, "m/44'/60'/0'/0/0", Some(0), 1, &mnemonic, "", "evm", "full", "", &None)?;
let account_xpub = wallet.master_xpub.unwrap();

// HOT SERVER: Derive deposit addresses from xpub (no private keys!)
let deposits = generate_from_xpub(&account_xpub, "m/44'/60'/0'/0", None, 1000, "evm")?;
// deposits.keys[0].private_key == "WATCH-ONLY" — safe for hot server
```

### Solana with cold-export security

```rust
// Generate addresses without exposing private keys
let wallet = generate_for_chain(&seed, "m/44'/501'/0'/0'", None, 100, &mnemonic, "", "solana", "cold-export", "", &None)?;
// wallet.keys[0].private_key == "HIDDEN_FOR_SECURITY"
```

### Solana xpriv derivation

```rust
use xgen::generate_from_xpriv;

// 64-byte hex: private_key(32) || chain_code(32)
let xpriv_hex = "a1b2c3d4...64_chars_total...";
let wallet = generate_from_xpriv(xpriv_hex, "m/44'/501'/0'/0'", None, 50, "solana", "cold-export", "", &None)?;
```

### Encrypt wallet output

```rust
use xgen::{encrypt_data, decrypt_data};

let json = serde_json::to_string(&wallet)?;
let encrypted = encrypt_data(&json, "strong_password")?;
// Save encrypted to file...

// Later:
let enc: EncryptedWallet = serde_json::from_str(&encrypted)?;
let decrypted = decrypt_data(&enc, "strong_password")?;
let wallet: WalletOutput = serde_json::from_str(&decrypted)?;
```

### Batch derivation with specific indices

```rust
// Derive only indices 0, 5, 10, 42
let wallet = generate_for_chain(&seed, "m/44'/60'/0'/0/0", None, 1, &mnemonic, "", "evm", "full", "", &Some("0,5,10,42".to_string()))?;
// wallet.keys has 4 entries
```

## Chain-specific details

| Chain | Curve | BIP44 Coin | Path | Address | xpub OK? | xpriv format |
|-------|-------|:----------:|------|---------|:--------:|-------------|
| `"evm"` | secp256k1 | 60 | `m/44'/60'/{a}'/{c}/{i}` | `0x...` EIP-55 | Yes | BIP32 base58 |
| `"btc"` | secp256k1 | 0 | `m/44'/0'/{a}'/{c}/{i}` | `bc1q...` P2WPKH | Yes | BIP32 base58 |
| `"solana"` | Ed25519 | 501 | `m/44'/501'/{a}'/{c}'` | Base58 | No | 64-byte hex |

**Solana security modes:**

| Mode | `.private_key` | `.xprv` | Use case |
|------|:---|:---:|----------|
| `"full"` | Hex secret | Some | Testing, personal use |
| `"cold-export"` | `"HIDDEN_FOR_SECURITY"` | None | Hot server deposit generation |
| `"hsm-sim"` | Hex secret | Some | Simulated HSM |
| `"pda"` | `"PDA_CAN_RECEIVE_ONLY..."` | None | Receive-only monitoring |

**Important Solana notes:**
- All derivation path segments must be hardened (≥ `0x80000000`). Unhardened segments cause errors.
- SLIP-0010 is used for Ed25519 derivation. Keys are compatible with Phantom/Solfare when using the same mnemonic.
- PDA mode derives program-derived addresses (receive-only, needs on-chain program to sweep).

## CLI usage (for running xgen commands directly)

When you need to run xgen from the terminal rather than use it programmatically:

### Install

```bash
cargo install xgen
# Or use from source: cargo build --release && ./target/release/xgen
```

### `xgen gen` — key flags

| Flag | Purpose |
|------|---------|
| `-c, --chain` | `evm` (default), `btc`, `solana` |
| `-m, --mnemonic` | Import existing mnemonic |
| `--strength` | `12` or `24` words for new mnemonic |
| `-i, --index` | Single derivation index |
| `-n, --num` | Sequential count (default 1) |
| `--indexes` | Comma-separated, e.g. `"0,5,10"` |
| `--account`, `--change` | BIP44 account/change (default 0) |
| `--xpub` | Watch-only from xpub (EVM/BTC) |
| `--xpriv` | Derive from xpriv |
| `--solana-mode` | `full`, `cold-export`, `hsm-sim`, `pda` |
| `--json` | JSON output |
| `-o, --output` | Save to file |
| `--encrypt` | Encrypt output |
| `--password` | Password for encrypt/decrypt |

### Common CLI examples

```bash
# Generate EVM wallet
xgen gen --chain evm --num 5 --json

# Solana cold-export (safe for hot server)
xgen gen --chain solana --solana-mode cold-export --num 100 --json -o deposits.json

# Watch-only EVM from xpub
xgen gen --xpub "xpub6DCoCpSuQZB2..." --num 1000 --chain evm --json

# Derive Solana from xpriv
xgen gen --xpriv "64bytehex..." --chain solana --solana-mode cold-export --indexes "7,42,99" --json

# Encrypt wallet file
xgen gen --chain solana --encrypt --output wallet.enc

# Decrypt
xgen decrypt wallet.enc --output wallet.json
```

## Cryptography notes

- **EVM EIP-55**: Checksum computed as Keccak256 of the lowercase address hex string (not the public key).
- **Bitcoin**: Native SegWit P2WPKH (Bech32, `bc1q`). Compressed public keys (33 bytes).
- **Solana SLIP-0010**: HMAC-SHA512 based derivation, all-hardened segments enforced.
- **Encryption**: AES-256-GCM with scrypt (N=2^16, r=8, p=1), salt/nonce from `OsRng`.
- **Memory safety**: `KeyInfo` has a `Drop` impl that zeroizes `private_key`, `xprv`, and `wif` fields.
- **Version enforcement**: `EncryptedWallet.version` must be `1` on decrypt.

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `"SLIP-0010 Ed25519 violation"` | Unhardened path segment for Solana | Ensure all segments have `'`: `m/44'/501'/0'/0'` |
| `"xpub mode is not supported"` | xpub used for Solana | Solana requires private key for hardened derivation; use `--xpriv` instead |
| `"Chain 'X' is not supported"` | Invalid chain name | Use `"evm"`, `"btc"`, or `"solana"` |
| `"Ed25519 xpriv must be 64 bytes"` | Wrong xpriv length for Solana | Must be exactly 64 bytes: priv(32) + chain_code(32) |
| `"Invalid BIP32 xpriv string"` | Wrong format for EVM/BTC xpriv | Use base58 BIP32 xpriv starting with `xprv` |
| `"Decryption failed"` | Wrong password or corrupted data | Verify password; ensure file not truncated |
| `"Unsupported wallet version"` | Future xgen format | Update xgen crate |
