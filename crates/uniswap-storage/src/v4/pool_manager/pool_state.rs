use alloy_eips::BlockId;
use alloy_primitives::{Address, B256, U256, keccak256};
use alloy_sol_types::SolValue;

use crate::{
    StorageSlotFetcher,
    v4::{UnpackedSlot0, common::UnpackSlot0}
};

// pool state
pub const POOL_MANAGER_POOL_STATE_MAP_SLOT: u8 = 6;
pub const POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL0_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL1_X128_SLOT_OFFSET: u8 = 2;
pub const POOL_MANAGER_POOL_LIQUIDITY_SLOT_OFFSET: u8 = 3;

pub fn pool_manager_pool_state_slot(pool_id: U256) -> B256 {
    keccak256((pool_id, U256::from(POOL_MANAGER_POOL_STATE_MAP_SLOT)).abi_encode())
}

pub async fn pool_manager_pool_fee_growth_global<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    pool_id: B256,
    block_id: BlockId
) -> eyre::Result<(U256, U256)> {
    let pool_state_slot = pool_manager_pool_state_slot(pool_id.into());
    let pool_state_slot_base = U256::from_be_slice(pool_state_slot.as_slice());

    let fee_growth_global0_x128_slot =
        pool_state_slot_base + U256::from(POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL0_X128_SLOT_OFFSET);
    let fee_growth_global1_x128_slot =
        pool_state_slot_base + U256::from(POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL1_X128_SLOT_OFFSET);

    let (fee_growth_global0_x128, fee_growth_global1_x128) = futures::try_join!(
        slot_fetcher.storage_at(pool_manager_address, fee_growth_global0_x128_slot.into(), block_id),
        slot_fetcher.storage_at(pool_manager_address, fee_growth_global1_x128_slot.into(), block_id)
    )?;

    Ok((fee_growth_global0_x128, fee_growth_global1_x128))
}

pub async fn pool_manager_pool_slot0<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    pool_id: B256,
    block_id: BlockId
) -> eyre::Result<UnpackedSlot0> {
    let pool_state_slot = pool_manager_pool_state_slot(pool_id.into());

    let packed_slot0 = slot_fetcher
        .storage_at(pool_manager_address, pool_state_slot, block_id)
        .await?;

    Ok(packed_slot0.unpack_slot0())
}

pub async fn pool_manager_pool_liquidity<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    pool_id: B256,
    block_id: BlockId
) -> eyre::Result<U256> {
    let pool_state_slot = pool_manager_pool_state_slot(pool_id.into());
    let pool_state_slot_base = U256::from_be_slice(pool_state_slot.as_slice());

    let liquidity_slot = pool_state_slot_base + U256::from(POOL_MANAGER_POOL_LIQUIDITY_SLOT_OFFSET);

    let liquidity = slot_fetcher
        .storage_at(pool_manager_address, liquidity_slot.into(), block_id)
        .await?;

    Ok(liquidity)
}

#[cfg(test)]
mod tests {
    use alloy_eips::BlockId;
    use alloy_primitives::{
        U160,
        aliases::{I24, U24}
    };

    use super::*;
    use crate::{
        angstrom::mainnet::ANGSTROM_L1_CONSTANTS_MAINNET,
        test_utils::*,
        v4::{UNISWAP_V4_CONSTANTS_MAINNET, V4PoolKey}
    };

    #[tokio::test]
    async fn test_pool_manager_pool_fee_growth_global() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };

        let results = pool_manager_pool_fee_growth_global(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            pool_key.into(),
            BlockId::number(block_number)
        )
        .await
        .unwrap();

        assert_eq!(
            results,
            (
                U256::from(13180763546271931776686481343917_u128),
                U256::from_str_radix("3950914474349982817843030715313190508120", 10).unwrap()
            )
        );
    }

    #[tokio::test]
    async fn test_pool_manager_pool_slot0() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };

        let results = pool_manager_pool_slot0(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            pool_key.into(),
            BlockId::number(block_number)
        )
        .await
        .unwrap();

        let expected = UnpackedSlot0 {
            sqrt_price_x96: U160::from(1426501997508494087190670451504265_u128),
            tick:           I24::unchecked_from(195977),
            protocol_fee:   U24::ZERO,
            lp_fee:         U24::ZERO
        };

        assert_eq!(results, expected);
    }

    #[tokio::test]
    async fn test_pool_manager_pool_liquidity() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };

        let results = pool_manager_pool_liquidity(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            pool_key.into(),
            BlockId::number(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(83376670949115238_u128));
    }
}
