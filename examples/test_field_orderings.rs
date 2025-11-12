/// Test different Order struct field orderings to find the correct one
///
/// Run with: cargo test --test test_field_orderings -- --nocapture

use starknet_crypto::{poseidon_hash_many, Felt};
use sha3::{Digest, Keccak256};

const EXPECTED_PYTHON_HASH: &str = "0x6975746003ff809e5fb38167ac8de1b409a9d966f9682adf5cbeb5497b24ece";

fn starknet_keccak(input: &[u8]) -> Felt {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Felt::from_bytes_be(&bytes)
}

fn hex_to_felt(hex_str: &str) -> Felt {
    let cleaned = hex_str.trim_start_matches("0x");
    Felt::from_hex(cleaned).expect("Invalid hex")
}

fn felt_to_hex(felt: &Felt) -> String {
    felt.to_hex_string()
}

fn encode_short_string(s: &str) -> Felt {
    let bytes = s.as_bytes();
    let mut padded = [0u8; 32];
    let offset = 32 - bytes.len();
    padded[offset..].copy_from_slice(bytes);
    Felt::from_bytes_be(&padded)
}

fn get_domain_type_hash() -> Felt {
    let domain_type_string = concat!(
        "StarknetDomain(",
        "name:shortstring,",
        "version:shortstring,",
        "chainId:shortstring,",
        "revision:shortstring",
        ")"
    );
    starknet_keccak(domain_type_string.as_bytes())
}

fn hash_domain() -> Felt {
    let type_hash = get_domain_type_hash();
    let name = encode_short_string("Perpetuals");
    let version = encode_short_string("v0");
    let chain_id = encode_short_string("SN_MAIN");
    let revision = Felt::ONE;  // Integer 1, not shortstring

    poseidon_hash_many(&[type_hash, name, version, chain_id, revision])
}

fn test_order_with_type_string(type_string: &str, fields: &[Felt]) -> String {
    // Compute type hash
    let type_hash = starknet_keccak(type_string.as_bytes());

    // Build fields array with type_hash first
    let mut all_fields = vec![type_hash];
    all_fields.extend_from_slice(fields);

    // Hash the struct
    let struct_hash = poseidon_hash_many(&all_fields);

    // Get other components
    let domain_hash = hash_domain();
    let account = hex_to_felt("0x338f4cb92453dfb7c7764549d85ab624e6614db51b4c25c0fd63da09f07d127");
    let prefix = starknet_keccak(b"StarkNet Message");

    // Final message hash
    let message_hash = poseidon_hash_many(&[prefix, domain_hash, account, struct_hash]);
    felt_to_hex(&message_hash)
}

