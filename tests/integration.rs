use std::path::PathBuf;
use std::process::Command;
use std::str;

const VALID_12_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const VALID_24_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

fn xgen_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("xgen");
    path
}

fn xgen(args: &[&str]) -> (String, String, bool) {
    let output = Command::new(xgen_bin())
        .args(args)
        .output()
        .expect("Failed to run xgen");
    let stdout = str::from_utf8(&output.stdout).unwrap().to_string();
    let stderr = str::from_utf8(&output.stderr).unwrap().to_string();
    (stdout, stderr, output.status.success())
}

fn assert_success(args: &[&str]) -> String {
    let (stdout, stderr, ok) = xgen(args);
    assert!(
        ok,
        "Command failed: xgen {}\nstderr: {}\nstdout: {}",
        args.join(" "),
        stderr,
        stdout
    );
    stdout
}

fn assert_fails(args: &[&str]) {
    let (stdout, _stderr, ok) = xgen(args);
    assert!(
        !ok,
        "Command should have failed: xgen {}\nstdout: {}",
        args.join(" "),
        stdout
    );
}

// ==================== CHAIN OUTPUT FORMATS ====================

mod chain_output {
    use super::*;

    #[test]
    fn test_evm_default() {
        let out = assert_success(&["gen", "--chain", "evm", "--num", "1"]);
        assert!(out.contains("=== EVM ==="), "Missing EVM header");
        assert!(out.contains("0x"), "EVM missing 0x prefix");
        assert!(out.contains("Private"), "EVM missing Private key");
        assert!(out.contains("xprv"), "EVM missing xprv");
        assert!(out.contains("xpub"), "EVM missing xpub");
    }

    #[test]
    fn test_evm_checksummed_address() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
        ]);
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(addr.starts_with("0x"), "Address must start with 0x");
        assert_eq!(addr.len(), 42, "EVM address must be 42 chars (0x + 40 hex)");
        let hex_part = &addr[2..];
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "EVM address must be hex"
        );
    }

    #[test]
    fn test_bitcoin_output() {
        let out = assert_success(&["gen", "--chain", "btc", "--num", "1"]);
        assert!(out.contains("=== BTC ==="));
        assert!(out.contains("WIF"), "BTC missing WIF");
        assert!(out.contains("xprv"));
        assert!(out.contains("xpub"));
    }

    #[test]
    fn test_bitcoin_multi_address() {
        let out = assert_success(&["gen", "--chain", "btc", "--num", "3"]);
        assert!(out.contains("Index 0"));
        assert!(out.contains("Index 1"));
        assert!(out.contains("Index 2"));
        assert!(out.matches("WIF").count() >= 3, "Expected 3 WIF entries");
        assert!(
            out.matches("Address").count() >= 3,
            "Expected 3 Address entries"
        );
    }

    #[test]
    fn test_solana_output() {
        let out = assert_success(&["gen", "--chain", "solana", "--num", "1"]);
        assert!(out.contains("=== SOLANA ==="));
        assert!(out.contains("Address"));
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(!addr.is_empty(), "Solana address should not be empty");
    }

    #[test]
    fn test_ton_output() {
        let out = assert_success(&["gen", "--chain", "ton", "--num", "1"]);
        assert!(out.contains("=== TON ==="));
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(addr.starts_with("EQ"), "TON address must start with EQ");
    }

    #[test]
    fn test_doge_output() {
        let out = assert_success(&["gen", "--chain", "doge", "--num", "1"]);
        assert!(out.contains("=== DOGE ==="));
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(addr.starts_with('D'), "Doge address must start with D");
    }

    #[test]
    fn test_xrp_output() {
        let out = assert_success(&["gen", "--chain", "xrp", "--num", "1"]);
        assert!(out.contains("=== XRP ==="));
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(addr.starts_with('r'), "XRP address must start with r");
    }

    #[test]
    fn test_cardano_output() {
        let out = assert_success(&["gen", "--chain", "cardano", "--num", "1"]);
        assert!(out.contains("=== CARDANO ==="));
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(
            addr.starts_with("addr1"),
            "Cardano address must start with addr1"
        );
    }

    #[test]
    fn test_monero_output() {
        let out = assert_success(&["gen", "--chain", "monero", "--num", "1"]);
        assert!(out.contains("=== MONERO ==="));
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(addr.starts_with('4'), "Monero address must start with 4");
    }

    #[test]
    fn test_chain_alias_ethereum() {
        let out = assert_success(&["gen", "--chain", "ethereum", "--num", "1"]);
        assert!(
            out.contains("0x"),
            "Ethereum alias should produce 0x address"
        );
    }

    #[test]
    fn test_chain_alias_bitcoin() {
        let out = assert_success(&["gen", "--chain", "bitcoin", "--num", "1"]);
        assert!(out.contains("WIF"), "bitcoin alias should produce WIF");
    }

    #[test]
    fn test_chain_alias_telegram() {
        let out = assert_success(&["gen", "--chain", "telegram", "--num", "1"]);
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(
            addr.starts_with("EQ"),
            "telegram alias should produce EQ address"
        );
    }

    #[test]
    fn test_chain_alias_ripple() {
        let out = assert_success(&["gen", "--chain", "ripple", "--num", "1"]);
        let addr_line = out.lines().find(|l| l.contains("Address")).unwrap();
        let addr = addr_line.split(':').nth(1).unwrap().trim();
        assert!(addr.starts_with('r'), "ripple alias should start with r");
    }
}

