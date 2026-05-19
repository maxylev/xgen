use anyhow::{Context, Result};
use bip39::{Language, Mnemonic};
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::{Address, NetworkKind};
use clap::Parser;
use colored::*;
use ed25519_dalek::SigningKey;
use qr2term::print_qr;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::fs;
use std::str::FromStr;

const HARDENED: u32 = 0x80000000;

#[derive(Parser)]
#[command(author, version, about = "xgen - Multi-chain HD Wallet CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    Gen {
        #[arg(short, long)]
        mnemonic: Option<String>,

        #[arg(short = 's', long, default_value = "")]
        passphrase: String,

        #[arg(short, long, default_value = "evm")]
        /// Target chain: `evm`, `btc`, `solana`
        chain: String,

        #[arg(short, long)]
        index: Option<u32>,

        #[arg(long, default_value_t = 0)]
        account: u32,

        #[arg(long, default_value_t = 0)]
        change: u32,

        #[arg(short, long, default_value_t = 1)]
        num: u32,

        #[arg(long, default_value_t = 12)]
        strength: u32,

        #[arg(long)]
        json: bool,

        #[arg(short, long)]
        output: Option<String>,

        #[arg(long)]
        qr: bool,

        #[arg(long)]
        encrypt: Option<String>,

        #[arg(long)]
        password: Option<String>,

        #[arg(long)]
        hw_sim: bool,

        #[arg(long)]
        xpub: Option<String>,

        #[arg(long)]
        xpub_path: Option<String>,

        /// Solana mode: full (keys visible), cold-export (keys hidden),
        /// hsm-sim (simulated HSM), pda (program-derived, receive-only)
        #[arg(long, default_value = "full")]
        solana_mode: String,

        /// Program ID for Solana PDA mode (base58)
        #[arg(long, default_value = "")]
        program_id: String,

        /// Specific indexes to derive (comma-separated). Overrides --index and --num.
        #[arg(long)]
        indexes: Option<String>,
    },

    Decrypt {
        #[arg(required = true)]
        file: String,

        #[arg(short, long)]
        output: Option<String>,

        #[arg(long)]
        password: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
struct KeyInfo {
    index: u32,
    path: String,
    xprv: Option<String>,
    xpub: Option<String>,
    private_key: String,
    public_key: String,
    address: String,
    wif: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct WalletOutput {
    mnemonic: String,
    passphrase: String,
    chain: String,
    master_xprv: Option<String>,
    master_xpub: Option<String>,
    keys: Vec<KeyInfo>,
}

#[derive(Serialize, Deserialize)]
struct EncryptedWallet {
    version: u32,
    salt: String,
    nonce: String,
    ciphertext: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gen {
            mnemonic,
            passphrase,
            chain,
            index,
            account,
            change,
            num,
            strength,
            json,
            output,
            qr,
            encrypt,
            password,
            hw_sim,
            xpub,
            xpub_path,
            solana_mode,
            program_id,
            indexes,
        } => {
            let chain_lower = chain.to_lowercase();
            let base_path = get_default_path(&chain_lower, account, change, hw_sim);
            let quiet = json || output.is_some();

            if let Some(xpub_str) = xpub {
                let xpub_base =
                    xpub_path.unwrap_or_else(|| base_path.trim_end_matches("/0").to_string());
                let result =
                    generate_from_xpub(&xpub_str, &xpub_base, index, num, &chain_lower, qr, quiet)?;
                handle_output(result, json, output, encrypt, password)?;
            } else {
                let mnemonic_obj = get_or_generate_mnemonic(mnemonic, strength, json)?;
                let seed = mnemonic_obj.to_seed(&passphrase);

                let result = generate_for_chain(
                    &seed,
                    &base_path,
                    index,
                    num,
                    &mnemonic_obj,
                    &passphrase,
                    &chain_lower,
                    qr,
                    quiet,
                    &solana_mode,
                    &program_id,
                    &indexes,
                )?;

                handle_output(result, json, output, encrypt, password)?;
            }
        }
        Commands::Decrypt {
            file,
            output,
            password,
        } => {
            decrypt_wallet(&file, output, password)?;
        }
    }
    Ok(())
}

fn get_default_path(chain: &str, account: u32, change: u32, _hw_sim: bool) -> String {
    match chain {
        "evm" | "ethereum" => format!("m/44'/60'/{account}'/{change}/0"),
        "btc" | "bitcoin" => format!("m/44'/0'/{account}'/{change}/0"),
        "solana" => format!("m/44'/501'/{account}'/{change}'"),
        _ => format!("m/44'/60'/{account}'/{change}/0"),
    }
}

fn get_or_generate_mnemonic(
    mnemonic: Option<String>,
    strength: u32,
    quiet: bool,
) -> Result<Mnemonic> {
    match mnemonic {
        Some(m) => Mnemonic::parse_in(Language::English, m).context("Invalid mnemonic"),
        None => {
            let word_count = if strength == 24 { 24 } else { 12 };
            let m = Mnemonic::generate_in(Language::English, word_count)
                .context("Failed to generate mnemonic")?;
            if !quiet {
                println!(
                    "{} {}",
                    "=== NEW MNEMONIC GENERATED ===".yellow().bold(),
                    "SAVE THIS SECURELY!".red().bold()
                );
                println!("{}", m.to_string().bright_cyan());
            }
            Ok(m)
        }
    }
}

fn is_ed25519_chain(chain: &str) -> bool {
    matches!(chain, "solana")
}

fn derive_slip10_ed25519(seed: &[u8], path: &[u32]) -> [u8; 64] {
    use hmac::{KeyInit, Mac};
    let mut mac = <hmac::Hmac<sha2::Sha512>>::new_from_slice(b"ed25519 seed")
        .expect("HMAC accepts any key length");
    mac.update(seed);
    let mut i = mac.finalize().into_bytes();

    for &idx in path {
        let mut mac = <hmac::Hmac<sha2::Sha512>>::new_from_slice(&i[32..])
            .expect("HMAC accepts any key length");
        mac.update(&[0u8]);
        mac.update(&i[..32]);
        mac.update(&idx.to_be_bytes());
        i = mac.finalize().into_bytes();
    }
    let mut res = [0u8; 64];
    res.copy_from_slice(&i);
    res
}

fn parse_path(path_str: &str) -> Result<Vec<u32>> {
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
            indexes.push(num + HARDENED);
        } else {
            let num: u32 = part.parse().context("Invalid path segment")?;
            indexes.push(num);
        }
    }
    Ok(indexes)
}

