use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;

use crate::{
    StorageSlotFetcher,
    v4::{
        pool_manager::{
            pool_state::{pool_manager_pool_fee_growth_global, pool_manager_pool_state_slot},
            pool_tick_state::pool_manager_pool_tick_fee_growth_outside
        },
        utils::encode_position_key
    }
};

// position state
pub const POOL_MANAGER_POSITION_STATE_OFFSET_SLOT: u8 = 6;
pub const POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE0_LAST_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE1_LAST_X128_SLOT_OFFSET: u8 = 2;

pub fn pool_manager_position_state_slot(pool_id: U256, position_id: U256) -> B256 {
    let pools_slot = U256::from_be_slice(pool_manager_pool_state_slot(pool_id).as_slice())
        + U256::from(POOL_MANAGER_POSITION_STATE_OFFSET_SLOT);
    keccak256((position_id, pools_slot).abi_encode())
}

pub async fn pool_manager_position_fee_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    pool_id: B256,
    current_tick: I24,
    tick_lower: I24,
    tick_upper: I24,
    block_number: Option<u64>
) -> eyre::Result<(U256, U256)> {
    let (
        (fee_growth_global0_x128, fee_growth_global1_x128),
        (tick_lower_fee_growth_outside0_x128, tick_lower_fee_growth_outside1_x128),
        (tick_upper_fee_growth_outside0_x128, tick_upper_fee_growth_outside1_x128)
    ) = futures::try_join!(
        pool_manager_pool_fee_growth_global(slot_fetcher, pool_manager_address, pool_id, block_number,),
        pool_manager_pool_tick_fee_growth_outside(slot_fetcher, pool_manager_address, pool_id, tick_lower, block_number,),
        pool_manager_pool_tick_fee_growth_outside(slot_fetcher, pool_manager_address, pool_id, tick_upper, block_number,)
    )?;

    let (fee_growth_inside0_x128, fee_growth_inside1_x128) = if current_tick < tick_lower {
        (
            tick_lower_fee_growth_outside0_x128 - tick_upper_fee_growth_outside0_x128,
            tick_lower_fee_growth_outside1_x128 - tick_upper_fee_growth_outside1_x128
        )
    } else if current_tick >= tick_upper {
        (
            tick_upper_fee_growth_outside0_x128 - tick_lower_fee_growth_outside0_x128,
            tick_upper_fee_growth_outside1_x128 - tick_lower_fee_growth_outside1_x128
        )
    } else {
        (
            fee_growth_global0_x128 - tick_lower_fee_growth_outside0_x128 - tick_upper_fee_growth_outside0_x128,
            fee_growth_global1_x128 - tick_lower_fee_growth_outside1_x128 - tick_upper_fee_growth_outside1_x128
        )
    };

    Ok((fee_growth_inside0_x128, fee_growth_inside1_x128))
}

pub async fn pool_manager_position_state_last_fee_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    position_manager_address: Address,
    pool_id: B256,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24,
    block_number: Option<u64>
) -> eyre::Result<(U256, U256)> {
    let position_key = U256::from_be_slice(
        encode_position_key(position_manager_address, position_token_id, tick_lower, tick_upper).as_slice()
    );
    let position_state_slot = pool_manager_position_state_slot(pool_id.into(), position_key);
    let position_state_slot_base = U256::from_be_slice(position_state_slot.as_slice());

    let fee_growth_inside0_last_x128_slot =
        position_state_slot_base + U256::from(POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE0_LAST_X128_SLOT_OFFSET);
    let fee_growth_inside1_last_x128_slot =
        position_state_slot_base + U256::from(POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE1_LAST_X128_SLOT_OFFSET);

    let (fee_growth_inside0_last_x128, fee_growth_inside1_last_x128) = futures::try_join!(
        slot_fetcher.storage_at(pool_manager_address, fee_growth_inside0_last_x128_slot.into(), block_number),
        slot_fetcher.storage_at(pool_manager_address, fee_growth_inside1_last_x128_slot.into(), block_number)
    )?;

    Ok((fee_growth_inside0_last_x128, fee_growth_inside1_last_x128))
}

pub async fn pool_manager_position_state_liquidity<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    position_manager_address: Address,
    pool_id: B256,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24,
    block_number: Option<u64>
) -> eyre::Result<u128> {
    let position_key = U256::from_be_slice(
        encode_position_key(position_manager_address, position_token_id, tick_lower, tick_upper).as_slice()
    );
    let position_state_slot = pool_manager_position_state_slot(pool_id.into(), position_key);

    let liquidity = slot_fetcher
        .storage_at(pool_manager_address, position_state_slot, block_number)
        .await?;

    Ok(liquidity.to())
}

#[cfg(test)]
mod tests {

    use alloy_primitives::aliases::U24;

    use super::*;
    use crate::{
        angstrom::mainnet::ANGSTROM_L1_CONSTANTS_MAINNET,
        test_utils::*,
        v4::{UNISWAP_V4_CONSTANTS_MAINNET, V4PoolKey}
    };

    #[tokio::test]
    async fn test_pool_manager_position_fee_growth_inside() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };

        let results = pool_manager_position_fee_growth_inside(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            pool_key.into(),
            I24::unchecked_from(190088),
            I24::unchecked_from(-887270),
            I24::unchecked_from(887270),
            Some(block_number)
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
    async fn test_pool_manager_position_state_last_fee_growth_inside() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };

        let results = pool_manager_position_state_last_fee_growth_inside(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            UNISWAP_V4_CONSTANTS_MAINNET.position_manager(),
            pool_key.into(),
            U256::from(14328_u64),
            I24::unchecked_from(-887270),
            I24::unchecked_from(887270),
            Some(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, (U256::ZERO, U256::ZERO));
    }

    #[tokio::test]
    async fn test_pool_manager_position_state_liquidity() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };

        let results = pool_manager_position_state_liquidity(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.pool_manager(),
            UNISWAP_V4_CONSTANTS_MAINNET.position_manager(),
            pool_key.into(),
            U256::from(14328_u64),
            I24::unchecked_from(-887270),
            I24::unchecked_from(887270),
            Some(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, 0);
    }
}
