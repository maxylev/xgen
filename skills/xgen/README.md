# xgen skill

AI agent skill for the [xgen](https://crates.io/crates/xgen) multi-chain HD wallet library and CLI (Rust). Teaches agents how to generate EVM, Bitcoin, and Solana wallets from BIP39 mnemonics, manage xpubs/xprivs, and encrypt wallet output.

## Install

```bash
npx skills add maxylev/xgen
```

## What this skill covers

- **Library API** — all public functions (`generate_for_chain`, `generate_from_xpub`, `generate_from_xpriv`, `encrypt_data`, `eth_address`, etc.), types (`KeyInfo`, `WalletOutput`, `EncryptedWallet`), and usage patterns
- **CLI usage** — `xgen gen` and `xgen decrypt` with all flags, chain-specific details, and exchange workflows
- **Security** — Solana security modes (`cold-export`, `pda`, `hsm-sim`), cold/hot wallet architecture, encryption best practices

## Chains supported

| Chain | Curve | Address Format | xpub watch-only |
|-------|-------|----------------|:---------------:|
| EVM | secp256k1 | `0x...` EIP-55 | Yes |
| Bitcoin | secp256k1 | `bc1q...` P2WPKH | Yes |
| Solana | Ed25519 (SLIP-0010) | Base58 | No |
