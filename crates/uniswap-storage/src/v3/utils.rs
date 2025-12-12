use alloy_primitives::keccak256;
pub use alloy_primitives::{B256, U256, aliases::I24};
use alloy_sol_types::SolValue;

// Storage slot constants for UniswapV3Pool
pub const SLOT0_SLOT: u8 = 0;
pub const FEE_GROWTH_GLOBAL_0X128_SLOT: u8 = 1;
pub const FEE_GROWTH_GLOBAL_1X128_SLOT: u8 = 2;
pub const LIQUIDITY_SLOT: u8 = 4;
pub const TICKS_SLOT: u8 = 5;
pub const TICK_BITMAP_SLOT: u8 = 6;

// Tick storage layout offsets
pub const TICK_FEE_GROWTH_OUTSIDE0_X128_OFFSET: u8 = 1;
pub const TICK_FEE_GROWTH_OUTSIDE1_X128_OFFSET: u8 = 2;
pub const TICK_INITIALIZED_OFFSET: u8 = 3;

pub fn v3_tick_slot(tick: I24) -> B256 {
    keccak256((tick, U256::from(TICKS_SLOT)).abi_encode())
}

pub fn position_compressed_tick(tick: I24, tick_spacing: I24, lte: bool) -> (I24, (i16, u8)) {
    let mut compressed = tick / tick_spacing;
    if tick < I24::ZERO && tick % tick_spacing != I24::ZERO {
        compressed -= I24::ONE;
    }

    let (word_pos, bit_pos) = if lte {
        let c = compressed.as_i32();
        ((c >> 8) as i16, (c % 256_i32) as u8)
    } else {
        let c = (compressed + I24::ONE).as_i32();
        ((c >> 8) as i16, (c % 256_i32) as u8)
    };

    (compressed, (word_pos, bit_pos))
}

pub fn v3_tick_bitmap_slot(word_pos: i16) -> B256 {
    keccak256((word_pos, U256::from(TICK_BITMAP_SLOT)).abi_encode())
}
pub fn most_significant_bit(x: U256) -> u8 {
    assert!(x > U256::ZERO, "x must be greater than 0");
    255 - (x.leading_zeros() as u8)
}

pub fn least_significant_bit(x: U256) -> u8 {
    assert!(x > U256::ZERO, "x must be greater than 0");
    x.trailing_zeros() as u8
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_position_compressed_tick_non_zfo() {
        let tick = I24::unchecked_from(193346);
        let result = position_compressed_tick(tick, I24::unchecked_from(10), false);

        let expected = (I24::unchecked_from(19334), (75, 135));

        assert_eq!(result, expected);
    }

    #[test]
    fn test_position_compressed_tick_zfo() {
        let tick = I24::unchecked_from(192026);
        let result = position_compressed_tick(tick, I24::unchecked_from(10), true);

        let expected = (I24::unchecked_from(19202), (75, 2));

        assert_eq!(result, expected);
    }
}
