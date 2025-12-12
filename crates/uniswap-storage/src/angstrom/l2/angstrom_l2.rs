/*
╭-----------------------+-----------------------------------------------------------+------+--------+-------+-------------------------------╮
| Name                  | Type                                                      | Slot | Offset | Bytes | Contract                      |
+===========================================================================================================================================+
| rewards               | mapping(PoolId => struct PoolRewards)                     | 0    | 0      | 32    | src/AngstromL2.sol:AngstromL2 |
|-----------------------+-----------------------------------------------------------+------+--------+-------+-------------------------------|
| _cachedWithdrawOnly   | bool                                                      | 1    | 0      | 1     | src/AngstromL2.sol:AngstromL2 |
|-----------------------+-----------------------------------------------------------+------+--------+-------+-------------------------------|
| _poolFeeConfiguration | mapping(PoolId => struct AngstromL2.PoolFeeConfiguration) | 2    | 0      | 32    | src/AngstromL2.sol:AngstromL2 |
|-----------------------+-----------------------------------------------------------+------+--------+-------+-------------------------------|
| liquidityBeforeSwap   | struct tuint256                                           | 3    | 0      | 32    | src/AngstromL2.sol:AngstromL2 |
|-----------------------+-----------------------------------------------------------+------+--------+-------+-------------------------------|
| slot0BeforeSwapStore  | struct tbytes32                                           | 4    | 0      | 32    | src/AngstromL2.sol:AngstromL2 |
|-----------------------+-----------------------------------------------------------+------+--------+-------+-------------------------------|
| poolKeys              | struct PoolKey[]                                          | 5    | 0      | 32    | src/AngstromL2.sol:AngstromL2 |
╰-----------------------+-----------------------------------------------------------+------+--------+-------+-------------------------------╯

struct PoolRewards {
    mapping(bytes32 uniPositionKey => Position position) positions;
    mapping(int24 tick => uint256 growthOutsideX128) rewardGrowthOutsideX128;
    uint256 globalGrowthX128;
}

struct Position {
    uint256 lastGrowthInsideX128;
}
*/

use std::pin::Pin;

use alloy_primitives::{
    Address, B256, U160, U256,
    aliases::{I24, U24},
    keccak256
};
use alloy_sol_types::SolValue;
use futures::{Stream, StreamExt, stream::FuturesUnordered};

use crate::{
    StorageSlotFetcher,
    v4::{V4PoolKey, utils::encode_position_key}
};

pub const ANGSTROM_L2_REWARDS_SLOT: u64 = 0;
pub const ANGSTROM_L2_POOL_FEE_CONFIG_SLOT: u64 = 2;
pub const ANGSTROM_L2_POOL_KEYS_SLOT: u64 = 5;

pub async fn angstrom_l2_pool_keys_stream<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    block_number: Option<u64>
) -> eyre::Result<Option<Pin<Box<dyn Stream<Item = eyre::Result<V4PoolKey>> + Send + '_>>>> {
    let length_slot = U256::from(ANGSTROM_L2_POOL_KEYS_SLOT);
    let array_length = slot_fetcher
        .storage_at(hook_address, length_slot.into(), block_number)
        .await?;

    let length = array_length.to::<u64>();

    if length == 0 {
        return Ok(None);
    }

    let array_data_base_slot = keccak256(length_slot.abi_encode());
    let base_slot_value = U256::from_be_bytes(array_data_base_slot.0);

    let stream = (0..length)
        .map(|i| async move {
            let element_offset = i * 5;

            let futures = (0..5).map(|j| {
                let slot = base_slot_value + U256::from(element_offset) + U256::from(j);
                slot_fetcher.storage_at(hook_address, slot.into(), block_number)
            });

            let slots: Vec<U256> = futures::future::try_join_all(futures).await?;

            let pool_key = V4PoolKey {
                currency0:   Address::from(U160::from(slots[0])),
                currency1:   Address::from(U160::from(slots[1])),
                fee:         U24::from(slots[2].to::<u32>() & 0xFFFFFF),
                tickSpacing: I24::unchecked_from(((slots[3].to::<u32>() & 0xFFFFFF) as i32) << 8 >> 8),
                hooks:       Address::from(U160::from(slots[4]))
            };

            eyre::Ok(pool_key)
        })
        .collect::<FuturesUnordered<_>>();

    Ok(Some(Box::pin(stream)))
}

pub async fn angstrom_l2_pool_keys_filter<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    block_number: Option<u64>,
    search_fn: impl Fn(V4PoolKey) -> bool,
    find_count: usize
) -> eyre::Result<Vec<V4PoolKey>> {
    assert_ne!(find_count, 0);

    let mut pool_keys = Vec::new();
    if let Some(mut key_stream) = angstrom_l2_pool_keys_stream(slot_fetcher, hook_address, block_number).await? {
        while let Some(pool_key_res) = key_stream.next().await {
            let pool_key = pool_key_res?;
            if search_fn(pool_key) {
                pool_keys.push(pool_key);
                if pool_keys.len() == find_count {
                    return Ok(pool_keys);
                }
            }
        }
    }

    Ok(pool_keys)
}

pub fn angstrom_l2_position_slot(pool_id: B256, position_key: B256) -> B256 {
    let rewards_pool_slot = keccak256((pool_id, U256::from(ANGSTROM_L2_REWARDS_SLOT)).abi_encode());
    keccak256((position_key, rewards_pool_slot).abi_encode())
}

