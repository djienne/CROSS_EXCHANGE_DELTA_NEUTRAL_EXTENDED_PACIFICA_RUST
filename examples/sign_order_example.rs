/// Example: Signing orders for Extended DEX using Python SDK integration
///
/// This example demonstrates the recommended production approach:
/// Using the Python SDK via subprocess to ensure 100% compatibility
/// with Extended's signature format.
///
/// Run with: cargo run --example sign_order_example

use serde_json::json;
use std::io::Write;
use std::process::{Command, Stdio};

fn sign_order_python(
    base_asset_id: &str,
    quote_asset_id: &str,
    base_amount: i128,
    quote_amount: i128,
    fee_amount: u128,
    position_id: u64,
    nonce: u64,
    expiration_epoch_millis: u64,
    public_key: &str,
    private_key: &str,
    domain_chain_id: &str,
) -> Result<(String, String), String> {
    // Build input JSON
    let input_json = json!({
        "base_asset_id": base_asset_id,
        "quote_asset_id": quote_asset_id,
        "fee_asset_id": quote_asset_id,  // Fee asset is always quote asset
        "base_amount": base_amount.to_string(),
        "quote_amount": quote_amount.to_string(),
        "fee_amount": fee_amount.to_string(),
        "position_id": position_id.to_string(),
        "nonce": nonce.to_string(),
        "expiration_epoch_millis": expiration_epoch_millis.to_string(),
        "public_key": public_key,
        "private_key": private_key,
        "domain_name": "Perpetuals",
        "domain_version": "v0",
        "domain_chain_id": domain_chain_id,
        "domain_revision": "1",
    });

    let input_str = serde_json::to_string(&input_json)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    // Call Python script
    let mut child = Command::new("python")
        .arg("scripts/sign_order.py")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn Python: {}", e))?;

    // Write input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input_str.as_bytes())
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
    }

    // Wait for output
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for Python: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python signing failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse Python output: {}\nOutput: {}", e, stdout))?;

    let r = result["r"]
        .as_str()
        .ok_or("Missing r in Python output")?
        .to_string();
    let s = result["s"]
        .as_str()
        .ok_or("Missing s in Python output")?
        .to_string();

    Ok((r, s))
}

fn main() {
    println!("🔐 Extended DEX Order Signing Example\n");

    // Example: BUY order for 0.1 SOL at ~$16.23
    let base_asset_id = "0x534f4c2d33"; // SOL-3
    let quote_asset_id = "0x1"; // USDC
    let base_amount = 100i128; // Positive for BUY
    let quote_amount = -16229000i128; // Negative for BUY (paying USDC)
    let fee_amount = 9738u128;
    let position_id = 226109u64;
    let nonce = 1234567890u64;
    let expiration_epoch_millis = 1700000000000u64;

    // Example keys (replace with real keys in production!)
    let public_key = "0x338f4cb92453dfb7c7764549d85ab624e6614db51b4c25c0fd63da09f07d127";
    let private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abc";

    println!("Order Parameters:");
    println!("  Base Asset: {} (SOL)", base_asset_id);
    println!("  Quote Asset: {} (USDC)", quote_asset_id);
    println!("  Side: BUY (base_amount positive)");
    println!("  Amount: {} base units", base_amount);
    println!("  Price: {} quote units", quote_amount.abs());
    println!("  Fee: {} units", fee_amount);
    println!("  Position ID: {}", position_id);
    println!("  Nonce: {}", nonce);
    println!();

    println!("Signing order...");
    match sign_order_python(
        base_asset_id,
        quote_asset_id,
        base_amount,
        quote_amount,
        fee_amount,
        position_id,
        nonce,
        expiration_epoch_millis,
        public_key,
        private_key,
        "SN_MAIN",
    ) {
        Ok((r, s)) => {
            println!("✅ Order signed successfully!\n");
            println!("Signature:");
            println!("  r: {}", r);
            println!("  s: {}", s);
            println!();
            println!("This signature can now be used in Extended DEX API requests.");
        }
        Err(e) => {
            eprintln!("❌ Error signing order: {}", e);
            eprintln!();
            eprintln!("Make sure:");
            eprintln!("  1. Python is installed and in PATH");
            eprintln!("  2. python_sdk-starknet is in the project root");
            eprintln!("  3. scripts/sign_order.py exists");
            std::process::exit(1);
        }
    }
}
