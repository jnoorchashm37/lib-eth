use alloy_eips::BlockId;
use alloy_primitives::{Address, B256, I64, U256, aliases::I24};
use serde::{Deserialize, Serialize};

use crate::{
    StorageSlotFetcher,
    v4::{
        pool_manager::pool_tick_state::pool_manager_pool_tick_bitmap_slot,
        utils::{MAX_TICK, MIN_TICK}
    }
};

pub fn compress_tick(tick: I24, tick_spacing: I24) -> I24 {
    tick.saturating_div(tick_spacing) - if tick % tick_spacing < I24::ZERO { I24::ONE } else { I24::ZERO }
}

pub fn tick_position_from_compressed(tick: I24, tick_spacing: I24) -> (i16, u8) {
    let compressed = compress_tick(tick, tick_spacing);
    _tick_position_from_compressed(compressed)
}

pub fn tick_position_from_compressed_inequality(tick: I24, tick_spacing: I24, add_sub: I24) -> (i16, u8) {
    let compressed = compress_tick(tick, tick_spacing) + add_sub;
    _tick_position_from_compressed(compressed)
}

pub fn normalize_tick(tick: I24, tick_spacing: I24) -> I24 {
    let norm = compress_tick(tick, tick_spacing) * tick_spacing;

    if I64::from(tick) > I64::from(norm) + I64::from(tick_spacing)
        || I64::from(tick) < I64::from(norm) - I64::from(tick_spacing)
        || norm.as_i32() < MIN_TICK
        || norm.as_i32() > MAX_TICK
    {
        if tick.is_negative() {
            return normalize_tick(tick + tick_spacing.abs(), tick_spacing);
        } else {
            return normalize_tick(tick - tick_spacing.abs(), tick_spacing);
        }
    }

    norm
}

fn _tick_position_from_compressed(compressed: I24) -> (i16, u8) {
    let compressed_i32 = compressed.as_i32();
    let word_pos = (compressed_i32 >> 8) as i16;
    let bit_pos = (compressed_i32 & 0xff) as u8;

    (word_pos, bit_pos)
}

pub fn tick_from_word_and_bit_pos(word_pos: i16, bit_pos: u8, tick_spacing: I24) -> I24 {
    (I24::unchecked_from(word_pos) * I24::unchecked_from(256) + I24::unchecked_from(bit_pos)) * tick_spacing
}

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct TickBitmap(pub U256);

impl TickBitmap {
    pub fn is_initialized(&self, bit_pos: u8) -> bool {
        self.0 & (U256::ONE << U256::from(bit_pos)) != U256::ZERO
    }

    pub fn next_bit_pos_gte(&self, bit_pos: u8) -> (bool, u8) {
        let word_shifted = self.0 >> U256::from(bit_pos);

        let relative_pos = if word_shifted == U256::ZERO { 256u16 } else { word_shifted.trailing_zeros() as u16 };

        let initialized = relative_pos != 256;
        let next_bit_pos = if initialized { (relative_pos as u8) + bit_pos } else { u8::MAX };

        (initialized, next_bit_pos)
    }

    pub fn next_bit_pos_lte(&self, bit_pos: u8) -> (bool, u8) {
        let offset = 0xff - bit_pos;

        let word_shifted = self.0 << U256::from(offset);

        let relative_pos = if word_shifted == U256::ZERO { 256u16 } else { 255u16 - word_shifted.leading_zeros() as u16 };

        let initialized = relative_pos != 256;
        let next_bit_pos = if initialized { (relative_pos as u8).saturating_sub(offset) } else { 0u8 };

        (initialized, next_bit_pos)
    }
}