pub fn angstrom_l2_pool_rewards_slot(pool_id: B256) -> B256 {
    keccak256((pool_id, U256::from(ANGSTROM_L2_REWARDS_SLOT)).abi_encode())
}

pub async fn angstrom_l2_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    pool_id: B256,
    current_pool_tick: I24,
    tick_lower: I24,
    tick_upper: I24,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let (lower_growth, upper_growth, global_growth) = futures::try_join!(
        angstrom_l2_tick_growth_outside(slot_fetcher, hook_address, pool_id, tick_lower, block_number,),
        angstrom_l2_tick_growth_outside(slot_fetcher, hook_address, pool_id, tick_upper, block_number,),
        angstrom_l2_global_growth(slot_fetcher, hook_address, pool_id, block_number,),
    )?;

    let rewards = if current_pool_tick < tick_lower {
        lower_growth.wrapping_sub(upper_growth)
    } else if current_pool_tick >= tick_upper {
        upper_growth.wrapping_sub(lower_growth)
    } else {
        global_growth
            .wrapping_sub(lower_growth)
            .wrapping_sub(upper_growth)
    };

    Ok(rewards)
}

pub async fn angstrom_l2_global_growth<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    pool_id: B256,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let pool_rewards_slot_base = U256::from_be_bytes(angstrom_l2_pool_rewards_slot(pool_id).0);
    let global_growth = slot_fetcher
        .storage_at(hook_address, (pool_rewards_slot_base + U256::from(2)).into(), block_number)
        .await?;

    Ok(global_growth)
}

pub async fn angstrom_l2_tick_growth_outside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    pool_id: B256,
    tick: I24,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let pool_rewards_slot_base = U256::from_be_bytes(angstrom_l2_pool_rewards_slot(pool_id).0);
    let tick_mapping_slot = pool_rewards_slot_base + U256::from(1);

    let tick_growth_slot = keccak256((tick, tick_mapping_slot).abi_encode());

    let growth_outside = slot_fetcher
        .storage_at(hook_address, tick_growth_slot, block_number)
        .await?;

    Ok(growth_outside)
}

pub async fn angstrom_l2_last_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    position_manager_address: Address,
    pool_id: B256,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let position_key = encode_position_key(position_manager_address, position_token_id, tick_lower, tick_upper);
    let position_slot = U256::from_be_bytes(angstrom_l2_position_slot(pool_id, position_key).0);

    let growth = slot_fetcher
        .storage_at(hook_address, position_slot.into(), block_number)
        .await?;

    Ok(growth)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AngstromL2PoolFeeConfiguration {
    pub is_initialized:       bool,
    pub creator_tax_fee_e6:   u32,
    pub protocol_tax_fee_e6:  u32,
    pub creator_swap_fee_e6:  u32,
    pub protocol_swap_fee_e6: u32
}

pub fn angstrom_l2_pool_fee_config_slot(pool_id: B256) -> B256 {
    keccak256((pool_id, U256::from(ANGSTROM_L2_POOL_FEE_CONFIG_SLOT)).abi_encode())
}

pub async fn angstrom_l2_pool_fee_config<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    pool_id: B256,
    block_number: Option<u64>
) -> eyre::Result<AngstromL2PoolFeeConfiguration> {
    let config_slot = angstrom_l2_pool_fee_config_slot(pool_id);

    let packed_config = slot_fetcher
        .storage_at(hook_address, config_slot, block_number)
        .await?;

    let is_initialized = (packed_config & U256::from(1)) != U256::ZERO;
    let creator_tax_fee_e6 = U256::to::<u32>(&((packed_config >> 8) & U256::from(0xFFFFFF)));
    let protocol_tax_fee_e6 = U256::to::<u32>(&((packed_config >> 32) & U256::from(0xFFFFFF)));
    let creator_swap_fee_e6 = U256::to::<u32>(&((packed_config >> 56) & U256::from(0xFFFFFF)));
    let protocol_swap_fee_e6 = U256::to::<u32>(&((packed_config >> 80) & U256::from(0xFFFFFF)));

    Ok(AngstromL2PoolFeeConfiguration {
        is_initialized,
        creator_tax_fee_e6,
        protocol_tax_fee_e6,
        creator_swap_fee_e6,
        protocol_swap_fee_e6
    })
}

/// Checks if a pool is initialized
pub async fn angstrom_l2_is_pool_initialized<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    pool_id: B256,
    block_number: Option<u64>
) -> eyre::Result<bool> {
    let config = angstrom_l2_pool_fee_config(slot_fetcher, hook_address, pool_id, block_number).await?;
    Ok(config.is_initialized)
}

/// Gets the total swap fee rate (creator + protocol) in E6 format
pub async fn angstrom_l2_total_swap_fee_rate_e6<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    pool_id: B256,
    block_number: Option<u64>
) -> eyre::Result<u32> {
    let config = angstrom_l2_pool_fee_config(slot_fetcher, hook_address, pool_id, block_number).await?;
    Ok(config.creator_swap_fee_e6 + config.protocol_swap_fee_e6)
}

/// Gets the total tax fee rate (creator + protocol) in E6 format
pub async fn angstrom_l2_total_tax_fee_rate_e6<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    hook_address: Address,
    pool_id: B256,
    block_number: Option<u64>
) -> eyre::Result<u32> {
    let config = angstrom_l2_pool_fee_config(slot_fetcher, hook_address, pool_id, block_number).await?;
    Ok(config.creator_tax_fee_e6 + config.protocol_tax_fee_e6)
}
