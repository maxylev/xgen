use anyhow::{Context, Result};
use bip39::{Language, Mnemonic};
use bitcoin::{Address, Network, PrivateKey};
use clap::Parser;
use colored::*;
use ed25519_dalek::SigningKey;
use hd_wallet::HdWallet;
use qr2term::print_qr;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::fs;

#[derive(Parser)]
#[command(author, version, about = "xgen - Multi-chain HD Wallet CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    Gen {
        #[arg(short, long)]
        mnemonic: Option<String>,

        #[arg(short = 's', long, default_value = "")]
        passphrase: String,

        #[arg(short, long, default_value = "evm")]
        chain: String,

        #[arg(short, long)]
        index: Option<u32>,

        #[arg(long, default_value_t = 0)]
        account: u32,

        #[arg(long, default_value_t = 0)]
        change: u32,

        #[arg(short, long, default_value_t = 3)]
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
        } => {
            let mnemonic_obj = get_or_generate_mnemonic(mnemonic, strength, json)?;
            let seed = mnemonic_obj.to_seed(&passphrase);

            let chain_lower = chain.to_lowercase();
            let base_path = get_default_path(&chain_lower, account, change, hw_sim);

            let quiet = json || output.is_some();
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
            )?;

            handle_output(result, json, output, encrypt, password)?;
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
        "evm" | "ethereum" => "m/44'/60'/0'/0/0".to_string(),
        "btc" | "bitcoin" => "m/44'/0'/0'/0/0".to_string(),
        "solana" => format!("m/44'/501'/{account}'/{change}'"),
        "ton" | "telegram" => format!("m/44'/607'/{account}'/{change}'"),
        "cardano" | "ada" => format!("m/1852'/1815'/{account}'/0/0"),
        "monero" | "xmr" => format!("m/44'/128'/{account}'/0/0"),
        "doge" | "dogecoin" => "m/44'/3'/0'/0/0".to_string(),
        "xrp" | "ripple" => "m/44'/144'/0'/0/0".to_string(),
        _ => "m/44'/60'/0'/0/0".to_string(),
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
    matches!(
        chain,
        "solana" | "ton" | "telegram" | "xrp" | "ripple" | "cardano" | "ada" | "monero" | "xmr"
    )
}

fn derive_master_key_pair_ed25519(
    seed: &[u8],
) -> hd_wallet::ExtendedKeyPair<hd_wallet::curves::Ed25519> {
    use generic_ec::{Scalar, SecretScalar};
    use hd_wallet::{ChainCode, ExtendedSecretKey};

    use hmac::{KeyInit, Mac};
    let mut mac = <hmac::Hmac<sha2::Sha512>>::new_from_slice(b"ed25519 seed")
        .expect("HMAC accepts any key length");
    mac.update(seed);
    let result = mac.finalize().into_bytes();

    let mut chain: ChainCode = [0u8; 32];
    chain.copy_from_slice(&result[32..]);

    let mut scalar = Scalar::<hd_wallet::curves::Ed25519>::from_be_bytes_mod_order(&result[..32]);
    let secret_key = SecretScalar::new(&mut scalar);

    ExtendedSecretKey {
        secret_key,
        chain_code: chain,
    }
    .into()
}

fn parse_path(path_str: &str) -> Vec<u32> {
    let trimmed = path_str
        .strip_prefix("m/")
        .or_else(|| path_str.strip_prefix("m"))
        .unwrap_or(path_str);
    if trimmed.is_empty() {
        return vec![];
    }
    let mut indexes = Vec::new();
    for part in trimmed.split('/') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(num_str) = part.strip_suffix('\'') {
            let num: u32 = num_str.parse().unwrap_or(0);
            indexes.push(num + hd_wallet::H);
        } else {
            let num: u32 = part.parse().unwrap_or(0);
            indexes.push(num);
        }
    }
    indexes
}