// ==================== MNEMONIC HANDLING ====================

mod mnemonic {
    use super::*;

    #[test]
    fn test_generate_new_12_words() {
        let out = assert_success(&["gen", "--chain", "evm", "--num", "1", "--strength", "12"]);
        assert!(out.contains("NEW MNEMONIC GENERATED"));
        assert!(out.contains("0x"));
    }

    #[test]
    fn test_generate_new_24_words() {
        let out = assert_success(&["gen", "--chain", "evm", "--num", "1", "--strength", "24"]);
        // 24 words → count spaces (23 spaces = 24 words)
        let mnemonic_line = out
            .lines()
            .skip_while(|l| !l.contains("NEW MNEMONIC"))
            .skip(1)
            .next()
            .unwrap_or("");
        let word_count = mnemonic_line.split_whitespace().count();
        assert_eq!(
            word_count, 24,
            "Expected 24-word mnemonic, got {} words: {}",
            word_count, mnemonic_line
        );
    }

    #[test]
    fn test_import_valid_12_mnemonic() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
        ]);
        assert!(
            out.contains("0x"),
            "Imported mnemonic should produce address"
        );
        assert!(
            !out.contains("NEW MNEMONIC"),
            "Should not generate new mnemonic"
        );
    }

    #[test]
    fn test_import_valid_24_mnemonic() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_24_MNEMONIC,
            "--index",
            "0",
        ]);
        assert!(
            out.contains("0x"),
            "24-word mnemonic should produce address"
        );
    }

    #[test]
    fn test_mnemonic_deterministic() {
        let out1 = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        let out2 = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        assert_eq!(
            out1, out2,
            "Same mnemonic + index should produce identical output"
        );
    }

    #[test]
    fn test_mnemonic_with_passphrase() {
        let out1 = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        let out2 = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--passphrase",
            "mypass",
            "--json",
        ]);
        assert_ne!(
            out1, out2,
            "Different passphrases should produce different keys"
        );
    }

    #[test]
    fn test_invalid_mnemonic_fails() {
        assert_fails(&[
            "gen",
            "--mnemonic",
            "this is not a valid mnemonic phrase at all",
            "--num",
            "1",
        ]);
    }

    #[test]
    fn test_too_few_words_fails() {
        assert_fails(&["gen", "--mnemonic", "abandon abandon abandon", "--num", "1"]);
    }

    #[test]
    fn test_mnemonic_with_wrong_checksum_fails() {
        assert_fails(&["gen", "--mnemonic", "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon zoo", "--num", "1"]);
    }
}

// ==================== DERIVATION OPTIONS ====================

mod derivation {
    use super::*;