/// https://github.com/Uniswap/v4-core/blob/main/src/libraries/TickBitmap.sol
///
/// function nextInitializedTickWithinOneWord(
///     mapping(int16 => uint256) storage self,
///     int24 tick,
///     int24 tickSpacing,
///     bool lte
/// ) internal view returns (int24 next, bool initialized)
pub async fn next_initialized_tick_within_one_word<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    pool_id: B256,
    tick: I24,
    tick_spacing: I24,
    lte: bool,
    block_id: BlockId
) -> eyre::Result<(I24, bool)> {
    let mut compressed = compress_tick(tick, tick_spacing);
    if lte {
        let (word_pos, bit_pos) = _tick_position_from_compressed(compressed);
        let mask = U256::MAX >> (U256::from(u8::MAX) - U256::from(bit_pos));
        let masked = tick_bitmap_from_word(slot_fetcher, pool_manager_address, pool_id, word_pos, block_id)
            .await?
            .0
            & mask;

        let initialized = masked != U256::ZERO;
        let next = if initialized {
            (compressed - I24::unchecked_from(bit_pos - most_significant_bit(masked))) * tick_spacing
        } else {
            (compressed - I24::unchecked_from(bit_pos)) * tick_spacing
        };
        Ok((next, initialized))
    } else {
        compressed += I24::ONE;
        let (word_pos, bit_pos) = _tick_position_from_compressed(compressed);
        let mask = !((U256::ONE << bit_pos) - U256::ONE);
        let masked = tick_bitmap_from_word(slot_fetcher, pool_manager_address, pool_id, word_pos, block_id)
            .await?
            .0
            & mask;

        let initialized = masked != U256::ZERO;
        let next = if initialized {
            let lsb = least_significant_bit(masked);
            let diff = (lsb as i32).wrapping_sub(bit_pos as i32);
            (compressed + I24::unchecked_from(diff)) * tick_spacing
        } else {
            (compressed + I24::unchecked_from(u8::MAX - bit_pos)) * tick_spacing
        };
        Ok((next, initialized))
    }
}

/// https://github.com/Uniswap/v4-core/blob/main/src/libraries/BitMath.sol
///
/// function mostSignificantBit(uint256 x) internal pure returns (uint8 r)
fn most_significant_bit(x: U256) -> u8 {
    assert!(x > U256::ZERO, "x must be greater than 0");

    // Use U256's leading_zeros method and convert to most significant bit position
    // U256 has 256 bits, so MSB position = 255 - leading_zeros
    255 - (x.leading_zeros() as u8)
}

/// https://github.com/Uniswap/v4-core/blob/main/src/libraries/BitMath.sol
///
/// function leastSignificantBit(uint256 x) internal pure returns (uint8 r)
fn least_significant_bit(x: U256) -> u8 {
    assert!(x > U256::ZERO, "x must be greater than 0");

    // Use U256's trailing_zeros method which efficiently counts zeros from the
    // right
    x.trailing_zeros() as u8
}

pub async fn tick_bitmap_from_word<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    pool_id: B256,
    word_pos: i16,
    block_id: BlockId
) -> eyre::Result<TickBitmap> {
    let pool_tick_bitmap_slot = pool_manager_pool_tick_bitmap_slot(pool_id.into(), word_pos);

    let tick_bitmap = slot_fetcher
        .storage_at(pool_manager_address, pool_tick_bitmap_slot, block_id)
        .await?;

    Ok(TickBitmap(tick_bitmap))
}

pub async fn tick_bitmap_from_tick<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    pool_id: B256,
    tick: I24,
    tick_spacing: I24,
    block_id: BlockId
) -> eyre::Result<TickBitmap> {
    let (word_pos, _) = tick_position_from_compressed(tick, tick_spacing);

    tick_bitmap_from_word(slot_fetcher, pool_manager_address, pool_id, word_pos, block_id).await
}

pub async fn tick_initialized<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    tick_spacing: I24,
    pool_id: B256,
    tick: I24,
    block_id: BlockId
) -> eyre::Result<bool> {
    let (word_pos, bit_pos) = tick_position_from_compressed(tick, tick_spacing);
    let tick_bitmap = tick_bitmap_from_word(slot_fetcher, pool_manager_address, pool_id, word_pos, block_id).await?;

    Ok(tick_bitmap.is_initialized(bit_pos))
}

pub async fn next_tick_gt<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    tick_spacing: I24,
    pool_id: B256,
    tick: I24,
    initialized_only: bool,
    block_id: BlockId
) -> eyre::Result<(bool, I24)> {
    if is_tick_at_bounds(tick, tick_spacing, false) {
        return Ok((false, tick));
    }

    let (word_pos, bit_pos) = tick_position_from_compressed_inequality(tick, tick_spacing, I24::unchecked_from(1));
    let tick_bitmap = tick_bitmap_from_word(slot_fetcher, pool_manager_address, pool_id, word_pos, block_id).await?;

    let (is_initialized, next_bit_pos) = tick_bitmap.next_bit_pos_gte(bit_pos);
    let next_tick = tick_from_word_and_bit_pos(word_pos, next_bit_pos, tick_spacing);
    if !initialized_only || is_initialized || I24::unchecked_from(MAX_TICK) - next_tick <= tick_spacing {
        Ok((is_initialized, next_tick))
    } else {
        Box::pin(next_tick_gt(
            slot_fetcher,
            pool_manager_address,
            tick_spacing,
            pool_id,
            next_tick,
            initialized_only,
            block_id
        ))
        .await
    }
}

