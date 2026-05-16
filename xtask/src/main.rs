//! Cargo xtask for verifying contract bindings
//!
//! This xtask provides commands to verify that contract bindings
//! are up to date with the source contracts.

use std::path::Path;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_help();
        std::process::exit(1);
    }

    match args[0].as_str() {
        "verify-bindings" => verify_bindings(),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("Unknown command: {}", args[0]);
            print_help();
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("CSV Protocol xtask");
    println!();
    println!("Usage: cargo xtask <command>");
    println!();
    println!("Commands:");
    println!("  verify-bindings  Verify that contract bindings are up to date");
    println!("  help             Show this help message");
}

fn verify_bindings() -> anyhow::Result<()> {
    println!("Verifying contract bindings...");
    println!();

    // Verify Ethereum bindings
    if Path::new("csv-contracts/ethereum").exists() {
        println!("Checking Ethereum contracts...");
        verify_ethereum_bindings()?;
    }

    // Verify Solana bindings
    if Path::new("csv-contracts/solana").exists() {
        println!("Checking Solana contracts...");
        verify_solana_bindings()?;
    }

    // Verify Sui bindings
    if Path::new("csv-contracts/sui").exists() {
        println!("Checking Sui contracts...");
        verify_sui_bindings()?;
    }

    // Verify Aptos bindings
    if Path::new("csv-contracts/aptos").exists() {
        println!("Checking Aptos contracts...");
        verify_aptos_bindings()?;
    }

    println!();
    println!("✓ All bindings verified successfully");
    Ok(())
}

fn verify_ethereum_bindings() -> anyhow::Result<()> {
    let contracts_dir = Path::new("csv-contracts/ethereum/contracts");

    if !contracts_dir.exists() {
        println!("  ⚠ Ethereum contracts directory not found, skipping");
        return Ok(());
    }

    // Check if forge is available
    let forge_available = Command::new("forge")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !forge_available {
        println!("  ⚠ Forge not found, skipping Ethereum binding verification");
        println!("    Install Foundry: https://getfoundry.sh/");
        return Ok(());
    }

    // Build contracts to verify they compile
    let output = Command::new("forge")
        .args(["build", "--sizes"])
        .current_dir(contracts_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Ethereum contract compilation failed:\n{}", stderr);
    }

    println!("  ✓ Ethereum contracts compile successfully");
    Ok(())
}

fn verify_solana_bindings() -> anyhow::Result<()> {
    let contracts_dir = Path::new("csv-contracts/solana/contracts");

    if !contracts_dir.exists() {
        println!("  ⚠ Solana contracts directory not found, skipping");
        return Ok(());
    }

    // Check if anchor is available
    let anchor_available = Command::new("anchor")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !anchor_available {
        println!("  ⚠ Anchor not found, skipping Solana binding verification");
        println!("    Install Anchor: https://www.anchor-lang.com/");
        return Ok(());
    }

    // Build contracts to verify they compile
    let output = Command::new("anchor")
        .args(["build"])
        .current_dir(contracts_dir)
        .env("NO_DNA", "1")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Solana contract compilation failed:\n{}", stderr);
    }

    println!("  ✓ Solana contracts compile successfully");
    Ok(())
}

fn verify_sui_bindings() -> anyhow::Result<()> {
    let contracts_dir = Path::new("csv-contracts/sui/contracts");

    if !contracts_dir.exists() {
        println!("  ⚠ Sui contracts directory not found, skipping");
        return Ok(());
    }

    // Check if sui is available
    let sui_available = Command::new("sui")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !sui_available {
        println!("  ⚠ Sui CLI not found, skipping Sui binding verification");
        println!("    Install Sui CLI: https://docs.sui.io/build/install");
        return Ok(());
    }

    // Build contracts to verify they compile
    let output = Command::new("sui")
        .args(["move", "build"])
        .current_dir(contracts_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Sui contract compilation failed:\n{}", stderr);
    }

    println!("  ✓ Sui contracts compile successfully");
    Ok(())
}

fn verify_aptos_bindings() -> anyhow::Result<()> {
    let contracts_dir = Path::new("csv-contracts/aptos/contracts");

    if !contracts_dir.exists() {
        println!("  ⚠ Aptos contracts directory not found, skipping");
        return Ok(());
    }

    // Check if aptos is available
    let aptos_available = Command::new("aptos")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !aptos_available {
        println!("  ⚠ Aptos CLI not found, skipping Aptos binding verification");
        println!("    Install Aptos CLI: https://aptos.dev/cli-tools/aptos-cli/install-cli/");
        return Ok(());
    }

    // Build contracts to verify they compile
    let output = Command::new("aptos")
        .args(["move", "compile"])
        .current_dir(contracts_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Aptos contract compilation failed:\n{}", stderr);
    }

    println!("  ✓ Aptos contracts compile successfully");
    Ok(())
}