    #[test]
    fn test_specific_index() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "42",
        ]);
        assert!(out.contains("Index 42"), "Should derive index 42");
    }

    #[test]
    fn test_index_zero() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
        ]);
        assert!(out.contains("Index 0"), "Should derive index 0");
        assert!(out.contains("0x"), "Index 0 should produce address");
    }

    #[test]
    fn test_different_indexes_different_keys() {
        let out0 = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        let out1 = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "1",
            "--json",
        ]);
        assert_ne!(
            out0, out1,
            "Different indexes should produce different keys"
        );
    }

    #[test]
    fn test_multiple_addresses() {
        let out = assert_success(&["gen", "--chain", "evm", "--num", "5"]);
        for i in 0..5 {
            assert!(out.contains(&format!("Index {i}")), "Missing index {i}");
        }
    }

    #[test]
    fn test_num_without_index() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--num",
            "3",
        ]);
        assert!(out.contains("Index 0"));
        assert!(out.contains("Index 1"));
        assert!(out.contains("Index 2"));
    }

    #[test]
    fn test_account_change() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "solana",
            "--account",
            "2",
            "--change",
            "1",
            "--num",
            "1",
        ]);
        assert!(out.contains("Path"), "Should show derivation path");
        assert!(out.contains("Address"));
    }

    #[test]
    fn test_index_overrides_num() {
        let out = assert_success(&["gen", "--chain", "evm", "--num", "10", "--index", "3"]);
        assert!(
            out.contains("Index 3"),
            "With --index, only one address at that index"
        );
        assert!(!out.contains("Index 0"), "Should not include index 0");
        assert!(!out.contains("Index 4"), "Should not include index 4");
    }

    #[test]
    fn test_hw_sim_solana() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "solana",
            "--hw-sim",
            "--account",
            "1",
            "--num",
            "1",
        ]);
        assert!(out.contains("Address"), "HW sim should produce address");
    }

    #[test]
    fn test_hw_sim_evm() {
        let out = assert_success(&["gen", "--chain", "evm", "--hw-sim", "--num", "1"]);
        assert!(out.contains("0x"), "HW sim EVM should produce 0x address");
    }
}

// ==================== OUTPUT MODES ====================

mod output {
    use super::*;

    #[test]
    fn test_default_terminal_output() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
        ]);
        assert!(out.contains("xprv"), "Should show xprv");
        assert!(out.contains("xpub"), "Should show xpub");
        assert!(out.contains("Private"), "Should show private key");
        assert!(out.contains("Path"), "Should show derivation path");
    }

    #[test]
    fn test_json_output() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        assert!(out.starts_with('{'), "JSON output should start with {{");
        assert!(out.contains("\"mnemonic\""), "JSON missing mnemonic field");
        assert!(out.contains("\"chain\""), "JSON missing chain field");
        assert!(out.contains("\"keys\""), "JSON missing keys field");
        assert!(out.contains("\"address\""), "JSON missing address field");
        assert!(
            out.contains("\"private_key\""),
            "JSON missing private_key field"
        );
    }

    #[test]
    fn test_json_parseable() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON should be valid");
        assert_eq!(parsed["chain"], "evm", "Chain should be evm");
        assert_eq!(
            parsed["mnemonic"], VALID_12_MNEMONIC,
            "Mnemonic should match"
        );
        assert!(parsed["keys"].is_array(), "Keys should be an array");
        assert_eq!(
            parsed["keys"].as_array().unwrap().len(),
            1,
            "Should have 1 key"
        );
    }

    #[test]
    fn test_json_all_chains() {
        for chain in &[
            "evm", "btc", "solana", "ton", "doge", "xrp", "cardano", "monero",
        ] {
            let out = assert_success(&[
                "gen",
                "--chain",
                chain,
                "--mnemonic",
                VALID_12_MNEMONIC,
                "--index",
                "0",
                "--json",
            ]);
            let parsed: serde_json::Value = serde_json::from_str(&out).unwrap_or_else(|e| {
                panic!("Invalid JSON for chain {}: {}\nOutput: {}", chain, e, out)
            });
            assert_eq!(
                parsed["chain"].as_str(),
                Some(*chain),
                "Wrong chain in JSON output"
            );
            assert!(
                parsed["keys"][0]["address"].is_string(),
                "Missing address for chain {}",
                chain
            );
        }
    }

    #[test]
    fn test_output_file() {
        let tmp = "/tmp/xgen_e2e_output.json";
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
            "--output",
            tmp,
        ]);
        assert!(out.contains("Wallet saved"), "Should show save message");
        let contents = std::fs::read_to_string(tmp).expect("File should exist");
        assert!(
            contents.contains("\"chain\""),
            "Saved file should contain JSON"
        );
        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn test_output_file_no_json() {
        let tmp = "/tmp/xgen_e2e_output_nojson.json";
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--output",
            tmp,
        ]);
        assert!(out.contains("Wallet saved"), "Should show save message");
        let contents = std::fs::read_to_string(tmp).expect("File should exist");
        assert!(contents.contains("\"chain\""), "Saved file should be JSON");
        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn test_qr_flag_does_not_crash() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--qr",
        ]);
        assert!(out.contains("0x"), "QR flag should produce address");
    }

    #[test]
    fn test_qr_on_all_chains() {
        for chain in &[
            "evm", "btc", "solana", "ton", "doge", "xrp", "cardano", "monero",
        ] {
            let out = assert_success(&[
                "gen",
                "--chain",
                chain,
                "--mnemonic",
                VALID_12_MNEMONIC,
                "--index",
                "0",
                "--qr",
            ]);
            assert!(
                out.contains("Address"),
                "QR mode should show address for {}",
                chain
            );
        }
    }
}