pub async fn next_tick_lt<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    tick_spacing: I24,
    pool_id: B256,
    tick: I24,
    initialized_only: bool,
    block_id: BlockId
) -> eyre::Result<(bool, I24)> {
    if is_tick_at_bounds(tick, tick_spacing, true) {
        return Ok((false, tick));
    }

    let (word_pos, bit_pos) = tick_position_from_compressed_inequality(tick, tick_spacing, I24::unchecked_from(-1));
    let tick_bitmap = tick_bitmap_from_word(slot_fetcher, pool_manager_address, pool_id, word_pos, block_id).await?;

    let (is_initialized, next_bit_pos) = tick_bitmap.next_bit_pos_lte(bit_pos);
    let next_tick = tick_from_word_and_bit_pos(word_pos, next_bit_pos, tick_spacing);
    if !initialized_only || is_initialized || next_tick - I24::unchecked_from(MIN_TICK) <= tick_spacing {
        Ok((is_initialized, next_tick))
    } else {
        Box::pin(next_tick_lt(
            slot_fetcher,
            pool_manager_address,
            tick_spacing,
            pool_id,
            next_tick,
            initialized_only,
            block_id
        ))
        .await
    }
}

pub async fn next_tick_le<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    tick_spacing: I24,
    pool_id: B256,
    tick: I24,
    initialized_only: bool,
    block_id: BlockId
) -> eyre::Result<(bool, I24)> {
    if is_tick_at_bounds(tick, tick_spacing, true) {
        return Ok((false, tick));
    }

    let (word_pos, bit_pos) = tick_position_from_compressed(tick, tick_spacing);
    let tick_bitmap = tick_bitmap_from_word(slot_fetcher, pool_manager_address, pool_id, word_pos, block_id).await?;

    let (is_initialized, next_bit_pos) = tick_bitmap.next_bit_pos_lte(bit_pos);
    let next_tick = tick_from_word_and_bit_pos(word_pos, next_bit_pos, tick_spacing);
    if !initialized_only || is_initialized || next_tick - I24::unchecked_from(MIN_TICK) <= tick_spacing {
        Ok((is_initialized, next_tick))
    } else {
        Box::pin(next_tick_le(
            slot_fetcher,
            pool_manager_address,
            tick_spacing,
            pool_id,
            next_tick,
            initialized_only,
            block_id
        ))
        .await
    }
}

fn is_tick_at_bounds(tick: I24, tick_spacing: I24, is_decreasing: bool) -> bool {
    let tick = I64::from(tick);
    let tick_spacing = I64::from(tick_spacing);
    let min = I64::unchecked_from(MIN_TICK);
    let max = I64::unchecked_from(MAX_TICK);

    if is_decreasing { tick - tick_spacing.abs() <= min } else { tick + tick_spacing.abs() >= max }
}

#[cfg(test)]
mod tests {
    use alloy_eips::BlockId;
    use alloy_primitives::{aliases::U24, b256};

    use super::*;
    use crate::{
        angstrom::mainnet::ANGSTROM_L1_CONSTANTS_MAINNET,
        test_utils::*,
        v4::{UNISWAP_V4_CONSTANTS_MAINNET, V4PoolKey}
    };

    #[test]
    fn test_most_significant_bit() {
        // Test basic cases
        assert_eq!(most_significant_bit(U256::from(1)), 0);
        assert_eq!(most_significant_bit(U256::from(2)), 1);

        // Test powers of two
        for i in 0..255 {
            let x = U256::ONE << i;
            assert_eq!(most_significant_bit(x), i as u8);
        }

        // Test max uint256
        assert_eq!(most_significant_bit(U256::MAX), 255);
    }

    #[test]
    #[should_panic(expected = "x must be greater than 0")]
    fn test_most_significant_bit_zero_panics() {
        most_significant_bit(U256::ZERO);
    }

