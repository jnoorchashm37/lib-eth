use alloy_primitives::{Address, StorageValue, U160, U256, aliases::I24};

use crate::{StorageSlotFetcher, types::TickData, v3::utils::*};

pub async fn v3_current_tick<P: StorageSlotFetcher>(
    provider: &P,
    pool: Address,
    block_number: Option<u64>
) -> eyre::Result<I24> {
    let slot0_key = U256::from(SLOT0_SLOT);
    let slot0_value = provider
        .storage_at(pool, slot0_key.into(), block_number)
        .await?;

    let tick_raw: U256 = (slot0_value >> 160) & U256::from((1u64 << 24) - 1);

    let tick_i32 = if tick_raw.bit(23) {
        let mask = !((1u32 << 24) - 1);
        (tick_raw.as_limbs()[0] as u32 | mask) as i32
    } else {
        tick_raw.as_limbs()[0] as i32
    };

    Ok(I24::unchecked_from(tick_i32))
}

pub async fn v3_current_liquidity<P: StorageSlotFetcher>(
    provider: &P,
    pool: Address,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let liquidity_key = U256::from(LIQUIDITY_SLOT);
    let liquidity = provider
        .storage_at(pool, liquidity_key.into(), block_number)
        .await?;

    Ok(liquidity & U256::from(u128::MAX))
}

pub async fn v3_fee_growth_global<P: StorageSlotFetcher>(
    provider: &P,
    pool: Address,
    block_number: Option<u64>
) -> eyre::Result<(U256, U256)> {
    let fee_growth_global0_key = U256::from(FEE_GROWTH_GLOBAL_0X128_SLOT);
    let fee_growth_global1_key = U256::from(FEE_GROWTH_GLOBAL_1X128_SLOT);

    let (fee_growth_global0, fee_growth_global1) = futures::try_join!(
        provider.storage_at(pool, fee_growth_global0_key.into(), block_number),
        provider.storage_at(pool, fee_growth_global1_key.into(), block_number),
    )?;

    Ok((fee_growth_global0, fee_growth_global1))
}

pub async fn v3_sqrt_price_x96<P: StorageSlotFetcher>(
    provider: &P,
    pool: Address,
    block_number: Option<u64>
) -> eyre::Result<U160> {
    let slot0_key = U256::from(SLOT0_SLOT);
    let slot0_value = provider
        .storage_at(pool, slot0_key.into(), block_number)
        .await?;

    Ok(U160::from(slot0_value & U256::from(U160::MAX)))
}

pub async fn v3_tick_data<P: StorageSlotFetcher>(
    provider: &P,
    pool: Address,
    tick: I24,
    block_number: Option<u64>
) -> eyre::Result<TickData> {
    let tick_slot = v3_tick_slot(tick);
    let tick_slot_base = U256::from_be_slice(tick_slot.as_slice());

    // Calculate storage slots
    let slot0 = tick_slot_base;
    let slot1 = tick_slot_base + U256::from(TICK_FEE_GROWTH_OUTSIDE0_X128_OFFSET);
    let slot2 = tick_slot_base + U256::from(TICK_FEE_GROWTH_OUTSIDE1_X128_OFFSET);
    let slot3 = tick_slot_base + U256::from(TICK_INITIALIZED_OFFSET);

    // Fetch all storage values in parallel
    let (slot0_data, fee_growth_outside0_x128, fee_growth_outside1_x128, slot3_data): (
        StorageValue,
        StorageValue,
        StorageValue,
        StorageValue
    ) = futures::try_join!(
        provider.storage_at(pool, slot0.into(), block_number),
        provider.storage_at(pool, slot1.into(), block_number),
        provider.storage_at(pool, slot2.into(), block_number),
        provider.storage_at(pool, slot3.into(), block_number)
    )?;

    let liquidity_gross_u128 = (slot0_data & U256::from(u128::MAX)).to::<u128>();
    let liquidity_net_i128 = (((slot0_data >> 128) & U256::from(u128::MAX)) as U256).to::<u128>() as i128;

    let initialized: U256 = (slot3_data >> 248) & U256::from(0xFF);

    Ok(TickData {
        tick,
        is_initialized: !initialized.is_zero(),
        liquidity_net: liquidity_net_i128,
        liquidity_gross: liquidity_gross_u128,
        fee_growth_outside0_x128,
        fee_growth_outside1_x128
    })
}

async fn v3_tick_bitmap_from_word<P: StorageSlotFetcher>(
    provider: &P,
    pool: Address,
    word_pos: i16,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let bitmap_slot = v3_tick_bitmap_slot(word_pos);
    let bitmap_value = provider.storage_at(pool, bitmap_slot, block_number).await?;
    Ok(bitmap_value)
}

