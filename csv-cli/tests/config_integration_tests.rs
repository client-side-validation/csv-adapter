//! Integration tests for configuration handling
//!
//! These tests verify the CLI correctly handles various config scenarios
//! including edge cases that could cause runtime errors.

use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

/// Test that CLI handles missing config gracefully by creating defaults
#[test]
fn test_cli_creates_default_config_when_missing() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("new_config.toml");
    
    // Run a simple command with a non-existent config
    let output = Command::new("cargo")
        .args([
            "run", "--package", "csv-cli", "--bin", "csv", "--",
            "--config", config_path.to_str().unwrap(),
            "chain", "list"
        ])
        .output()
        .expect("Failed to execute command");
    
    // Command should succeed
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Config file should have been created
    assert!(config_path.exists(), "Config file should be created. stderr: {}, stdout: {}", stderr, stdout);
    
    // Should indicate config creation
    assert!(
        stderr.contains("Created default config") || stdout.contains("bitcoin") || stdout.contains("ethereum"),
        "Should create default config or show chain list. stderr: {}, stdout: {}", stderr, stdout
    );
}

/// Test CLI handles legacy config format (wallet-only) without crashing
#[test]
fn test_cli_handles_legacy_wallet_only_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("legacy_config.toml");
    
    // Create a legacy-style config with only wallet section (the bug scenario)
    let legacy_config = r#"
[wallet]
mnemonic = "test test test test test test test test test test test junk"
network = "dev"

[wallet.bitcoin]
address = "bcrt1legacy"

[wallet.ethereum]
address = "0xlegacy"
"#;
    
    let mut file = std::fs::File::create(&config_path).unwrap();
    file.write_all(legacy_config.as_bytes()).unwrap();
    drop(file);
    
    // Run chain list command - should NOT panic
    let output = Command::new("cargo")
        .args([
            "run", "--package", "csv-cli", "--bin", "csv", "--",
            "--config", config_path.to_str().unwrap(),
            "chain", "list"
        ])
        .output()
        .expect("Failed to execute command");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should NOT contain "missing field `chains`" error
    assert!(
        !stderr.contains("missing field `chains`"),
        "Should not error about missing chains field. stderr: {}", stderr
    );
    
    // Should complete without TOML parse errors
    assert!(
        !stderr.contains("TOML parse error"),
        "Should not have TOML parse errors. stderr: {}", stderr
    );
    
    // Should show the chain list (even if empty, shows headers)
    assert!(
        stdout.contains("Chain") || stdout.contains("Network") || stderr.contains("Created default"),
        "Should show chain list or create default. stdout: {}, stderr: {}", stdout, stderr
    );
}

/// Test CLI handles empty config file
#[test]
fn test_cli_handles_empty_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("empty_config.toml");
    
    // Create an empty config file
    std::fs::File::create(&config_path).unwrap();
    
    // Run chain list command
    let output = Command::new("cargo")
        .args([
            "run", "--package", "csv-cli", "--bin", "csv", "--",
            "--config", config_path.to_str().unwrap(),
            "chain", "list"
        ])
        .output()
        .expect("Failed to execute command");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should not error about TOML parsing
    assert!(
        !stderr.contains("TOML parse error"),
        "Should not error on empty config. stderr: {}", stderr
    );
}

/// Test CLI handles invalid TOML gracefully
#[test]
fn test_cli_handles_invalid_toml_gracefully() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid_config.toml");
    
    // Create an invalid TOML file
    let invalid_toml = r#"
[chains.bitcoin
rpc_url = "missing bracket"
"#;
    
    let mut file = std::fs::File::create(&config_path).unwrap();
    file.write_all(invalid_toml.as_bytes()).unwrap();
    drop(file);
    
    // Run chain list command
    let output = Command::new("cargo")
        .args([
            "run", "--package", "csv-cli", "--bin", "csv", "--",
            "--config", config_path.to_str().unwrap(),
            "chain", "list"
        ])
        .output()
        .expect("Failed to execute command");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should show an error message (but not panic)
    assert!(
        !output.status.success() || stderr.contains("Error") || stderr.contains("TOML"),
        "Should report error for invalid TOML, not crash silently"
    );
}

