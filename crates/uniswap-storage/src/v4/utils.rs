use alloy_primitives::{Address, B256, U256, U512, aliases::I24, b256, keccak256};

pub const FIXED_POINT_128: B256 = b256!("0x0000000000000000000000000000000100000000000000000000000000000000");

pub const MIN_TICK: i32 = -887272;
pub const MAX_TICK: i32 = 887272;

pub fn encode_position_key(
    position_manager_address: Address,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> B256 {
    let mut bytes = [0u8; 70];
    bytes[12..32].copy_from_slice(&**position_manager_address);
    bytes[32..35].copy_from_slice(&tick_lower.to_be_bytes::<3>());
    bytes[35..38].copy_from_slice(&tick_upper.to_be_bytes::<3>());
    bytes[38..].copy_from_slice(&*B256::from(position_token_id));
    keccak256(&bytes[12..])
}

pub fn flat_div_x128(numerator: U256, denominator: U256) -> U256 {
    if denominator.is_zero() {
        return U256::ZERO;
    }

    // Promote everything to 512 bits.
    let num_u512: U512 = U512::from(numerator) << 128; // numerator * 2**128
    let den_u512: U512 = U512::from(denominator);

    // Full‑precision division, then cast back to 256 bits (guaranteed to fit).
    let result_u512: U512 = num_u512 / den_u512;
    U256::from(result_u512)
}

pub fn full_mul_x128(x: U256, y: U256) -> U256 {
    if x.is_zero() || y.is_zero() {
        return U256::ZERO;
    }

    let prod: U512 = U512::from(x) * U512::from(y);

    let shifted: U512 = prod >> 128u32;

    if (shifted >> 256u32) != U512::ZERO {
        panic!("We check the final result doesn't overflow by checking that p1_0 = 0"); // same condition that triggers revert in Solidity
    }

    U256::from(shifted)
}

pub fn mul_div(a: U256, b: U256, denominator: U256) -> U256 {
    if denominator.is_zero() {
        panic!("require(denominator != 0)");
    }

    // 512-bit product
    let product: U512 = U512::from(a) * U512::from(b);

    // Split into high / low 256-bit words
    let mask_256: U512 = U512::from(U256::MAX); // 2^256 − 1
    let prod0 = U256::from(product & mask_256); // low 256 bits
    let prod1 = U256::from(product >> 256u32); // high 256 bits

    // Overflow check (denominator must be > prod1)
    if denominator <= prod1 {
        panic!("require(denominator > prod1)");
    }

    if prod1.is_zero() {
        return prod0 / denominator;
    }

    let quotient = product / U512::from(denominator);
    U256::from(quotient)
}

pub fn max_valid_tick(tick_spacing: I24) -> I24 {
    I24::unchecked_from(MAX_TICK) / tick_spacing * tick_spacing
}

pub fn min_valid_tick(tick_spacing: I24) -> I24 {
    I24::unchecked_from(MIN_TICK) / tick_spacing * tick_spacing
}
