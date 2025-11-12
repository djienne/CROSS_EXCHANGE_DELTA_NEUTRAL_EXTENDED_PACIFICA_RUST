use starknet_crypto::Felt;
use sha3::{Digest, Keccak256};

fn main() {
    let input = b"test";
    let mut hasher = Keccak256::new();
    hasher.update(input);
    let result = hasher.finalize();

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    let felt = Felt::from_bytes_be(&bytes);

    println!("Raw Keccak bytes: 0x{}", hex::encode(&bytes));
    println!("As Felt: {}", felt.to_hex_string());

    // Now test with domain type string
    let domain_type_string = concat!(
        "StarknetDomain(",
        "name:shortstring,",
        "version:shortstring,",
        "chainId:shortstring,",
        "revision:shortstring",
        ")"
    );

    let mut hasher2 = Keccak256::new();
    hasher2.update(domain_type_string.as_bytes());
    let result2 = hasher2.finalize();

    let mut bytes2 = [0u8; 32];
    bytes2.copy_from_slice(&result2);
    let felt2 = Felt::from_bytes_be(&bytes2);

    println!("\nDomain type string: {}", domain_type_string);
    println!("Raw Keccak bytes: 0x{}", hex::encode(&bytes2));
    println!("As Felt: {}", felt2.to_hex_string());
}