#[allow(clippy::too_many_arguments)]
fn print_solana_mode_info(mode: &str) {
    match mode {
        "full" => println!(
            "{}",
            "⚠️  FULL MODE - Private keys exposed (High Risk)"
                .red()
                .bold()
        ),
        "hsm-sim" => println!("{}", "🛡️  HSM Simulation Mode".cyan()),
        "cold-export" => println!(
            "{}",
            "🔒 COLD-EXPORT - Only public addresses (Recommended)"
                .green()
                .bold()
        ),
        "pda" => println!(
            "{}",
            "📍 PDA MODE - Can RECEIVE SOL but cannot sweep (controlled by program)"
                .bright_purple()
                .bold()
        ),
        _ => {}
    }
}

fn parse_indexes(indexes_str: &str) -> Result<Vec<u32>> {
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
fn generate_for_chain(
    seed: &[u8],
    base_path: &str,
    specific_index: Option<u32>,
    num: u32,
    mnemonic: &Mnemonic,
    bip39_pass: &str,
    chain: &str,
    show_qr: bool,
    quiet: bool,
    solana_mode: &str,
    program_id: &str,
    indexes: &Option<String>,
) -> Result<WalletOutput> {
    if !quiet {
        println!(
            "\n{}",
            format!("=== {} ===", chain.to_uppercase()).blue().bold()
        );
        if chain == "solana" {
            print_solana_mode_info(solana_mode);
        }
    }

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

        if !quiet {
            print_key_info(&info, show_qr);
        }
        keys.push(info);
    }

    let mut wallet = build_output(mnemonic, bip39_pass, chain, keys);

    let account_path = base_path.trim_end_matches(|c: char| c.is_numeric() || c == '/');
    if is_ed25519_chain(chain) {
        if let Ok(indexes) = parse_path(account_path) {
            let derived = derive_slip10_ed25519(seed, &indexes);
            let chain_code = &derived[32..];
            let sk_bytes: [u8; 32] = derived[..32].try_into().unwrap();
            let signing_key = SigningKey::from_bytes(&sk_bytes);
            let pubkey_bytes = signing_key.verifying_key().to_bytes();
            let mut xpub = chain_code.to_vec();
            xpub.extend_from_slice(&pubkey_bytes);
            wallet.master_xpub = Some(hex::encode(xpub));
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

fn build_xpub(
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

fn parse_xpub(xpub_str: &str) -> Result<bitcoin::bip32::Xpub> {
    use std::str::FromStr;

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

fn generate_from_xpub(
    xpub_str: &str,
    base_path: &str,
    specific_index: Option<u32>,
    num: u32,
    chain: &str,
    show_qr: bool,
    quiet: bool,
) -> Result<WalletOutput> {
    if !quiet {
        println!("\n{}", "=== WATCH-ONLY (xpub mode) ===".yellow().bold());
        println!("Using xpub: {}", xpub_str);
        println!("Derivation path: {}/*\n", base_path.trim_end_matches('/'));
    }

    let count = specific_index.map_or(num, |_| 1);
    let mut keys = vec![];

    if is_ed25519_chain(chain) {
        anyhow::bail!(
            "xpub mode is not supported for {} (Ed25519 curve).\n\
             Ed25519 uses hardened derivation which requires the private key.\n\
             \n\
             Recommendations:\n\
             \x20  Instead, pre-generate addresses from the seed (offline):\n\
             \x20    xgen gen --chain {} --mnemonic \"your phrase\" --num 100 --json\n\
             \x20  Export the public keys to your hot server. No private keys exposed.",
            chain,
            chain
        );
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
                let pubkey = bitcoin::PublicKey::new(child.public_key);
                bitcoin::Address::p2pkh(pubkey, bitcoin::NetworkKind::Main).to_string()
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

        if !quiet {
            print_key_info(&info, show_qr);
        }
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

fn build_derivation_path(base: &str, index: u32, chain: &str) -> String {
    if is_ed25519_chain(chain) {
        let base = base.trim_end_matches('\'');
        format!("{}/{}'", base, index)
    } else {
        let base = base.trim_end_matches(|c: char| c.is_numeric() || c == '/');
        format!("{}/{}", base, index)
    }
}

// ==================== Chain Implementations ====================

fn derive_secp_key(seed: &[u8], path: &str) -> Result<Xpriv> {
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

fn generate_evm(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
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

fn generate_bitcoin(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let child = derive_secp_key(seed, path)?;

    let priv_key = child.to_priv();
    let pub_key = priv_key.public_key(&secp);
    let address = Address::p2pkh(pub_key, bitcoin::NetworkKind::Main);
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

fn generate_solana(
    seed: &[u8],
    path: &str,
    idx: u32,
    mode: &str,
    program_id: &str,
) -> Result<KeyInfo> {
    let path_indexes = parse_path(path)?;
    let derived = derive_slip10_ed25519(seed, &path_indexes);
    let sk_bytes: [u8; 32] = derived[..32].try_into().unwrap();

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

fn eth_address(pubkey_bytes: &[u8]) -> String {
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

    let mut checksum = String::with_capacity(42);
    checksum.push_str("0x");
    for (i, c) in addr.chars().enumerate() {
        let n = u8::from_str_radix(&hash[i..=i], 16).unwrap_or(0);
        checksum.push(if n > 7 { c.to_ascii_uppercase() } else { c });
    }
    checksum
}

fn print_key_info(info: &KeyInfo, show_qr: bool) {
    println!("\n{}", format!("Index {}", info.index).yellow().bold());
    println!("Path      : {}", info.path);
    if let Some(x) = &info.xprv {
        println!("xprv      : {}", x);
    }
    if let Some(x) = &info.xpub {
        println!("xpub      : {}", x);
    }
    println!("Private   : {}", info.private_key);
    if let Some(w) = &info.wif {
        println!("WIF       : {}", w);
    }
    println!("Address   : {}", info.address.bright_green());

    if show_qr {
        let _ = print_qr(&info.address);
    }
}

fn build_output(mn: &Mnemonic, pass: &str, chain: &str, keys: Vec<KeyInfo>) -> WalletOutput {
    WalletOutput {
        mnemonic: mn.to_string(),
        passphrase: pass.to_string(),
        chain: chain.to_string(),
        master_xprv: None,
        master_xpub: None,
        keys,
    }
}

// ==================== Encryption / Decryption ====================

fn handle_output(
    wallet: WalletOutput,
    json: bool,
    output: Option<String>,
    encrypt_cmd: Option<String>,
    cli_password: Option<String>,
) -> Result<()> {
    let data = serde_json::to_string_pretty(&wallet)?;

    let password = encrypt_cmd.or(cli_password);

    if let Some(pass) = password {
        let encrypted = encrypt_data(&data, &pass)?;
        if let Some(file) = output {
            fs::write(&file, encrypted)?;
            println!(
                "{} Encrypted wallet saved -> {}",
                "LOCK".green().bold(),
                file
            );
        } else {
            println!("{}", encrypted);
        }
    } else if json || output.is_some() {
        if let Some(file) = output {
            fs::write(&file, data)?;
            println!("{} Wallet saved -> {}", "OK".green().bold(), file);
        } else {
            println!("{}", data);
        }
    }
    Ok(())
}

fn encrypt_data(data: &str, password: &str) -> Result<String> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit};
    use base64::Engine;
    use scrypt::scrypt;

    let salt: [u8; 16] = rand::random();
    let mut key = [0u8; 32];
    let params = scrypt::Params::new(15, 8, 1).context("Invalid scrypt params")?;

    scrypt(password.as_bytes(), &salt, &params, &mut key)
        .context("Scrypt key derivation failed")?;

    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow::anyhow!("AES init failed: {:?}", e))?;
    let nonce: [u8; 12] = rand::random();
    let ciphertext = cipher
        .encrypt(&nonce.into(), data.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {:?}", e))?;

    let engine = base64::engine::general_purpose::STANDARD;
    let enc = EncryptedWallet {
        version: 1,
        salt: engine.encode(salt),
        nonce: engine.encode(nonce),
        ciphertext: engine.encode(ciphertext),
    };

    serde_json::to_string_pretty(&enc).context("Failed to serialize encrypted data")
}

fn decrypt_wallet(file: &str, output: Option<String>, cli_pass: Option<String>) -> Result<()> {
    let content = fs::read_to_string(file).context("Failed to read encrypted file")?;
    let enc: EncryptedWallet =
        serde_json::from_str(&content).context("Invalid encrypted wallet format")?;

    let password = match cli_pass {
        Some(p) => p,
        None => rpassword::prompt_password("Enter decryption password: ")
            .context("Failed to read password")?,
    };

    let decrypted = decrypt_data(&enc, &password)?;
    let wallet: WalletOutput = serde_json::from_str(&decrypted)?;

    if let Some(out_file) = output {
        fs::write(&out_file, serde_json::to_string_pretty(&wallet)?)?;
        println!(
            "{} Decrypted successfully -> {}",
            "OK".green().bold(),
            out_file
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&wallet)?);
    }
    Ok(())
}

fn decrypt_data(enc: &EncryptedWallet, password: &str) -> Result<String> {
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
    let params = scrypt::Params::new(15, 8, 1)?;
    scrypt(password.as_bytes(), &salt, &params, &mut key)?;

    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let plaintext = cipher
        .decrypt(&nonce_arr.into(), ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Decryption failed. Wrong password?"))?;

    Ok(String::from_utf8(plaintext)?)
}
