/*
╭------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------╮
| Name                               | Type                                   | Slot | Offset | Bytes | Contract                                    |
+===================================================================================================================================================+
| withdrawOnly                       | bool                                   | 0    | 0      | 1     | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| defaultProtocolSwapFeeAsMultipleE6 | uint24                                 | 0    | 1      | 3     | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| defaultProtocolTaxFeeE6            | uint24                                 | 0    | 4      | 3     | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| isVerifiedHook                     | mapping(contract AngstromL2 => bool)   | 1    | 0      | 32    | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| allHooks                           | contract AngstromL2[]                  | 2    | 0      | 32    | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| hookPoolIds                        | mapping(PoolId => contract AngstromL2) | 3    | 0      | 32    | src/AngstromL2Factory.sol:AngstromL2Factory |
╰------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------╯

AngstromL2 is `Address`
PoolId is `B256`

*/

use alloy_primitives::{Address, B256, U160, U256, keccak256};
use alloy_sol_types::SolValue;

use crate::StorageSlotFetcher;

// Storage slot constants
pub const ANGSTROM_L2_FACTORY_SLOT_0: u64 = 0; // withdrawOnly, defaultProtocolSwapFeeAsMultipleE6, defaultProtocolTaxFeeE6
pub const ANGSTROM_L2_FACTORY_IS_VERIFIED_HOOK_SLOT: u64 = 1;
pub const ANGSTROM_L2_FACTORY_ALL_HOOKS_SLOT: u64 = 2;
pub const ANGSTROM_L2_FACTORY_HOOK_POOL_IDS_SLOT: u64 = 3;

/// Gets the packed slot 0 data which contains withdrawOnly,
/// defaultProtocolSwapFeeAsMultipleE6, and defaultProtocolTaxFeeE6
pub async fn angstrom_l2_factory_get_slot_0<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    slot_fetcher
        .storage_at(factory_address, U256::from(ANGSTROM_L2_FACTORY_SLOT_0).into(), block_number)
        .await
}

/// Gets whether the factory is in withdraw-only mode
pub async fn angstrom_l2_factory_withdraw_only<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_number: Option<u64>
) -> eyre::Result<bool> {
    let slot_0 = angstrom_l2_factory_get_slot_0(slot_fetcher, factory_address, block_number).await?;
    Ok((slot_0 & U256::from(0xFF)) != U256::ZERO)
}

/// Gets the default protocol swap fee as multiple in E6 format
pub async fn angstrom_l2_factory_default_protocol_swap_fee_as_multiple_e6<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_number: Option<u64>
) -> eyre::Result<u32> {
    let slot_0 = angstrom_l2_factory_get_slot_0(slot_fetcher, factory_address, block_number).await?;
    Ok(U256::to::<u32>(&((slot_0 >> 8) & U256::from(0xFFFFFF))))
}

/// Gets the default protocol tax fee in E6 format
pub async fn angstrom_l2_factory_default_protocol_tax_fee_e6<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_number: Option<u64>
) -> eyre::Result<u32> {
    let slot_0 = angstrom_l2_factory_get_slot_0(slot_fetcher, factory_address, block_number).await?;
    Ok(U256::to::<u32>(&((slot_0 >> 32) & U256::from(0xFFFFFF))))
}

/// Checks if a hook is verified
pub async fn angstrom_l2_factory_is_verified_hook<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    hook_address: Address,
    block_number: Option<u64>
) -> eyre::Result<bool> {
    // Compute the slot for isVerifiedHook[hook_address]
    let slot = keccak256((hook_address, U256::from(ANGSTROM_L2_FACTORY_IS_VERIFIED_HOOK_SLOT)).abi_encode());

    let is_verified = slot_fetcher
        .storage_at(factory_address, slot, block_number)
        .await?;

    Ok(is_verified != U256::ZERO)
}

/// Gets the number of hooks in the allHooks array
pub async fn angstrom_l2_factory_all_hooks_length<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_number: Option<u64>
) -> eyre::Result<u64> {
    let length = slot_fetcher
        .storage_at(factory_address, U256::from(ANGSTROM_L2_FACTORY_ALL_HOOKS_SLOT).into(), block_number)
        .await?;

    Ok(length.to::<u64>())
}

/// Gets a hook address at a specific index from the allHooks array
pub async fn angstrom_l2_factory_all_hooks_at<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    index: u64,
    block_number: Option<u64>
) -> eyre::Result<Address> {
    // Calculate the slot for array element at index
    let base_slot = keccak256(U256::from(ANGSTROM_L2_FACTORY_ALL_HOOKS_SLOT).abi_encode());
    let element_slot = U256::from_be_bytes(base_slot.0) + U256::from(index);

    let hook_address = slot_fetcher
        .storage_at(factory_address, element_slot.into(), block_number)
        .await?;

    Ok(Address::from(U160::from(hook_address)))
}

/// Gets all hooks from the allHooks array
pub async fn angstrom_l2_factory_all_hooks<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_number: Option<u64>
) -> eyre::Result<Vec<Address>> {
    let length = angstrom_l2_factory_all_hooks_length(slot_fetcher, factory_address, block_number).await?;

    if length == 0 {
        return Ok(Vec::new());
    }

    let mut hooks = Vec::with_capacity(length as usize);
    for i in 0..length {
        let hook = angstrom_l2_factory_all_hooks_at(slot_fetcher, factory_address, i, block_number).await?;
        hooks.push(hook);
    }

    Ok(hooks)
}

/// Gets the hook address for a specific pool ID
pub async fn angstrom_l2_factory_hook_pool_id<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    pool_id: B256,
    block_number: Option<u64>
) -> eyre::Result<Option<Address>> {
    // Compute the slot for hookPoolIds[pool_id]
    let slot = keccak256((pool_id, U256::from(ANGSTROM_L2_FACTORY_HOOK_POOL_IDS_SLOT)).abi_encode());

    let hook_address = slot_fetcher
        .storage_at(factory_address, slot, block_number)
        .await?;

    if hook_address == U256::ZERO { Ok(None) } else { Ok(Some(Address::from(U160::from(hook_address)))) }
}