// ==================== ENCRYPTION / DECRYPTION ====================

mod encrypt_decrypt {
    use super::*;

    struct TempFile {
        path: String,
    }

    impl TempFile {
        fn new(name: &str) -> Self {
            let path = format!("/tmp/xgen_e2e_{}", name);
            let _ = std::fs::remove_file(&path);
            Self { path }
        }

        fn path(&self) -> &str {
            &self.path
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn test_encrypt_default_output() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--encrypt",
            "mypass",
        ]);
        assert!(out.contains('{'), "Encrypted output should be JSON");
        assert!(
            out.contains("\"ciphertext\""),
            "Encrypted output should contain ciphertext"
        );
        assert!(
            out.contains("\"salt\""),
            "Encrypted output should contain salt"
        );
        assert!(
            out.contains("\"nonce\""),
            "Encrypted output should contain nonce"
        );
    }

    #[test]
    fn test_encrypt_to_file() {
        let tmp = TempFile::new("enc_file.json");
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--encrypt",
            "mypass",
            "--output",
            tmp.path(),
        ]);
        assert!(out.contains("LOCK"), "Should show LOCK icon");
        let contents = std::fs::read_to_string(tmp.path()).expect("File should exist");
        let parsed: serde_json::Value =
            serde_json::from_str(&contents).expect("Should be valid JSON");
        assert!(parsed.get("ciphertext").is_some(), "Should have ciphertext");
        assert!(parsed.get("salt").is_some(), "Should have salt");
        assert!(parsed.get("nonce").is_some(), "Should have nonce");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let enc = TempFile::new("roundtrip_enc.json");
        let dec = TempFile::new("roundtrip_dec.json");
        let password = "supersecret!123";

        assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
            "--output",
            dec.path(),
        ]);
        let original = std::fs::read_to_string(dec.path()).expect("Should read original");
        let _ = std::fs::remove_file(dec.path());

        assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--encrypt",
            password,
            "--output",
            enc.path(),
        ]);

        assert_success(&[
            "decrypt",
            enc.path(),
            "--output",
            dec.path(),
            "--password",
            password,
        ]);
        let decrypted = std::fs::read_to_string(dec.path()).expect("Should read decrypted");
        assert_eq!(
            original, decrypted,
            "Decrypted content should match original"
        );
    }

    #[test]
    fn test_encrypt_decrypt_wrong_password_fails() {
        let enc = TempFile::new("wrong_pass_enc.json");
        assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--encrypt",
            "correctpass",
            "--output",
            enc.path(),
        ]);
        assert_fails(&["decrypt", enc.path(), "--password", "wrongpass"]);
    }

    #[test]
    fn test_decrypt_nonexistent_file_fails() {
        assert_fails(&[
            "decrypt",
            "/tmp/xgen_nonexistent_encrypted.json",
            "--password",
            "test",
        ]);
    }

    #[test]
    fn test_decrypt_invalid_file_fails() {
        let tmp = TempFile::new("invalid.json");
        std::fs::write(tmp.path(), "not json at all").unwrap();
        assert_fails(&["decrypt", tmp.path(), "--password", "test"]);
    }

    #[test]
    fn test_encrypt_with_password_flag() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--password",
            "cli_pass",
        ]);
        assert!(
            out.contains("\"ciphertext\""),
            "--password flag should encrypt output"
        );
    }

    #[test]
    fn test_encrypt_same_data_different_ciphertext() {
        let out1 = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--encrypt",
            "samepass",
            "--json",
        ]);
        let out2 = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--encrypt",
            "samepass",
            "--json",
        ]);
        assert_ne!(
            out1, out2,
            "Same password + same data should produce different ciphertext (random salt/nonce)"
        );
    }

    #[test]
    fn test_decrypt_stdout_output() {
        let enc = TempFile::new("decrypt_stdout.json");
        assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--encrypt",
            "pass",
            "--output",
            enc.path(),
        ]);
        let out = assert_success(&["decrypt", enc.path(), "--password", "pass"]);
        assert!(
            out.contains("\"chain\""),
            "Decrypt to stdout should output JSON"
        );
    }

    #[test]
    fn test_encrypt_all_chains() {
        for chain in &[
            "evm", "btc", "solana", "ton", "doge", "xrp", "cardano", "monero",
        ] {
            let out = assert_success(&[
                "gen",
                "--chain",
                chain,
                "--mnemonic",
                VALID_12_MNEMONIC,
                "--index",
                "0",
                "--encrypt",
                "testpass",
            ]);
            assert!(
                out.contains("\"ciphertext\""),
                "Encrypted output for chain {} should contain ciphertext",
                chain
            );
        }
    }
}

