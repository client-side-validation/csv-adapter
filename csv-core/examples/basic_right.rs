//! Example: Creating and transferring a Right
//!
//! This example demonstrates the basic lifecycle of a Right:
//! 1. Creating a Right with an initial owner
//! 2. Transferring it to a new owner
//! 3. Verifying the Right's integrity

use csv_adapter_core::{Hash, OwnershipProof, Right, SignatureScheme};

fn main() {
    println!("=== CSV Adapter Core: Basic Right Example ===\n");

    // Step 1: Create a commitment (represents the Right's content/state)
    let commitment = Hash::new([0xAB; 32]);
    println!(
        "1. Created commitment: {}",
        hex::encode(commitment.as_bytes())
    );

    // Step 2: Create an initial ownership proof
    // In a real scenario, this would contain a cryptographic signature
    let initial_owner = OwnershipProof {
        proof: vec![0x01; 64], // Simulated signature (64 bytes)
        owner: vec![0x02; 33], // Simulated public key (33 bytes, compressed)
        scheme: Some(SignatureScheme::Secp256k1),
    };

    // Step 3: Create a Right with unique salt
    let salt = b"genesis-salt-2026";
    let right = Right::new(commitment, initial_owner, salt);

    println!(
        "2. Created Right with ID: {}",
        hex::encode(right.id.0.as_bytes())
    );
    println!(
        "   Commitment: {}",
        hex::encode(right.commitment.as_bytes())
    );
    println!("   Owner: {}", hex::encode(&right.owner.owner));
    println!("   Salt: {}", String::from_utf8_lossy(&right.salt));

    // Step 4: Transfer to a new owner
    let new_owner = OwnershipProof {
        proof: vec![0x03; 64], // New simulated signature
        owner: vec![0x04; 33], // New simulated public key
        scheme: Some(SignatureScheme::Secp256k1),
    };

    let transfer_salt = b"transfer-salt-2026";
    let transferred_right = right.transfer(new_owner.clone(), transfer_salt);

    println!("\n3. Transferred Right:");
    println!(
        "   New Right ID: {}",
        hex::encode(transferred_right.id.0.as_bytes())
    );
    println!(
        "   New Owner: {}",
        hex::encode(&transferred_right.owner.owner)
    );
    println!(
        "   New Salt: {}",
        String::from_utf8_lossy(&transferred_right.salt)
    );

    println!("\n=== Example Complete ===");
}