fn scalar_to_32_bytes<E: generic_ec::Curve>(scalar: &generic_ec::SecretScalar<E>) -> [u8; 32] {
    let encoded = scalar.as_ref().to_be_bytes();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(encoded.as_ref());
    arr
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
) -> Result<WalletOutput> {
    if !quiet {
        println!(
            "\n{}",
            format!("=== {} ===", chain.to_uppercase()).blue().bold()
        );
    }

    let count = specific_index.map_or(num, |_| 1);
    let mut keys = vec![];

    for i in 0..count {
        let idx = specific_index.unwrap_or(i);
        let path = build_derivation_path(base_path, idx, chain);

        let info = match chain {
            "evm" | "ethereum" => generate_evm(seed, &path, idx)?,
            "btc" | "bitcoin" => generate_bitcoin(seed, &path, idx)?,
            "solana" => generate_solana(seed, &path, idx)?,
            "ton" | "telegram" => generate_ton(seed, &path, idx)?,
            "doge" | "dogecoin" => generate_doge(seed, &path, idx)?,
            "xrp" | "ripple" => generate_xrp(seed, &path, idx)?,
            "cardano" | "ada" => generate_cardano(seed, &path, idx)?,
            "monero" | "xmr" => generate_monero(seed, &path, idx)?,
            _ => anyhow::bail!("Chain not supported yet"),
        };

        if !quiet {
            print_key_info(&info, show_qr);
        }
        keys.push(info);
    }

    let mut wallet = build_output(mnemonic, bip39_pass, chain, keys);
    wallet.master_xprv = Some("Master key hidden for security".to_string());
    Ok(wallet)
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

fn generate_evm(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    use hd_wallet::curves::Secp256k1;

    let master_sk = hd_wallet::slip10::derive_master_key::<Secp256k1>(seed)?;
    let key_pair = hd_wallet::ExtendedKeyPair::from(master_sk);
    let child = hd_wallet::Slip10::derive_child_key_pair_with_path(&key_pair, parse_path(path));

    let pk_point = child.public_key().public_key;
    let pub_bytes = pk_point.to_bytes(false);
    let address = eth_address(&pub_bytes);

    let sk_bytes = scalar_to_32_bytes(&child.secret_key().secret_key);

    let mut xprv_data = child.secret_key().chain_code.to_vec();
    xprv_data.extend_from_slice(&sk_bytes);
    let mut xpub_data = child.public_key().chain_code.to_vec();
    xpub_data.extend_from_slice(&pub_bytes);
    let xprv = format!("xprv{}", hex::encode(xprv_data));
    let xpub = format!("xpub{}", hex::encode(xpub_data));

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: Some(xprv),
        xpub: Some(xpub),
        private_key: format!("0x{}", hex::encode(sk_bytes)),
        public_key: format!("0x{}", hex::encode(&pub_bytes)),
        address,
        wif: None,
    })
}

fn generate_bitcoin(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    use hd_wallet::curves::Secp256k1;

    let secp = bitcoin::secp256k1::Secp256k1::new();
    let master_sk = hd_wallet::slip10::derive_master_key::<Secp256k1>(seed)?;
    let key_pair = hd_wallet::ExtendedKeyPair::from(master_sk);
    let child = hd_wallet::Slip10::derive_child_key_pair_with_path(&key_pair, parse_path(path));

    let sk_bytes = scalar_to_32_bytes(&child.secret_key().secret_key);

    let secret_key = bitcoin::secp256k1::SecretKey::from_slice(&sk_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid secret key: {:?}", e))?;
    let priv_key = PrivateKey::new(secret_key, Network::Bitcoin);
    let pub_key = priv_key.public_key(&secp);
    let address = Address::p2pkh(pub_key, Network::Bitcoin);
    let wif = priv_key.to_wif();

    let pk_point = child.public_key().public_key;
    let pub_bytes = pk_point.to_bytes(true);

    let mut xprv_data = child.secret_key().chain_code.to_vec();
    xprv_data.extend_from_slice(&sk_bytes);
    let mut xpub_data = child.public_key().chain_code.to_vec();
    xpub_data.extend_from_slice(&pub_bytes);
    let xprv = format!("xprv{}", hex::encode(xprv_data));
    let xpub = format!("xpub{}", hex::encode(xpub_data));

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

fn generate_solana(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    let key_pair = derive_master_key_pair_ed25519(seed);
    let child = hd_wallet::Edwards::derive_child_key_pair_with_path(&key_pair, parse_path(path));

    let sk_bytes = scalar_to_32_bytes(&child.secret_key().secret_key);

    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();
    let pubkey = Pubkey::new_from_array(verifying_key.to_bytes());

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: Some(hex::encode(sk_bytes)),
        xpub: Some(pubkey.to_string()),
        private_key: hex::encode(sk_bytes),
        public_key: hex::encode(verifying_key.to_bytes()),
        address: pubkey.to_string(),
        wif: None,
    })
}