#[test]
fn test_all_field_orderings() {
    // Test data
    let position_id = Felt::from(226109u64);
    let base_asset_id = hex_to_felt("0x534f4c2d33");
    let base_amount = Felt::from(100u128);
    let quote_asset_id = Felt::from(1u128);
    let quote_amount = Felt::ZERO - Felt::from(16229000u128);  // Negative
    let fee_asset_id = Felt::from(1u128);
    let fee_amount = Felt::from(9738u128);
    let expiration = Felt::from(1701209600u64);
    let salt = Felt::from(1234567890u64);

    println!("\n=== TESTING FIELD ORDERINGS ===");
    println!("Expected hash: {}\n", EXPECTED_PYTHON_HASH);

    // Test 1: Original order (fee_amount before fee_asset_id)
    let type1 = concat!(
        "Order(position_id:felt,base_asset_id:felt,base_amount:felt,",
        "quote_asset_id:felt,quote_amount:felt,fee_amount:felt,fee_asset_id:felt,",
        "expiration:felt,salt:felt)"
    );
    let fields1 = vec![
        position_id, base_asset_id, base_amount,
        quote_asset_id, quote_amount, fee_amount, fee_asset_id,
        expiration, salt
    ];
    let hash1 = test_order_with_type_string(type1, &fields1);
    println!("Test 1 (fee_amount before fee_asset_id): {}", hash1);
    if hash1 == EXPECTED_PYTHON_HASH {
        println!("✓ MATCH FOUND!\n");
        return;
    }

    // Test 2: Swapped fees (fee_asset_id before fee_amount)
    let type2 = concat!(
        "Order(position_id:felt,base_asset_id:felt,base_amount:felt,",
        "quote_asset_id:felt,quote_amount:felt,fee_asset_id:felt,fee_amount:felt,",
        "expiration:felt,salt:felt)"
    );
    let fields2 = vec![
        position_id, base_asset_id, base_amount,
        quote_asset_id, quote_amount, fee_asset_id, fee_amount,
        expiration, salt
    ];
    let hash2 = test_order_with_type_string(type2, &fields2);
    println!("Test 2 (fee_asset_id before fee_amount): {}", hash2);
    if hash2 == EXPECTED_PYTHON_HASH {
        println!("✓ MATCH FOUND!\n");
        return;
    }

    // Test 3: All IDs first, then amounts
    let type3 = concat!(
        "Order(position_id:felt,base_asset_id:felt,quote_asset_id:felt,fee_asset_id:felt,",
        "base_amount:felt,quote_amount:felt,fee_amount:felt,expiration:felt,salt:felt)"
    );
    let fields3 = vec![
        position_id, base_asset_id, quote_asset_id, fee_asset_id,
        base_amount, quote_amount, fee_amount,
        expiration, salt
    ];
    let hash3 = test_order_with_type_string(type3, &fields3);
    println!("Test 3 (IDs first, then amounts): {}", hash3);
    if hash3 == EXPECTED_PYTHON_HASH {
        println!("✓ MATCH FOUND!\n");
        return;
    }

    // Test 4: Salt first (some implementations put nonce/salt early)
    let type4 = concat!(
        "Order(salt:felt,position_id:felt,base_asset_id:felt,base_amount:felt,",
        "quote_asset_id:felt,quote_amount:felt,fee_asset_id:felt,fee_amount:felt,expiration:felt)"
    );
    let fields4 = vec![
        salt, position_id, base_asset_id, base_amount,
        quote_asset_id, quote_amount, fee_asset_id, fee_amount,
        expiration
    ];
    let hash4 = test_order_with_type_string(type4, &fields4);
    println!("Test 4 (salt first): {}", hash4);
    if hash4 == EXPECTED_PYTHON_HASH {
        println!("✓ MATCH FOUND!\n");
        return;
    }

    // Test 5: Expiration earlier (before fees)
    let type5 = concat!(
        "Order(position_id:felt,base_asset_id:felt,base_amount:felt,",
        "quote_asset_id:felt,quote_amount:felt,expiration:felt,",
        "fee_asset_id:felt,fee_amount:felt,salt:felt)"
    );
    let fields5 = vec![
        position_id, base_asset_id, base_amount,
        quote_asset_id, quote_amount, expiration,
        fee_asset_id, fee_amount, salt
    ];
    let hash5 = test_order_with_type_string(type5, &fields5);
    println!("Test 5 (expiration before fees): {}", hash5);
    if hash5 == EXPECTED_PYTHON_HASH {
        println!("✓ MATCH FOUND!\n");
        return;
    }

    // Test 6: user_public_key in the struct (at the end)
    let public_key = hex_to_felt("0x338f4cb92453dfb7c7764549d85ab624e6614db51b4c25c0fd63da09f07d127");
    let type6 = concat!(
        "Order(position_id:felt,base_asset_id:felt,base_amount:felt,",
        "quote_asset_id:felt,quote_amount:felt,fee_asset_id:felt,fee_amount:felt,",
        "expiration:felt,salt:felt,user_public_key:felt)"
    );
    let fields6 = vec![
        position_id, base_asset_id, base_amount,
        quote_asset_id, quote_amount, fee_asset_id, fee_amount,
        expiration, salt, public_key
    ];

    // For this test, we need to NOT include public_key in the final hash since it's in the struct
    let type_hash6 = starknet_keccak(type6.as_bytes());
    let mut all_fields6 = vec![type_hash6];
    all_fields6.extend_from_slice(&fields6);
    let struct_hash6 = poseidon_hash_many(&all_fields6);
    let domain_hash = hash_domain();
    let prefix = starknet_keccak(b"StarkNet Message");
    // Now hash WITHOUT account since it's in the struct
    let message_hash6 = poseidon_hash_many(&[prefix, domain_hash, struct_hash6]);
    let hash6 = felt_to_hex(&message_hash6);

    println!("Test 6 (user_public_key in struct, NO account in final hash): {}", hash6);
    if hash6 == EXPECTED_PYTHON_HASH {
        println!("✓ MATCH FOUND!\n");
        return;
    }

    // Test 7: user_public_key in struct, but still use account in final hash
    let message_hash7 = poseidon_hash_many(&[prefix, domain_hash, public_key, struct_hash6]);
    let hash7 = felt_to_hex(&message_hash7);
    println!("Test 7 (user_public_key in struct, WITH account in final hash): {}", hash7);
    if hash7 == EXPECTED_PYTHON_HASH {
        println!("✓ MATCH FOUND!\n");
        return;
    }

    println!("\n✗ No match found in common patterns");
}
