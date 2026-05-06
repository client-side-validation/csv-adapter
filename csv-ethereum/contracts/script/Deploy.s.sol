// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/CSVLock.sol";
import "../src/CSVMint.sol";

/// @title Deploy — Deploy CSVLock and CSVMint on Sepolia testnet
/// @notice Run with: forge script script/Deploy.s.sol --rpc-url $SEPOLIA_RPC_URL --private-key $DEPLOYER_KEY --broadcast --verify
contract Deploy is Script {
    function run() external returns (address lockAddr, address mintAddr) {
        uint256 deployerKey = vm.envUint("DEPLOYER_KEY");
        address deployer = vm.addr(deployerKey);

        console.log("Deployer:", deployer);
        console.log("Balance:", deployer.balance);

        vm.startBroadcast(deployerKey);

        // Step 1: Deploy CSVLock (no constructor args)
        CSVLock lock = new CSVLock();
        console.log("CSVLock deployed at:", address(lock));

        // Step 2: Deploy CSVMint with lock contract address
        // Verifier is initially deployer (can be updated later)
        CSVMint mint = new CSVMint(address(lock), deployer);
        console.log("CSVMint deployed at:", address(mint));

        // Step 3: Wire up mint contract in lock contract
        lock.setMintContract(address(mint));
        console.log("CSVLock.setMintContract called");

        // Step 4: Verify the wiring
        require(lock.mintContract() == address(mint), "Mint contract not wired");
        require(mint.lockContract() == address(lock), "Lock contract not set");

        vm.stopBroadcast();

        // Output for CI/state.json parsing
        console.log("\n=== DEPLOYMENT SUMMARY ===");
        console.log("CSVLock:", address(lock));
        console.log("CSVMint:", address(mint));
        console.log("Network: Sepolia (chainId 11155111)");
        console.log("==========================\n");

        lockAddr = address(lock);
        mintAddr = address(mint);
    }
}