fn generate_ton(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    let key_pair = derive_master_key_pair_ed25519(seed);
    let child = hd_wallet::Edwards::derive_child_key_pair_with_path(&key_pair, parse_path(path));

    let sk_bytes = scalar_to_32_bytes(&child.secret_key().secret_key);

    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    let encoded = bs58::encode(verifying_key.to_bytes()).into_string();
    let address = format!("EQ{}", &encoded[..encoded.len().min(48)]);

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: None,
        xpub: None,
        private_key: hex::encode(sk_bytes),
        public_key: hex::encode(verifying_key.to_bytes()),
        address,
        wif: None,
    })
}

fn generate_doge(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    use hd_wallet::curves::Secp256k1;

    let master_sk = hd_wallet::slip10::derive_master_key::<Secp256k1>(seed)?;
    let key_pair = hd_wallet::ExtendedKeyPair::from(master_sk);
    let child = hd_wallet::Slip10::derive_child_key_pair_with_path(&key_pair, parse_path(path));

    let pk_point = child.public_key().public_key;
    let pk_bytes = pk_point.to_bytes(true);
    let address = format!("D{}", hex::encode(&pk_bytes[..pk_bytes.len().min(20)]));

    let sk_bytes = scalar_to_32_bytes(&child.secret_key().secret_key);

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: None,
        xpub: None,
        private_key: hex::encode(sk_bytes),
        public_key: hex::encode(pk_bytes),
        address,
        wif: None,
    })
}

fn generate_xrp(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    let key_pair = derive_master_key_pair_ed25519(seed);
    let child = hd_wallet::Edwards::derive_child_key_pair_with_path(&key_pair, parse_path(path));

    let sk_bytes = scalar_to_32_bytes(&child.secret_key().secret_key);

    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    let encoded = bs58::encode(verifying_key.to_bytes()).into_string();
    let address = format!("r{}", &encoded[..encoded.len().min(33)]);

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: None,
        xpub: None,
        private_key: hex::encode(sk_bytes),
        public_key: hex::encode(verifying_key.to_bytes()),
        address,
        wif: None,
    })
}

fn generate_cardano(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    let key_pair = derive_master_key_pair_ed25519(seed);
    let child = hd_wallet::Edwards::derive_child_key_pair_with_path(&key_pair, parse_path(path));

    let sk_bytes = scalar_to_32_bytes(&child.secret_key().secret_key);

    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    let encoded = bs58::encode(verifying_key.to_bytes()).into_string();
    let address = format!("addr1{}", &encoded[..encoded.len().min(50)]);

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: None,
        xpub: None,
        private_key: hex::encode(sk_bytes),
        public_key: hex::encode(verifying_key.to_bytes()),
        address,
        wif: None,
    })
}

fn generate_monero(seed: &[u8], path: &str, idx: u32) -> Result<KeyInfo> {
    let key_pair = derive_master_key_pair_ed25519(seed);
    let child = hd_wallet::Edwards::derive_child_key_pair_with_path(&key_pair, parse_path(path));

    let sk_bytes = scalar_to_32_bytes(&child.secret_key().secret_key);

    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    let encoded = bs58::encode(verifying_key.to_bytes()).into_string();
    let address = format!("4{}", &encoded[..encoded.len().min(94)]);

    Ok(KeyInfo {
        index: idx,
        path: path.to_string(),
        xprv: None,
        xpub: None,
        private_key: hex::encode(sk_bytes),
        public_key: hex::encode(verifying_key.to_bytes()),
        address,
        wif: None,
    })
}

fn eth_address(pubkey_bytes: &[u8]) -> String {
    use tiny_keccak::{Hasher, Keccak};
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]);
    hasher.finalize(&mut output);

    let addr = hex::encode(&output[12..]);
    let hash = hex::encode(output);
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
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
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
    #[allow(deprecated)]
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), data.as_bytes())
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
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::Engine;
    use scrypt::scrypt;

    let engine = base64::engine::general_purpose::STANDARD;

    let salt = engine.decode(&enc.salt).context("Invalid salt")?;
    let nonce = engine.decode(&enc.nonce).context("Invalid nonce")?;
    let ciphertext = engine
        .decode(&enc.ciphertext)
        .context("Invalid ciphertext")?;

    let mut key = [0u8; 32];
    let params = scrypt::Params::new(15, 8, 1)?;
    scrypt(password.as_bytes(), &salt, &params, &mut key)?;

    let cipher = Aes256Gcm::new_from_slice(&key)?;
    #[allow(deprecated)]
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Decryption failed. Wrong password?"))?;

    Ok(String::from_utf8(plaintext)?)
}