pub async fn v3_next_initialized_tick_within_one_word<P: StorageSlotFetcher>(
    provider: &P,
    pool: Address,
    tick: I24,
    tick_spacing: I24,
    lte: bool,
    block_number: Option<u64>
) -> eyre::Result<(I24, bool)> {
    let (compressed, (word_pos, bit_pos)) = position_compressed_tick(tick, tick_spacing, lte);
    let bitmap = v3_tick_bitmap_from_word::<P>(provider, pool, word_pos, block_number).await?;
    if lte {
        let mask = (U256::ONE << bit_pos) - U256::ONE + (U256::ONE << bit_pos);
        let masked = bitmap & mask;

        let initialized = masked != U256::ZERO;
        let next = if initialized {
            (compressed - I24::unchecked_from(bit_pos - most_significant_bit(masked))) * tick_spacing
        } else {
            (compressed - I24::unchecked_from(bit_pos)) * tick_spacing
        };
        Ok((next, initialized))
    } else {
        let mask = !((U256::ONE << bit_pos) - U256::ONE);
        let masked = bitmap & mask;
        let initialized = masked != U256::ZERO;
        let next = if initialized {
            let lsb = least_significant_bit(masked);
            let diff = (lsb as i32).wrapping_sub(bit_pos as i32);
            (compressed + I24::ONE + I24::unchecked_from(diff)) * tick_spacing
        } else {
            (compressed + I24::ONE + I24::unchecked_from(u8::MAX - bit_pos)) * tick_spacing
        };
        Ok((next, initialized))
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;
    use alloy_provider::RootProvider;

    use super::*;
    use crate::test_utils::eth_provider;

    const POOL_ADDRESS: Address = address!("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640");

    #[tokio::test]
    async fn test_v3_current_tick() {
        let block_number = 23451188;

        let provider = eth_provider().await;

        let result = v3_current_tick::<RootProvider>(&provider, POOL_ADDRESS, Some(block_number))
            .await
            .unwrap();
        let expected = I24::unchecked_from(193335);

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_v3_current_liquidity() {
        let block_number = 23451373;

        let provider = eth_provider().await;

        let result = v3_current_liquidity::<RootProvider>(&provider, POOL_ADDRESS, Some(block_number))
            .await
            .unwrap();
        let expected = U256::from(2088207984683946894_u128);

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_v3_fee_growth_global() {
        let block_number = 23451249;

        let provider = eth_provider().await;

        let result = v3_fee_growth_global::<RootProvider>(&provider, POOL_ADDRESS, Some(block_number))
            .await
            .unwrap();
        let expected = (
            U256::from_str_radix("4197554680435777242459725317018526", 10).unwrap(),
            U256::from_str_radix("1816403166083297570433961717456020679856760", 10).unwrap()
        );

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_v3_sqrt_price_x96() {
        let block_number = 23451262;

        let provider = eth_provider().await;

        let result = v3_sqrt_price_x96::<RootProvider>(&provider, POOL_ADDRESS, Some(block_number))
            .await
            .unwrap();
        let expected = U160::from(1249291445425461764422472060708837_u128);

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_v3_tick_data() {
        let block_number = 23451277;

        let provider = eth_provider().await;

        let tick = I24::unchecked_from(193320);
        let result = v3_tick_data::<RootProvider>(&provider, POOL_ADDRESS, tick, Some(block_number))
            .await
            .unwrap();
        let expected = TickData {
            tick,
            is_initialized: true,
            liquidity_net: -2349468138820706,
            liquidity_gross: 2372067723247216,
            fee_growth_outside0_x128: U256::from_str_radix("3114302369834678599345866169150924", 10).unwrap(),
            fee_growth_outside1_x128: U256::from_str_radix("1399568734137934671216963164240387940486177", 10).unwrap()
        };

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_v3_next_initialized_tick_within_one_word_non_zfo() {
        let block_number = 23451089;

        let provider = eth_provider().await;

        let tick = I24::unchecked_from(193346);
        let result = v3_next_initialized_tick_within_one_word::<RootProvider>(
            &provider,
            POOL_ADDRESS,
            tick,
            I24::unchecked_from(10),
            false,
            Some(block_number)
        )
        .await
        .unwrap();
        let expected = (I24::unchecked_from(193350), true);

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_v3_next_initialized_tick_within_one_word_zfo() {
        let block_number = 23139316;

        let provider = eth_provider().await;

        let tick = I24::unchecked_from(192026);
        let result = v3_next_initialized_tick_within_one_word::<RootProvider>(
            &provider,
            POOL_ADDRESS,
            tick,
            I24::unchecked_from(10),
            true,
            Some(block_number)
        )
        .await
        .unwrap();
        let expected = (I24::unchecked_from(192020), true);

        assert_eq!(result, expected);
    }
}