/// Test that CLI can load and use a valid custom config
#[test]
fn test_cli_loads_valid_custom_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("custom_config.toml");
    
    // Create a valid custom config
    let custom_config = r#"
[chains.bitcoin]
rpc_url = "https://custom.bitcoin.rpc.com"
network = "test"
finality_depth = 3

[chains.ethereum]
rpc_url = "https://custom.ethereum.rpc.com"
network = "main"
finality_depth = 12
chain_id = 1

[wallets.bitcoin]
mnemonic = "test test test test test test test test test test test junk"
"#;
    
    let mut file = std::fs::File::create(&config_path).unwrap();
    file.write_all(custom_config.as_bytes()).unwrap();
    drop(file);
    
    // Run chain status command for bitcoin
    let output = Command::new("cargo")
        .args([
            "run", "--package", "csv-cli", "--bin", "csv", "--",
            "--config", config_path.to_str().unwrap(),
            "chain", "status", "--chain", "bitcoin"
        ])
        .output()
        .expect("Failed to execute command");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should NOT error about TOML
    assert!(
        !stderr.contains("TOML parse error"),
        "Should parse valid config. stderr: {}", stderr
    );
    
    // Should show bitcoin in output
    assert!(
        stdout.to_lowercase().contains("bitcoin") || stderr.to_lowercase().contains("bitcoin"),
        "Should reference bitcoin chain. stdout: {}, stderr: {}", stdout, stderr
    );
}

/// Test config with partial data (some chains missing fields)
#[test]
fn test_cli_handles_partial_chain_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("partial_config.toml");
    
    // Create config with only some fields
    let partial_config = r#"
[chains.bitcoin]
rpc_url = "https://bitcoin.example.com"
network = "test"
finality_depth = 6
# Missing: contract_address, chain_id, default_fee

[chains.sui]
rpc_url = "https://sui.example.com"
network = "dev"
finality_depth = 1
# Missing: contract_address, chain_id, default_fee
"#;
    
    let mut file = std::fs::File::create(&config_path).unwrap();
    file.write_all(partial_config.as_bytes()).unwrap();
    drop(file);
    
    // Run chain list
    let output = Command::new("cargo")
        .args([
            "run", "--package", "csv-cli", "--bin", "csv", "--",
            "--config", config_path.to_str().unwrap(),
            "chain", "list"
        ])
        .output()
        .expect("Failed to execute command");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should not error
    assert!(
        !stderr.contains("TOML parse error"),
        "Should handle partial config. stderr: {}", stderr
    );
    
    // Should show configured chains
    assert!(
        stdout.to_lowercase().contains("bitcoin") || stdout.to_lowercase().contains("sui"),
        "Should show configured chains. stdout: {}", stdout
    );
}

/// Test that default config is used when no config path specified
/// This uses the default ~/.csv/config.toml path
#[test]
#[ignore = "modifies ~/.csv directory - run manually"]
fn test_cli_uses_default_config_path() {
    // This test is ignored by default because it modifies the user's home directory
    // Run manually with: cargo test --test config_integration_tests test_cli_uses_default_config_path -- --ignored
    
    // First, backup existing config if present
    let home = dirs::home_dir().expect("Home dir not found");
    let csv_dir = home.join(".csv");
    let config_path = csv_dir.join("config.toml");
    let backup_path = csv_dir.join("config.toml.bak");
    
    let had_existing = config_path.exists();
    if had_existing {
        std::fs::copy(&config_path, &backup_path).unwrap();
    }
    
    // Remove any existing config to force creation
    if config_path.exists() {
        std::fs::remove_file(&config_path).unwrap();
    }
    
    // Run command to trigger default config creation
    let output = Command::new("cargo")
        .args([
            "run", "--package", "csv-cli", "--bin", "csv", "--",
            "chain", "list"
        ])
        .output()
        .expect("Failed to execute command");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should have created default config
    assert!(config_path.exists(), "Default config should be created at ~/.csv/config.toml");
    assert!(stderr.contains("Created default config"), "Should report config creation");
    
    // Cleanup - restore backup if existed
    if had_existing {
        std::fs::copy(&backup_path, &config_path).unwrap();
        std::fs::remove_file(&backup_path).unwrap();
    }
}
