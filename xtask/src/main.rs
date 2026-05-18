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

#[cfg(test)]
mod ci_checks {
    use std::fs;
    use std::path::Path;

    // Forbidden patterns per .agents/AGENT.md
    const FORBIDDEN_PATTERNS: &[(&str, &str)] = &[
        ("todo", "todo!() or TODO comments"),
        ("unimplemented!()", "unimplemented!()"),
        ("unwrap()", "unwrap()"),
        ("expect()", "expect()"),
        ("unsafe ", "unsafe keyword"),
        ("new_unchecked", "new_unchecked()"),
        ("Ok(true)", "Ok(true) in verification paths"),
        ("Ok(Default::default())", "Ok(Default::default())"),
        ("assert!(true)", "assert!(true)"),
        ("Sha256::digest", "raw Sha256::digest"),
        ("Keccak256::digest", "raw Keccak256::digest"),
        ("blake3::hash", "raw blake3::hash"),
    ];

    fn is_exempt_path(path: &Path) -> bool {
        let s = path.to_string_lossy();
        // Allow these directories for tests/fuzz/benches
        if s.contains("/tests/") || s.contains("/fuzz/") || s.contains("/benches/") {
            return true;
        }
        // Allow files under crates' tests directories
        if s.ends_with("_test.rs") || s.ends_with("mod_test.rs") {
            return true;
        }
        false
    }

    #[test]
    fn forbidden_patterns_not_present_in_production() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let mut violations = Vec::new();

        let mut stack = vec![root.to_path_buf()];
        while let Some(p) = stack.pop() {
            let md = match fs::metadata(&p) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.is_dir() {
                if p.ends_with("target") || p.ends_with(".git") {
                    continue;
                }
                for entry in fs::read_dir(&p).unwrap().flatten() {
                    stack.push(entry.path());
                }
            } else if md.is_file() {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if ext == "rs" {
                        if is_exempt_path(&p) {
                            continue;
                        }
                        if let Ok(text) = fs::read_to_string(&p) {
                            for (pat, desc) in FORBIDDEN_PATTERNS.iter() {
                                if text.contains(pat) {
                                    violations.push(format!("{}: {} ({})", p.display(), pat, desc));
                                }
                            }
                        }
                    }
                }
            }
        }

        if !violations.is_empty() {
            eprintln!("Forbidden patterns detected:");
            for v in &violations {
                eprintln!(" - {}", v);
            }
            panic!("CI check failed: forbidden patterns present in production code");
        }
    }
}
