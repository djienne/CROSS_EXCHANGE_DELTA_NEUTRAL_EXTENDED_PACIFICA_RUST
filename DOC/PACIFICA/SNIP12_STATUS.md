# SNIP-12 Implementation Status

## Overview

This document tracks the status of the pure Rust SNIP-12 implementation for Extended DEX order signing.

## Current Status: ⚠️ **Work in Progress**

The Rust implementation is **functionally complete** but produces different signatures than Extended's Python SDK. This is due to an undocumented Order struct field ordering in Extended's smart contract.

### What's Working ✅

All individual SNIP-12 components have been implemented and verified against the Python SDK:

1. **Domain Separator Encoding**
   - Correctly encodes `StarknetDomain` with Poseidon hashing
   - Properly handles revision field as integer `1` (not shortstring per SNIP-12 rev 1 spec)
   - Verified: `domain_type_hash` matches Python exactly

2. **Type Hashing**
   - Correctly computes Keccak-256 hashes of type strings
   - Properly reduces modulo Starknet field prime
   - Verified: `order_type_hash` and `domain_type_hash` match Python

3. **Field Encoding**
   - Correct short string encoding for domain fields
   - Proper negative number handling using field arithmetic (`Felt::ZERO - abs(x)`)
   - Correct hex-to-Felt conversions for asset IDs and public keys
   - Verified: All individual field encodings match Python

4. **Settlement Expiration**
   - Correctly adds 14-day buffer to order expiration
   - Converts milliseconds to seconds properly

5. **Prefix Hashing**
   - Correctly computes `starknet_keccak("StarkNet Message")`

6. **ECDSA Signing**
   - Uses `starknet-crypto` crate for signing on STARK curve
   - Generates valid signatures (r, s components)

### What's Not Working ❌

**Final message hash doesn't match Python SDK**

- Rust: `0x704f7e06d65d973266be0df983d5859f39807ce1590af5caf0d942dcdf7089c`
- Python: `0x6975746003ff809e5fb38167ac8de1b409a9d966f9682adf5cbeb5497b24ece`

Since all input components are verified correct, the issue is in the **Order struct field ordering**.

## Root Cause Analysis

Extended's `fast_stark_crypto` library (used by the Python SDK) is a compiled Rust binary without public source code. The exact Order struct definition used by Extended's smart contract is not documented.

### Tested Field Orderings

We systematically tested multiple common patterns:

1. ❌ Standard: `position_id, base_asset_id, base_amount, quote_asset_id, quote_amount, fee_amount, fee_asset_id, expiration, salt`
2. ❌ Swapped fees: `..., fee_asset_id, fee_amount, ...`
3. ❌ Grouped IDs: `position_id, base_asset_id, quote_asset_id, fee_asset_id, base_amount, ...`
4. ❌ Salt first: `salt, position_id, ...`
5. ❌ Expiration early: `..., expiration, fee_asset_id, fee_amount, salt`
6. ❌ user_public_key in struct (end)
7. ❌ user_public_key in struct (various positions)

None matched the Python SDK output.

## Current Implementation (src/snip12/hash.rs)

```rust
Order(
    position_id:felt,
    base_asset_id:felt,
    base_amount:felt,
    quote_asset_id:felt,
    quote_amount:felt,
    fee_amount:felt,
    fee_asset_id:felt,
    expiration:felt,
    salt:felt
)
```

This follows standard SNIP-12 conventions but doesn't match Extended's actual contract.

## Recommended Action Plan

### Short Term (Current) ✅

**Use Python SDK via subprocess** for production order signing:

```rust
// In signature.rs or similar
fn sign_order_python(order_params) -> Result<Signature> {
    // Call scripts/sign_order.py via subprocess
    // This is reliable and matches Extended's expectations
}
```

**Advantages:**
- ✅ Works reliably with Extended's API
- ✅ Signatures are guaranteed correct
- ✅ No risk of production failures

**Disadvantages:**
- ⚠️ Requires Python runtime
- ⚠️ Slower than native Rust (subprocess overhead)
- ⚠️ Dependency on python_sdk-starknet

### Long Term (Future) 🎯

**Obtain the exact Order struct from Extended:**

**Option A: Contact Extended Support**
- Ask for the exact Order struct field ordering
- Request the type string used in their smart contract
- Fastest path to solution

**Option B: Find Smart Contract Source**
- Search Starknet mainnet/testnet explorers
- Look for Extended's deployed perpetual contract
- Extract Order struct from Cairo source code

**Option C: Reverse Engineer from Network Traffic**
- Capture actual order placement requests from Extended's UI
- Analyze the settlement object structure
- May reveal the correct field ordering

Once obtained:
1. Update `src/snip12/hash.rs::get_order_type_hash()` with correct field order
2. Update `src/snip12/hash.rs::hash_order_struct()` to match
3. Run tests to verify: `cargo test test_buy_order_rust_vs_python -- --ignored`
4. Switch production code to pure Rust implementation

## Testing

### Run Comparison Tests

```bash
# Compare Rust vs Python SDK signatures
cargo test test_buy_order_rust_vs_python -- --ignored --nocapture

# Test individual field orderings
cargo test --test test_field_orderings -- --nocapture
```

### Expected Output (Once Fixed)

```
Rust message_hash: 0x6975746003ff809e5fb38167ac8de1b409a9d966f9682adf5cbeb5497b24ece
Python message_hash: 0x6975746003ff809e5fb38167ac8de1b409a9d966f9682adf5cbeb5497b24ece

✓ Signatures match!
```

## Files

- `src/snip12/mod.rs` - Module entry point with documentation
- `src/snip12/domain.rs` - StarknetDomain struct
- `src/snip12/hash.rs` - All hashing functions (WORKS but needs correct field order)
- `src/snip12/signing.rs` - ECDSA signing (WORKS correctly)
- `src/snip12/tests.rs` - Comparison tests against Python SDK
- `test_field_orderings.rs` - Systematic field ordering tests
- `scripts/sign_order.py` - Python SDK wrapper (PRODUCTION USE)
- `generate_test_vector.py` - Test vector generator

## Dependencies

```toml
starknet-crypto = "0.8"        # ECDSA signing, Poseidon hashing
starknet-types-core = "0.2"    # Field elements (must match starknet-crypto)
sha3 = "0.10"                  # Keccak-256 for type hashing
```

## References

- [SNIP-12 Specification](https://github.com/starknet-io/SNIPs/blob/main/SNIPS/snip-12.md)
- [Extended API Documentation](https://api.docs.extended.exchange/)
- [Extended Python SDK](https://github.com/x10xchange/python_sdk)
- [fast_stark_crypto (Extended's Rust library)](https://github.com/x10xchange/stark-crypto-wrapper)

## Timeline

- **2025-01-06**: Initial SNIP-12 implementation completed
- **2025-01-06**: Debugging revealed field ordering mismatch
- **2025-01-06**: Documented status, reverted to Python subprocess for production
- **TBD**: Awaiting Extended's Order struct specification

## Contact Extended

If you need the Order struct definition:

1. **Support**: Contact Extended's technical support
2. **GitHub**: Open an issue in their Python SDK repo
3. **Documentation**: Check if API docs have been updated with struct definition
4. **Community**: Ask in Extended's Discord/Telegram

---

**Last Updated**: 2025-01-06
**Status**: Ready for production with Python subprocess, Rust implementation on hold pending struct definition