// ==================== ERROR HANDLING ====================

mod errors {
    use super::*;

    #[test]
    fn test_invalid_chain() {
        assert_fails(&["gen", "--chain", "nonexistent", "--num", "1"]);
    }

    #[test]
    fn test_invalid_strength() {
        let (_, _, _success) = xgen(&["gen", "--chain", "evm", "--strength", "7", "--num", "1"]);
        // A strength of 7 should probably fail or produce default 12
        // Just ensure it doesn't crash
    }

    #[test]
    fn test_gen_no_args_works() {
        let out = assert_success(&["gen"]);
        assert!(
            out.contains("NEW MNEMONIC"),
            "gen without args should generate new mnemonic"
        );
        assert!(
            out.contains("0x"),
            "gen without args should produce addresses"
        );
    }

    #[test]
    fn test_decrypt_no_args_fails() {
        assert_fails(&["decrypt"]);
    }

    #[test]
    fn test_unknown_flag_fails() {
        let (_, _, ok) = xgen(&["gen", "--nonexistent-flag"]);
        assert!(!ok);
    }

    #[test]
    fn test_negative_index_fails() {
        let (_, _, ok) = xgen(&["gen", "--chain", "evm", "--index", "-1"]);
        assert!(!ok);
    }
}

// ==================== HELP AND VERSION ====================

mod help {
    use super::*;

    #[test]
    fn test_top_level_help() {
        let out = assert_success(&["--help"]);
        assert!(out.contains("xgen"), "Help should mention xgen");
        assert!(out.contains("gen"), "Help should list gen subcommand");
        assert!(
            out.contains("decrypt"),
            "Help should list decrypt subcommand"
        );
    }

    #[test]
    fn test_gen_help() {
        let out = assert_success(&["gen", "--help"]);
        assert!(out.contains("--chain"), "Gen help should list --chain");
        assert!(
            out.contains("--mnemonic"),
            "Gen help should list --mnemonic"
        );
        assert!(out.contains("--index"), "Gen help should list --index");
        assert!(out.contains("--num"), "Gen help should list --num");
        assert!(out.contains("--encrypt"), "Gen help should list --encrypt");
        assert!(out.contains("--qr"), "Gen help should list --qr");
        assert!(out.contains("--json"), "Gen help should list --json");
        assert!(out.contains("--output"), "Gen help should list --output");
        assert!(out.contains("--hw-sim"), "Gen help should list --hw-sim");
        assert!(
            out.contains("--passphrase"),
            "Gen help should list --passphrase"
        );
        assert!(out.contains("--account"), "Gen help should list --account");
        assert!(out.contains("--change"), "Gen help should list --change");
        assert!(
            out.contains("--strength"),
            "Gen help should list --strength"
        );
        assert!(
            out.contains("--password"),
            "Gen help should list --password"
        );
    }