    #[test]
    fn test_least_significant_bit() {
        // Test basic cases
        assert_eq!(least_significant_bit(U256::from(1)), 0);
        assert_eq!(least_significant_bit(U256::from(2)), 1);
        assert_eq!(least_significant_bit(U256::from(3)), 0);
        assert_eq!(least_significant_bit(U256::from(4)), 2);
        assert_eq!(least_significant_bit(U256::from(8)), 3);
        assert_eq!(least_significant_bit(U256::from(0x80)), 7);
        assert_eq!(least_significant_bit(U256::from(0x100)), 8);

        // Test powers of two
        for i in 0..255 {
            let x = U256::ONE << i;
            assert_eq!(least_significant_bit(x), i as u8);
        }

        // Test max uint256 (all bits set)
        assert_eq!(least_significant_bit(U256::MAX), 0);
    }

    #[test]
    #[should_panic(expected = "x must be greater than 0")]
    fn test_least_significant_bit_zero_panics() {
        least_significant_bit(U256::ZERO);
    }

    #[tokio::test]
    async fn test_tick_bitmap() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };
        let pool_id = pool_key.into();

        let results =
            tick_bitmap_from_word(&provider, UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(), pool_id, 346, BlockId::number(block_number))
                .await
                .unwrap();
        assert_eq!(results.0, U256::from_str_radix("2854495385411919762116571938898990272765493248", 10).unwrap());
    }

    #[tokio::test]
    async fn test_tick_initialized() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };
        let tick = I24::unchecked_from(191000);
        let tick_spacing = pool_key.tickSpacing;
        let pool_id = pool_key.into();

        let results = tick_initialized(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            tick_spacing,
            pool_id,
            tick,
            BlockId::number(block_number)
        )
        .await
        .unwrap();
        assert!(results);
    }

    #[tokio::test]
    async fn test_next_tick_gt() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };
        let tick = I24::unchecked_from(190980);
        let tick_spacing = pool_key.tickSpacing;
        let pool_id = pool_key.into();

        let (_, results) = next_tick_gt(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            tick_spacing,
            pool_id,
            tick,
            true,
            BlockId::number(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, I24::unchecked_from(191000));
    }

    #[tokio::test]
    async fn test_next_tick_lt() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };
        let tick = I24::unchecked_from(192311);
        let tick_spacing = pool_key.tickSpacing;
        let pool_id = pool_key.into();

        let (_, results) = next_tick_lt(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            tick_spacing,
            pool_id,
            tick,
            true,
            BlockId::number(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, I24::unchecked_from(192220));
    }

    #[tokio::test]
    async fn test_next_tick_le() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };
        let tick = I24::unchecked_from(192311);
        let tick_spacing = pool_key.tickSpacing;
        let pool_id = pool_key.into();

        let (_, results) = next_tick_le(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            tick_spacing,
            pool_id,
            tick,
            true,
            BlockId::number(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, I24::unchecked_from(192310));
    }

    #[tokio::test]
    async fn test_next_initialized_tick_within_one_word_non_lte() {
        let provider = eth_provider().await;
        let block_number = 23440790;

        let tick = I24::unchecked_from(-193345);
        let tick_spacing = I24::unchecked_from(10);
        let pool_id = b256!("0x21c67e77068de97969ba93d4aab21826d33ca12bb9f565d8496e8fda8a82ca27");

        let (result_tick, initialized) = next_initialized_tick_within_one_word(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            pool_id,
            tick,
            tick_spacing,
            false,
            BlockId::number(block_number)
        )
        .await
        .unwrap();

        assert!(initialized);
        assert_eq!(result_tick, I24::unchecked_from(-193330));
    }

    #[tokio::test]
    async fn test_next_initialized_tick_within_one_word_lte() {
        let provider = eth_provider().await;
        let block_number = 23441855;

        let tick = I24::unchecked_from(-193678);
        let tick_spacing = I24::unchecked_from(10);
        let pool_id = b256!("0x21c67e77068de97969ba93d4aab21826d33ca12bb9f565d8496e8fda8a82ca27");

        let (result_tick, initialized) = next_initialized_tick_within_one_word(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            pool_id,
            tick,
            tick_spacing,
            true,
            BlockId::number(block_number)
        )
        .await
        .unwrap();

        assert!(initialized);
        assert_eq!(result_tick, I24::unchecked_from(-193690));
    }
}
