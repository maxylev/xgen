use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use qr2term::print_qr;
use std::fs;

use xgen::{
    decrypt_data, encrypt_data, generate_for_chain, generate_from_xpriv, generate_from_xpub,
    get_default_path, get_or_generate_mnemonic, EncryptedWallet, KeyInfo, WalletOutput,
};

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
        encrypt: bool,

        #[arg(long)]
        password: Option<String>,

        #[arg(long)]
        hw_sim: bool,

        #[arg(long)]
        xpub: Option<String>,

        #[arg(long)]
        xpub_path: Option<String>,

        #[arg(long)]
        xpriv: Option<String>,

        #[arg(long)]
        xpriv_path: Option<String>,

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
            xpriv,
            xpriv_path,
            solana_mode,
            program_id,
            indexes,
        } => {
            let chain_lower = chain.to_lowercase();
            let base_path = get_default_path(&chain_lower, account, change, hw_sim);
            let quiet = json || output.is_some();

            let wallet = if let Some(xpriv_str) = xpriv {
                let xpriv_base =
                    xpriv_path.unwrap_or_else(|| base_path.trim_end_matches("/0").to_string());

                if !quiet {
                    println!("\n{}", "=== DERIVING FROM xpriv ===".yellow().bold());
                    println!("Using xpriv: {}", xpriv_str);
                    println!("Derivation path: {}/*\n", xpriv_base.trim_end_matches('/'));
                    if chain_lower == "solana" {
                        print_solana_mode_info(&solana_mode);
                    }
                }

                let wallet_out = generate_from_xpriv(
                    &xpriv_str,
                    &xpriv_base,
                    index,
                    num,
                    &chain_lower,
                    &solana_mode,
                    &program_id,
                    &indexes,
                )?;

                if !quiet {
                    for key in &wallet_out.keys {
                        print_key_info(key, qr);
                    }
                }
                wallet_out
            } else if let Some(xpub_str) = xpub {
                let xpub_base =
                    xpub_path.unwrap_or_else(|| base_path.trim_end_matches("/0").to_string());

                if !quiet {
                    println!("\n{}", "=== WATCH-ONLY (xpub mode) ===".yellow().bold());
                    println!("Using xpub: {}", xpub_str);
                    println!("Derivation path: {}/*\n", xpub_base.trim_end_matches('/'));
                }

                let wallet_out =
                    generate_from_xpub(&xpub_str, &xpub_base, index, num, &chain_lower)?;

                if !quiet {
                    for key in &wallet_out.keys {
                        print_key_info(key, qr);
                    }
                }
                wallet_out
            } else {
                let is_imported = mnemonic.is_some();
                let mnemonic_obj = get_or_generate_mnemonic(mnemonic, strength)?;

                if !quiet && !is_imported {
                    println!(
                        "{} {}",
                        "=== NEW MNEMONIC GENERATED ===".yellow().bold(),
                        "SAVE THIS SECURELY!".red().bold()
                    );
                    println!("{}", mnemonic_obj.to_string().bright_cyan());
                }

                let seed = mnemonic_obj.to_seed(&passphrase);

                if !quiet {
                    println!(
                        "\n{}",
                        format!("=== {} ===", chain_lower.to_uppercase())
                            .blue()
                            .bold()
                    );
                    if chain_lower == "solana" {
                        print_solana_mode_info(&solana_mode);
                    }
                }

                let wallet_out = generate_for_chain(
                    &seed,
                    &base_path,
                    index,
                    num,
                    &mnemonic_obj,
                    &passphrase,
                    &chain_lower,
                    &solana_mode,
                    &program_id,
                    &indexes,
                )?;

                if !quiet {
                    for key in &wallet_out.keys {
                        print_key_info(key, qr);
                    }
                }
                wallet_out
            };

            let resolved_password = match (encrypt, password) {
                (true, Some(pass)) => Some(pass),
                (true, None) => {
                    let pass = rpassword::prompt_password("Enter encryption password: ")
                        .context("Failed to read password")?;
                    let confirm = rpassword::prompt_password("Confirm encryption password: ")
                        .context("Failed to read password confirmation")?;
                    if pass != confirm {
                        anyhow::bail!("Passwords do not match");
                    }
                    Some(pass)
                }
                (false, Some(pass)) => Some(pass),
                (false, None) => None,
            };
            let should_encrypt = encrypt || resolved_password.is_some();

            handle_output(wallet, json, output, should_encrypt, resolved_password)?;
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

fn handle_output(
    wallet: WalletOutput,
    json: bool,
    output: Option<String>,
    encrypt: bool,
    cli_password: Option<String>,
) -> Result<()> {
    let data = serde_json::to_string_pretty(&wallet)?;

    if encrypt {
        let pass = cli_password.context("No password provided for encryption")?;
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

fn decrypt_wallet(file: &str, output: Option<String>, cli_pass: Option<String>) -> Result<()> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("Failed to read encrypted file from '{}'", file))?;

    let enc: EncryptedWallet = serde_json::from_str(&content)
        .context("Corrupted or invalid encrypted wallet envelope format")?;

    let password = match cli_pass {
        Some(p) => p,
        None => rpassword::prompt_password("Enter decryption password: ")
            .context("Failed to securely read password from TTY")?,
    };

    let decrypted = decrypt_data(&enc, &password)
        .context("Failed to decrypt wallet payload. Please verify your password.")?;

    let wallet: WalletOutput = serde_json::from_str(&decrypted).context(
        "Decrypted successfully, but wallet payload contains invalid JSON metadata structure",
    )?;

    if let Some(out_file) = output {
        fs::write(&out_file, serde_json::to_string_pretty(&wallet)?)
            .with_context(|| format!("Failed to write decrypted wallet to '{}'", out_file))?;
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
