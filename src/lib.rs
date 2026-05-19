use anyhow::{Context, Result};
use bip39::{Language, Mnemonic};
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::NetworkKind;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use zeroize::Zeroize;

pub const HARDENED: u32 = 0x80000000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeyInfo {
    pub index: u32,
    pub path: String,
    pub xprv: Option<String>,
    pub xpub: Option<String>,
    pub private_key: String,
    pub public_key: String,
    pub address: String,
    pub wif: Option<String>,
}

impl Drop for KeyInfo {
    fn drop(&mut self) {
        self.private_key.zeroize();
        if let Some(ref mut xprv) = self.xprv {
            xprv.zeroize();
        }
        if let Some(ref mut wif) = self.wif {
            wif.zeroize();
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletOutput {
    pub mnemonic: String,
    pub passphrase: String,
    pub chain: String,
    pub master_xprv: Option<String>,
    pub master_xpub: Option<String>,
    pub keys: Vec<KeyInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EncryptedWallet {
    pub version: u32,
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}

pub fn get_default_path(chain: &str, account: u32, change: u32, _hw_sim: bool) -> String {
    match chain {
        "evm" | "ethereum" => format!("m/44'/60'/{account}'/{change}/0"),
        "btc" | "bitcoin" => format!("m/44'/0'/{account}'/{change}/0"),
        "solana" => format!("m/44'/501'/{account}'/{change}'"),
        _ => format!("m/44'/60'/{account}'/{change}/0"),
    }
}

pub fn get_or_generate_mnemonic(mnemonic: Option<String>, strength: u32) -> Result<Mnemonic> {
    match mnemonic {
        Some(raw_mnemonic) => {
            let sanitized: String = raw_mnemonic
                .to_lowercase()
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join(" ");
            Mnemonic::parse_in(Language::English, sanitized).context("Invalid mnemonic")
        }
        None => {
            let word_count = if strength == 24 { 24 } else { 12 };
            Mnemonic::generate_in(Language::English, word_count)
                .context("Failed to generate mnemonic")
        }
    }
}

pub fn is_ed25519_chain(chain: &str) -> bool {
    matches!(chain, "solana")
}

pub fn derive_slip10_ed25519(seed: &[u8], path: &[u32]) -> Result<[u8; 64]> {
    use hmac::{KeyInit, Mac};

    let mut mac = <hmac::Hmac<sha2::Sha512>>::new_from_slice(b"ed25519 seed")
        .expect("HMAC accepts any key length");
    mac.update(seed);
    let mut i = mac.finalize().into_bytes();

    for &idx in path {
        if idx < HARDENED {
            anyhow::bail!(
                "SLIP-0010 Ed25519 violation: path segment index {:#x} is unhardened. All segments must be hardened.",
                idx
            );
        }

        let mut mac = <hmac::Hmac<sha2::Sha512>>::new_from_slice(&i[32..])
            .expect("HMAC accepts any key length");
        mac.update(&[0u8]);
        mac.update(&i[..32]);
        mac.update(&idx.to_be_bytes());
        i = mac.finalize().into_bytes();
    }

    let mut res = [0u8; 64];
    res.copy_from_slice(&i);
    Ok(res)
}

pub fn derive_slip10_ed25519_child(parent: &[u8; 64], child_index: u32) -> Result<[u8; 64]> {
    use hmac::{KeyInit, Mac};

    if child_index < HARDENED {
        anyhow::bail!(
            "SLIP-0010 child derivation index {:#x} is unhardened. Only hardened child derivation is secure.",
            child_index
        );
    }

    let mut mac = <hmac::Hmac<sha2::Sha512>>::new_from_slice(&parent[32..])
        .context("Failed to construct HMAC state from parent chain code")?;

    mac.update(&[0u8]);
    mac.update(&parent[..32]);
    mac.update(&child_index.to_be_bytes());

    let i = mac.finalize().into_bytes();
    let mut res = [0u8; 64];
    res.copy_from_slice(&i);
    Ok(res)
}

pub fn parse_path(path_str: &str) -> Result<Vec<u32>> {
    let trimmed = path_str
        .strip_prefix("m/")
        .or_else(|| path_str.strip_prefix("m"))
        .unwrap_or(path_str);
    if trimmed.is_empty() {
        return Ok(vec![]);
    }
    let mut indexes = Vec::new();
    for part in trimmed.split('/') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(num_str) = part.strip_suffix('\'') {
            let num: u32 = num_str.parse().context("Invalid hardened path segment")?;
            if num >= HARDENED {
                anyhow::bail!(
                    "Hardened path index '{}' is out of range. Max index is {}.",
                    num,
                    HARDENED - 1
                );
            }
            indexes.push(num + HARDENED);
        } else {
            let num: u32 = part.parse().context("Invalid path segment")?;
            if num >= HARDENED {
                anyhow::bail!(
                    "Standard path index '{}' is out of range. Max index is {}.",
                    num,
                    HARDENED - 1
                );
            }
            indexes.push(num);
        }
    }
    Ok(indexes)
}

pub fn parse_indexes(indexes_str: &str) -> Result<Vec<u32>> {
    indexes_str
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<u32>()
                .with_context(|| format!("Invalid index: '{}'", s.trim()))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn generate_for_chain(
    seed: &[u8],
    base_path: &str,
    specific_index: Option<u32>,
    num: u32,
    mnemonic: &Mnemonic,
    bip39_pass: &str,
    chain: &str,
    solana_mode: &str,
    program_id: &str,
    indexes: &Option<String>,
) -> Result<WalletOutput> {
    let indices: Vec<u32> = if let Some(idx_str) = indexes {
        parse_indexes(idx_str)?
    } else if let Some(single) = specific_index {
        vec![single]
    } else {
        (0..num).collect()
    };

    let mut keys = vec![];

    for &idx in &indices {
        let path = build_derivation_path(base_path, idx, chain);

        let info = match chain {
            "evm" | "ethereum" => generate_evm(seed, &path, idx)?,
            "btc" | "bitcoin" => generate_bitcoin(seed, &path, idx)?,
            "solana" => generate_solana(seed, &path, idx, solana_mode, program_id)?,
            _ => anyhow::bail!(
                "Chain '{}' is not supported. Supported: evm, btc, solana",
                chain
            ),
        };

        keys.push(info);
    }

    let mut wallet = WalletOutput {
        mnemonic: mnemonic.to_string(),
        passphrase: bip39_pass.to_string(),
        chain: chain.to_string(),
        master_xprv: None,
        master_xpub: None,
        keys,
    };

    let account_path = base_path.trim_end_matches(|c: char| c.is_numeric() || c == '/');
    if is_ed25519_chain(chain) {
        if let Ok(indexes) = parse_path(account_path) {
            if let Ok(derived) = derive_slip10_ed25519(seed, &indexes) {
                let chain_code = &derived[32..];
                let sk_bytes: [u8; 32] = derived[..32].try_into().unwrap();
                let signing_key = SigningKey::from_bytes(&sk_bytes);
                let pubkey_bytes = signing_key.verifying_key().to_bytes();
                let mut xpub = chain_code.to_vec();
                xpub.extend_from_slice(&pubkey_bytes);
                wallet.master_xpub = Some(hex::encode(xpub));
            }
        }
    } else if let Ok(account_key) = derive_secp_key(seed, account_path) {
        let secp = bitcoin::secp256k1::Secp256k1::new();
        wallet.master_xpub = Some(Xpub::from_priv(&secp, &account_key).to_string());
    }
    if wallet.master_xprv.is_none() {
        wallet.master_xprv = Some("Master key hidden for security".to_string());
    }
    Ok(wallet)
}

pub fn build_xpub(
    pubkey: bitcoin::secp256k1::PublicKey,
    chain_code: bitcoin::bip32::ChainCode,
) -> bitcoin::bip32::Xpub {
    bitcoin::bip32::Xpub {
        network: bitcoin::NetworkKind::Main,
        depth: 0,
        parent_fingerprint: bitcoin::bip32::Fingerprint::default(),
        child_number: bitcoin::bip32::ChildNumber::Normal { index: 0 },
        public_key: pubkey,
        chain_code,
    }
}

pub fn parse_xpub(xpub_str: &str) -> Result<bitcoin::bip32::Xpub> {
    if let Ok(xpub) = bitcoin::bip32::Xpub::from_str(xpub_str) {
        return Ok(xpub);
    }

    let stripped = xpub_str.strip_prefix("xpub").unwrap_or(xpub_str);
    if stripped.len() >= 64 && stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        let bytes = hex::decode(stripped).context("Invalid hex")?;
        if bytes.len() < 32 {
            anyhow::bail!("Hex too short");
        }
        let mut chain = [0u8; 32];
        chain.copy_from_slice(&bytes[..32]);
        let cc = bitcoin::bip32::ChainCode::from_hex(&hex::encode(chain))
            .expect("32 bytes is always valid hex");
        let pk = bitcoin::secp256k1::PublicKey::from_slice(&bytes[32..])
            .context("Invalid public key in hex xpub")?;
        return Ok(build_xpub(pk, cc));
    }

    anyhow::bail!("Invalid xpub: expected base58 BIP32 xpub or hex chain_code(32)+pubkey(33)")
}

pub fn generate_from_xpub(
    xpub_str: &str,
    base_path: &str,
    specific_index: Option<u32>,
    num: u32,
    chain: &str,
) -> Result<WalletOutput> {
    let count = specific_index.map_or(num, |_| 1);
    let mut keys = vec![];

    if is_ed25519_chain(chain) {
        anyhow::bail!("xpub mode is not supported for {} (Ed25519 curve).", chain);
    }

    let xpub = parse_xpub(xpub_str)?;
    let secp = bitcoin::secp256k1::Secp256k1::new();

    for i in 0..count {
        let idx = specific_index.unwrap_or(i);
        let path = format!("{}/{}", base_path.trim_end_matches('/'), idx);

        let child_idx = bitcoin::bip32::ChildNumber::from_normal_idx(idx)?;
        let child = xpub
            .ckd_pub(&secp, child_idx)
            .context("Failed to derive child")?;

        let pk_bytes = child.public_key.serialize_uncompressed();
        let address = match chain {
            "evm" | "ethereum" => eth_address(&pk_bytes),
            "btc" | "bitcoin" => {
                let cpubkey =
                    bitcoin::CompressedPublicKey::from_slice(&child.public_key.serialize())
                        .expect("valid compressed pk");
                bitcoin::Address::p2wpkh(&cpubkey, bitcoin::Network::Bitcoin).to_string()
            }
            _ => anyhow::bail!("xpub mode is not supported for chain '{}'", chain),
        };

        let info = KeyInfo {
            index: idx,
            path,
            xprv: None,
            xpub: Some(xpub_str.to_string()),
            private_key: "WATCH-ONLY".to_string(),
            public_key: hex::encode(pk_bytes),
            address,
            wif: None,
        };

        keys.push(info);
    }

    Ok(WalletOutput {
        mnemonic: "WATCH-ONLY (xpub mode)".to_string(),
        passphrase: String::new(),
        chain: chain.to_string(),
        master_xprv: None,
        master_xpub: Some(xpub_str.to_string()),
        keys,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn generate_from_xpriv(
    xpriv_str: &str,
    base_path: &str,
    specific_index: Option<u32>,
    num: u32,
    chain: &str,
    solana_mode: &str,
    program_id: &str,
    indexes: &Option<String>,
) -> Result<WalletOutput> {
    let indices: Vec<u32> = if let Some(idx_str) = indexes {
        parse_indexes(idx_str)?
    } else if let Some(single) = specific_index {
        vec![single]
    } else {
        (0..num).collect()
    };

    let mut keys = vec![];

    if is_ed25519_chain(chain) {
        let bytes = hex::decode(xpriv_str.strip_prefix("0x").unwrap_or(xpriv_str))
            .context("Invalid hex for Ed25519 xpriv")?;
        if bytes.len() != 64 {
            anyhow::bail!(
                "Ed25519 xpriv must be 64 bytes (32-byte private key + 32-byte chain code), got {} bytes",
                bytes.len()
            );
        }
        let parent: [u8; 64] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to convert xpriv bytes to [u8; 64]"))?;

        for &idx in &indices {
            let path = format!("{}/{}'", base_path.trim_end_matches('/'), idx);
            let child = derive_slip10_ed25519_child(&parent, idx + HARDENED)?;
            let sk_bytes: [u8; 32] = child[..32]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to extract 32-byte secret key slice"))?;
            let chain_code = &child[32..];

            let signing_key = SigningKey::from_bytes(&sk_bytes);
            let verifying_key = signing_key.verifying_key();
            let user_pubkey = Pubkey::new_from_array(verifying_key.to_bytes());

            let (address, private_key, xprv, xpub, wif) = match solana_mode {
                "pda" => {
                    let program_pubkey = if program_id.is_empty() {
                        Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap()
                    } else {
                        Pubkey::from_str(program_id).context("Invalid program ID for PDA")?
                    };
                    let seed_label = format!("user_deposit_{}", idx);
                    let (pda, _bump) = Pubkey::find_program_address(
                        &[seed_label.as_bytes(), &user_pubkey.to_bytes()],
                        &program_pubkey,
                    );
                    (
                        pda.to_string(),
                        "PDA_CAN_RECEIVE_ONLY_SWEEP_NEEDS_PROGRAM".to_string(),
                        None,
                        None,
                        None,
                    )
                }
                "cold-export" => (
                    user_pubkey.to_string(),
                    "HIDDEN_FOR_SECURITY".to_string(),
                    None,
                    None,
                    None,
                ),
                _ => {
                    let mut xpub_bytes = chain_code.to_vec();
                    xpub_bytes.extend_from_slice(&verifying_key.to_bytes());
                    (
                        user_pubkey.to_string(),
                        hex::encode(sk_bytes),
                        Some(hex::encode(child)),
                        Some(hex::encode(xpub_bytes)),
                        None,
                    )
                }
            };

            keys.push(KeyInfo {
                index: idx,
                path,
                xprv,
                xpub,
                private_key,
                public_key: hex::encode(verifying_key.to_bytes()),
                address,
                wif,
            });
        }
    } else {
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let parent_xpriv =
            bitcoin::bip32::Xpriv::from_str(xpriv_str).context("Invalid BIP32 xpriv string")?;

        for &idx in &indices {
            let path = format!("{}/{}", base_path.trim_end_matches('/'), idx);

            let child_idx = bitcoin::bip32::ChildNumber::from_normal_idx(idx)?;
            let child_xpriv = parent_xpriv
                .derive_priv(&secp, &[child_idx])
                .context("Failed to derive child private key")?;

            let mut priv_key = child_xpriv.to_priv();
            priv_key.compressed = true;
            let pub_key = priv_key.public_key(&secp);
            let pub_bytes = pub_key.inner.serialize_uncompressed();
            let compressed_pub = pub_key.inner.serialize();

            let address = match chain {
                "evm" | "ethereum" => eth_address(&pub_bytes),
                "btc" | "bitcoin" => {
                    let cpubkey =
                        bitcoin::CompressedPublicKey::from_slice(&pub_key.inner.serialize())
                            .expect("valid compressed pk");
                    bitcoin::Address::p2wpkh(&cpubkey, bitcoin::Network::Bitcoin).to_string()
                }
                _ => anyhow::bail!("Chain '{}' not supported for xpriv mode", chain),
            };

            let wif = if chain == "btc" || chain == "bitcoin" {
                Some(priv_key.to_wif())
            } else {
                None
            };

            let sk_bytes = child_xpriv.private_key.secret_bytes();
            let xprv = child_xpriv.to_string();
            let xpub = Xpub::from_priv(&secp, &child_xpriv).to_string();

            keys.push(KeyInfo {
                index: idx,
                path,
                xprv: Some(xprv),
                xpub: Some(xpub),
                private_key: if chain == "evm" || chain == "ethereum" {
                    format!("0x{}", hex::encode(sk_bytes))
                } else {
                    priv_key.to_wif()
                },
                public_key: hex::encode(compressed_pub),
                address,
                wif,
            });
        }
    }

    Ok(WalletOutput {
        mnemonic: "Derived from parent xpriv".to_string(),
        passphrase: String::new(),
        chain: chain.to_string(),
        master_xprv: Some(xpriv_str.to_string()),
        master_xpub: None,
        keys,
    })
}

pub fn build_derivation_path(base: &str, index: u32, chain: &str) -> String {
    if is_ed25519_chain(chain) {
        let normalized_base = base
            .split('/')
            .map(|segment| {
                if segment == "m" || segment.is_empty() || segment.ends_with('\'') {
                    segment.to_string()
                } else {
                    format!("{}'", segment)
                }
            })
            .collect::<Vec<String>>()
            .join("/");

        format!("{normalized_base}/{}'", index)
    } else {
        let base = base.trim_end_matches(|c: char| c.is_numeric() || c == '/');
        format!("{}/{}", base, index)
    }
}

pub fn derive_secp_key(seed: &[u8], path: &str) -> Result<Xpriv> {
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let master = Xpriv::new_master(NetworkKind::Main, seed)
        .context("Failed to create master key from seed")?;
    let stripped = path.strip_prefix("m/").unwrap_or(path);
    let dp = DerivationPath::from_str(stripped).context("Invalid derivation path")?;
    let child = master
        .derive_priv(&secp, &dp)
        .context("Key derivation failed")?;
    Ok(child)
}

pub fn generate_evm(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let child = derive_secp_key(seed, path)?;

    let pub_key = child.to_priv().public_key(&secp);
    let pk_secp = pub_key.inner;
    let pub_bytes = pk_secp.serialize_uncompressed();
    let address = eth_address(&pub_bytes);

    let sk_bytes = child.private_key.secret_bytes();
    let xprv = child.to_string();
    let xpub = Xpub::from_priv(&secp, &child).to_string();

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: Some(xprv),
        xpub: Some(xpub),
        private_key: format!("0x{}", hex::encode(sk_bytes)),
        public_key: format!("0x{}", hex::encode(pub_bytes)),
        address,
        wif: None,
    })
}

pub fn generate_bitcoin(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let child = derive_secp_key(seed, path)?;

    let mut priv_key = child.to_priv();
    priv_key.compressed = true;

    let pub_key = priv_key.public_key(&secp);

    let cpubkey = bitcoin::CompressedPublicKey::from_slice(&pub_key.inner.serialize())
        .expect("valid compressed pk");
    let address = bitcoin::Address::p2wpkh(&cpubkey, bitcoin::Network::Bitcoin);

    let wif = priv_key.to_wif();
    let pub_bytes = pub_key.inner.serialize().to_vec();

    let xprv = child.to_string();
    let xpub = Xpub::from_priv(&secp, &child).to_string();

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: Some(xprv),
        xpub: Some(xpub),
        private_key: wif.clone(),
        public_key: hex::encode(pub_bytes),
        address: address.to_string(),
        wif: Some(wif),
    })
}

pub fn generate_solana(
    seed: &[u8],
    path: &str,
    idx: u32,
    mode: &str,
    program_id: &str,
) -> Result<KeyInfo> {
    let path_indexes = parse_path(path)?;
    let derived = derive_slip10_ed25519(seed, &path_indexes)?;
    let sk_bytes: [u8; 32] = derived[..32]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse private key slice"))?;

    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();
    let user_pubkey = Pubkey::new_from_array(verifying_key.to_bytes());

    let (address, private_key, xprv, xpub, wif) = match mode {
        "pda" => {
            let program_pubkey = if program_id.is_empty() {
                Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap()
            } else {
                Pubkey::from_str(program_id).context("Invalid program ID for PDA")?
            };
            let seed_label = format!("user_deposit_{}", idx);
            let (pda, _bump) = Pubkey::find_program_address(
                &[seed_label.as_bytes(), &user_pubkey.to_bytes()],
                &program_pubkey,
            );
            (
                pda.to_string(),
                "PDA_CAN_RECEIVE_ONLY_SWEEP_NEEDS_PROGRAM".to_string(),
                None,
                None,
                None,
            )
        }
        "cold-export" => (
            user_pubkey.to_string(),
            "HIDDEN_FOR_SECURITY".to_string(),
            None,
            None,
            None,
        ),
        _ => (
            user_pubkey.to_string(),
            hex::encode(sk_bytes),
            Some(hex::encode(sk_bytes)),
            Some(user_pubkey.to_string()),
            None,
        ),
    };

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv,
        xpub,
        private_key,
        public_key: hex::encode(verifying_key.to_bytes()),
        address,
        wif,
    })
}

