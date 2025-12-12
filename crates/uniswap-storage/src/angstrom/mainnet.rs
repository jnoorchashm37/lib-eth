use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;

use crate::{StorageSlotFetcher, v4::utils::encode_position_key};

pub const ANGSTROM_POOL_REWARDS_GROWTH_ARRAY_SIZE: u64 = 16777216;
pub const BLOCKS_24HR: u64 = 7200;

pub const ANGSTROM_POSITIONS_SLOT: u8 = 6;
pub const ANGSTROM_POOL_REWARDS_SLOT: u8 = 7;

pub fn angstrom_position_slot(pool_id: B256, position_key: B256) -> B256 {
    let inner = keccak256((pool_id, U256::from(ANGSTROM_POSITIONS_SLOT)).abi_encode());
    keccak256((position_key, inner).abi_encode())
}

pub fn angstrom_pool_rewards_slot(pool_id: B256) -> B256 {
    keccak256((pool_id, U256::from(ANGSTROM_POOL_REWARDS_SLOT)).abi_encode())
}

pub async fn angstrom_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    pool_id: B256,
    current_pool_tick: I24,
    tick_lower: I24,
    tick_upper: I24,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let (lower_growth, upper_growth, global_growth) = futures::try_join!(
        angstrom_tick_growth_outside(slot_fetcher, angstrom_address, pool_id, tick_lower, block_number,),
        angstrom_tick_growth_outside(slot_fetcher, angstrom_address, pool_id, tick_upper, block_number,),
        angstrom_global_growth(slot_fetcher, angstrom_address, pool_id, block_number,),
    )?;

    let rewards = if current_pool_tick < tick_lower {
        lower_growth - upper_growth
    } else if current_pool_tick >= tick_upper {
        upper_growth - lower_growth
    } else {
        global_growth - lower_growth - upper_growth
    };

    Ok(rewards)
}

pub async fn angstrom_global_growth<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    pool_id: B256,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let pool_rewards_slot_base = U256::from_be_bytes(angstrom_pool_rewards_slot(pool_id).0);
    let global_growth = slot_fetcher
        .storage_at(
            angstrom_address,
            (pool_rewards_slot_base + U256::from(ANGSTROM_POOL_REWARDS_GROWTH_ARRAY_SIZE)).into(),
            block_number
        )
        .await?;

    Ok(global_growth)
}

pub async fn angstrom_tick_growth_outside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    pool_id: B256,
    tick: I24,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let pool_rewards_slot_base = U256::from_be_bytes(angstrom_pool_rewards_slot(pool_id).0);
    let growth_outside = slot_fetcher
        .storage_at(
            angstrom_address,
            (pool_rewards_slot_base + U256::from_be_slice(&tick.to_be_bytes::<3>())).into(),
            block_number
        )
        .await?;

    Ok(growth_outside)
}

pub async fn angstrom_last_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    position_manager_address: Address,
    pool_id: B256,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let position_key = encode_position_key(position_manager_address, position_token_id, tick_lower, tick_upper);
    let position_slot_base = U256::from_be_bytes(angstrom_position_slot(pool_id, position_key).0);

    let growth = slot_fetcher
        .storage_at(angstrom_address, position_slot_base.into(), block_number)
        .await?;

    Ok(growth)
}

#[cfg(test)]
mod tests {

    use alloy_primitives::{address, aliases::U24};

    use super::*;
    use crate::{
        test_utils::{ANGSTROM_ADDRESS, V4_POSITION_MANAGER_ADDRESS, eth_provider},
        v4::V4PoolKey
    };

    #[tokio::test]
    async fn test_angstrom_growth_inside() {
        let provider = eth_provider().await;
        let block_number = 0;

        let pool_key = V4PoolKey {
            currency0:   address!("0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"),
            currency1:   address!("0xfff9976782d46cc05630d1f6ebab18b2324d6b14"),
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_ADDRESS
        };

        let results = angstrom_growth_inside(
            &provider,
            ANGSTROM_ADDRESS,
            pool_key.into(),
            I24::unchecked_from(190088),
            I24::unchecked_from(-887270),
            I24::unchecked_from(887270),
            Some(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(701740166379348581587029420336_u128))
    }

    #[tokio::test]
    async fn test_angstrom_last_growth_inside() {
        let provider = eth_provider().await;
        let block_number = 0;

        let pool_key = V4PoolKey {
            currency0:   address!("0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"),
            currency1:   address!("0xfff9976782d46cc05630d1f6ebab18b2324d6b14"),
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_ADDRESS
        };

        let results = angstrom_last_growth_inside(
            &provider,
            ANGSTROM_ADDRESS,
            V4_POSITION_MANAGER_ADDRESS,
            pool_key.into(),
            U256::from(14328_u64),
            I24::unchecked_from(-887270),
            I24::unchecked_from(887270),
            Some(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(699096039990817998971892310067_u128))
    }

    #[tokio::test]
    async fn test_angstrom_global_growth() {
        let provider = eth_provider().await;
        let block_number = 0;

        let pool_key = V4PoolKey {
            currency0:   address!("0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"),
            currency1:   address!("0xfff9976782d46cc05630d1f6ebab18b2324d6b14"),
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_ADDRESS
        };

        let results = angstrom_global_growth(&provider, ANGSTROM_ADDRESS, pool_key.into(), Some(block_number))
            .await
            .unwrap();

        assert_eq!(results, U256::from(701740166379348581587029420336_u128))
    }

    #[tokio::test]
    async fn test_angstrom_tick_growth_outside() {
        let provider = eth_provider().await;
        let block_number = 0;

        let pool_key = V4PoolKey {
            currency0:   address!("0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"),
            currency1:   address!("0xfff9976782d46cc05630d1f6ebab18b2324d6b14"),
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_ADDRESS
        };

        let results = angstrom_tick_growth_outside(
            &provider,
            ANGSTROM_ADDRESS,
            pool_key.into(),
            I24::unchecked_from(188250),
            Some(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(655382197592272071439615771424_u128))
    }
}