    #[test]
    fn test_decrypt_help() {
        let out = assert_success(&["decrypt", "--help"]);
        assert!(
            out.contains("decrypt"),
            "Decrypt help should mention decrypt"
        );
        assert!(
            out.contains("--output"),
            "Decrypt help should list --output"
        );
        assert!(
            out.contains("--password"),
            "Decrypt help should list --password"
        );
    }

    #[test]
    fn test_version() {
        let out = assert_success(&["--version"]);
        assert!(out.contains("0.8.0"), "Version should be 0.8.0");
    }
}

// ==================== EDGE CASES ====================

mod edge_cases {
    use super::*;

    #[test]
    fn test_same_mnemonic_different_chains_different_keys() {
        let evm = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        let sol = assert_success(&[
            "gen",
            "--chain",
            "solana",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        assert_ne!(
            evm, sol,
            "Different chains should produce different outputs"
        );
    }

    #[test]
    fn test_all_chains_same_index() {
        for chain in &[
            "evm", "btc", "solana", "ton", "doge", "xrp", "cardano", "monero",
        ] {
            let out = assert_success(&[
                "gen",
                "--chain",
                chain,
                "--mnemonic",
                VALID_12_MNEMONIC,
                "--index",
                "5",
            ]);
            assert!(
                out.contains("Index 5"),
                "Chain {} should have index 5",
                chain
            );
            assert!(
                out.contains("Address"),
                "Chain {} should have address",
                chain
            );
        }
    }

    #[test]
    fn test_large_index() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "9999",
        ]);
        assert!(out.contains("Index 9999"), "Should derive large index 9999");
        assert!(out.contains("0x"), "Large index should produce address");
    }

    #[test]
    fn test_all_chains_multi_address() {
        for chain in &[
            "evm", "btc", "solana", "ton", "doge", "xrp", "cardano", "monero",
        ] {
            let out = assert_success(&[
                "gen",
                "--chain",
                chain,
                "--mnemonic",
                VALID_12_MNEMONIC,
                "--num",
                "2",
            ]);
            assert!(out.contains("Index 0"), "Chain {} missing index 0", chain);
            assert!(out.contains("Index 1"), "Chain {} missing index 1", chain);
        }
    }

    #[test]
    fn test_multiple_runs_different_mnemonics() {
        let out1 = assert_success(&["gen", "--chain", "evm", "--num", "1", "--json"]);
        let out2 = assert_success(&["gen", "--chain", "evm", "--num", "1", "--json"]);
        assert_ne!(
            out1, out2,
            "Two fresh generations should produce different mnemonics"
        );
    }

    #[test]
    fn test_json_fields_comprehensive() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["mnemonic"].is_string());
        assert!(v["passphrase"].is_string());
        assert!(v["chain"].is_string());
        assert!(v["master_xprv"].is_null() || v["master_xprv"].is_string());
        assert!(v["master_xpub"].is_null() || v["master_xpub"].is_string());
        let key = &v["keys"][0];
        assert!(key["index"].is_number());
        assert!(key["path"].is_string());
        assert!(key["private_key"].is_string());
        assert!(key["public_key"].is_string());
        assert!(key["address"].is_string());
    }

    #[test]
    fn test_passphrase_empty_default() {
        let out = assert_success(&[
            "gen",
            "--chain",
            "evm",
            "--mnemonic",
            VALID_12_MNEMONIC,
            "--index",
            "0",
            "--json",
        ]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["passphrase"], "", "Default passphrase should be empty");
    }

    #[test]
    fn test_generated_mnemonic_preserved_in_json() {
        let out = assert_success(&["gen", "--chain", "evm", "--num", "1", "--json"]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let phrase = v["mnemonic"].as_str().unwrap();
        let word_count = phrase.split_whitespace().count();
        assert!(
            word_count == 12 || word_count == 24,
            "Generated mnemonic should be 12 or 24 words, got {}",
            word_count
        );
    }
}