pub fn eth_address(pubkey_bytes: &[u8]) -> String {
    use tiny_keccak::{Hasher, Keccak};
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]);
    hasher.finalize(&mut output);

    let addr = hex::encode(&output[12..]);

    let mut hash_output = [0u8; 32];
    let mut hasher2 = Keccak::v256();
    hasher2.update(addr.as_bytes());
    hasher2.finalize(&mut hash_output);
    let hash = hex::encode(hash_output);
    let hash_bytes = hash.as_bytes();

    let mut checksum = String::with_capacity(42);
    checksum.push_str("0x");
    for (i, c) in addr.chars().enumerate() {
        let n = match hash_bytes[i] {
            b @ b'0'..=b'9' => b - b'0',
            b @ b'a'..=b'f' => b - b'a' + 10,
            b @ b'A'..=b'F' => b - b'A' + 10,
            _ => 0,
        };
        checksum.push(if n > 7 { c.to_ascii_uppercase() } else { c });
    }
    checksum
}

pub fn encrypt_data(data: &str, password: &str) -> Result<String> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit};
    use base64::Engine;
    use rand::Rng;
    use scrypt::scrypt;

    let mut rng = rand::rng();

    let mut salt = [0u8; 16];
    rng.fill_bytes(&mut salt);

    let mut key = [0u8; 32];
    let params = scrypt::Params::new(16, 8, 1).context("Invalid scrypt params")?;

    scrypt(password.as_bytes(), &salt, &params, &mut key)
        .context("Scrypt key derivation failed")?;

    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow::anyhow!("AES init failed: {:?}", e))?;

    let mut nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce_bytes.into(), data.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

    let engine = base64::engine::general_purpose::STANDARD;
    let enc = EncryptedWallet {
        version: 1,
        salt: engine.encode(salt),
        nonce: engine.encode(nonce_bytes),
        ciphertext: engine.encode(ciphertext),
    };

    serde_json::to_string_pretty(&enc).context("Failed to serialize encrypted data")
}

pub fn decrypt_data(enc: &EncryptedWallet, password: &str) -> Result<String> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit};
    use base64::Engine;
    use scrypt::scrypt;

    if enc.version != 1 {
        anyhow::bail!("Unsupported wallet version: {}", enc.version);
    }

    let engine = base64::engine::general_purpose::STANDARD;

    let salt = engine.decode(&enc.salt).context("Invalid salt")?;
    let nonce_vec = engine.decode(&enc.nonce).context("Invalid nonce")?;
    let ciphertext = engine
        .decode(&enc.ciphertext)
        .context("Invalid ciphertext")?;

    let nonce_arr: [u8; 12] = nonce_vec
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid nonce length"))?;

    let mut key = [0u8; 32];
    let params = scrypt::Params::new(16, 8, 1)?;
    scrypt(password.as_bytes(), &salt, &params, &mut key)?;

    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let plaintext = cipher
        .decrypt(&nonce_arr.into(), ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Decryption failed. Wrong password?"))?;

    Ok(String::from_utf8(plaintext)?)
}
