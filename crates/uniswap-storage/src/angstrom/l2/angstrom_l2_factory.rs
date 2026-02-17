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
| defaultJITTaxEnabled               | bool                                   | 0    | 7      | 1     | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| defaultPriorityFeeTaxFloor         | uint256                                | 1    | 0      | 32    | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| isVerifiedHook                     | mapping(contract AngstromL2 => bool)   | 2    | 0      | 32    | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| allHooks                           | contract AngstromL2[]                  | 3    | 0      | 32    | src/AngstromL2Factory.sol:AngstromL2Factory |
|------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------|
| hookPoolIds                        | mapping(PoolId => contract AngstromL2) | 4    | 0      | 32    | src/AngstromL2Factory.sol:AngstromL2Factory |
╰------------------------------------+----------------------------------------+------+--------+-------+---------------------------------------------╯


*/

use alloy_eips::BlockId;
use alloy_primitives::{Address, B256, U160, U256, keccak256};
use alloy_sol_types::SolValue;
use futures::{StreamExt, stream::FuturesUnordered};

use crate::{StorageSlotFetcher, angstrom::l2::AngstromL2FactorySlot0};

// Storage slot constants
pub const ANGSTROM_L2_FACTORY_SLOT_0: u64 = 0; // withdrawOnly, defaultProtocolSwapFeeAsMultipleE6, defaultProtocolTaxFeeE6, defaultJITTaxEnabled
pub const ANGSTROM_L2_FACTORY_DEFAULT_PRIORITY_FEE_TAX_FLOOR_SLOT: u64 = 1;
pub const ANGSTROM_L2_FACTORY_IS_VERIFIED_HOOK_SLOT: u64 = 2;
pub const ANGSTROM_L2_FACTORY_ALL_HOOKS_SLOT: u64 = 3;
pub const ANGSTROM_L2_FACTORY_HOOK_POOL_IDS_SLOT: u64 = 4;

/// Gets the packed slot 0 data which contains withdrawOnly,
/// defaultProtocolSwapFeeAsMultipleE6, defaultProtocolTaxFeeE6, and
/// defaultJITTaxEnabled
pub async fn angstrom_l2_factory_get_slot0<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_id: BlockId
) -> eyre::Result<AngstromL2FactorySlot0> {
    let out = slot_fetcher
        .storage_at(factory_address, U256::from(ANGSTROM_L2_FACTORY_SLOT_0).into(), block_id)
        .await?;

    Ok(AngstromL2FactorySlot0::from(out))
}

/// Gets defaultPriorityFeeTaxFloor
pub async fn angstrom_l2_factory_default_priority_fee_tax_floor<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_id: BlockId
) -> eyre::Result<U256> {
    slot_fetcher
        .storage_at(factory_address, U256::from(ANGSTROM_L2_FACTORY_DEFAULT_PRIORITY_FEE_TAX_FLOOR_SLOT).into(), block_id)
        .await
}

/// Checks if a hook is verified
pub async fn angstrom_l2_factory_is_verified_hook<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    hook_address: Address,
    block_id: BlockId
) -> eyre::Result<bool> {
    // Compute the slot for isVerifiedHook[hook_address]
    let slot = keccak256((hook_address, U256::from(ANGSTROM_L2_FACTORY_IS_VERIFIED_HOOK_SLOT)).abi_encode());

    let is_verified = slot_fetcher
        .storage_at(factory_address, slot, block_id)
        .await?;

    Ok(is_verified != U256::ZERO)
}

/// Gets the number of hooks in the allHooks array
pub async fn angstrom_l2_factory_all_hooks_length<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_id: BlockId
) -> eyre::Result<u64> {
    let length = slot_fetcher
        .storage_at(factory_address, U256::from(ANGSTROM_L2_FACTORY_ALL_HOOKS_SLOT).into(), block_id)
        .await?;

    Ok(length.to::<u64>())
}

/// Gets a hook address at a specific index from the allHooks array
pub async fn angstrom_l2_factory_all_hooks_at<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    index: u64,
    block_id: BlockId
) -> eyre::Result<Address> {
    // Calculate the slot for array element at index
    let base_slot = keccak256(U256::from(ANGSTROM_L2_FACTORY_ALL_HOOKS_SLOT).abi_encode());
    let element_slot = U256::from_be_bytes(base_slot.0) + U256::from(index);

    let hook_address = slot_fetcher
        .storage_at(factory_address, element_slot.into(), block_id)
        .await?;

    Ok(Address::from(U160::from(hook_address)))
}

/// Gets all hooks from the allHooks array
pub async fn angstrom_l2_factory_all_hooks<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    block_id: BlockId
) -> eyre::Result<Vec<Address>> {
    let length = angstrom_l2_factory_all_hooks_length(slot_fetcher, factory_address, block_id).await?;

    if length == 0 {
        return Ok(Vec::new());
    }

    let hook_futs = (0..length)
        .map(|i| angstrom_l2_factory_all_hooks_at(slot_fetcher, factory_address, i, block_id))
        .collect::<FuturesUnordered<_>>();

    let hooks = hook_futs.collect::<Vec<_>>().await;

    hooks.into_iter().collect()
}

/// Gets the hook address for a specific pool ID
pub async fn angstrom_l2_factory_hook_address_for_pool_id<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    factory_address: Address,
    pool_id: B256,
    block_id: BlockId
) -> eyre::Result<Option<Address>> {
    // Compute the slot for hookPoolIds[pool_id]
    let slot = keccak256((pool_id, U256::from(ANGSTROM_L2_FACTORY_HOOK_POOL_IDS_SLOT)).abi_encode());

    let hook_address = slot_fetcher
        .storage_at(factory_address, slot, block_id)
        .await?;

    Ok((!hook_address.is_zero()).then_some(Address::from(U160::from(hook_address))))
}

#[cfg(test)]
mod test {
    use alloy_eips::BlockId;
    use alloy_primitives::{U256, address, aliases::U24, b256};

    use super::*;
    use crate::{angstrom::l2::ANGSTROM_L2_CONSTANTS_BASE_MAINNET, test_utils::eth_base_provider};

    const HOOK_ADDRESS: Address = address!("0xC7F6fFDb7a058ac431b852Bc1bF00cc0Fd4c65Cf");
    const POOL_ID: B256 = b256!("0x343ee3036741f45b5512ebf7ad0d8ab259dbb8e5a38ff0d19022da176ee04574");
    const BLOCK_NUMBER: u64 = 40426000;

    #[tokio::test]
    async fn test_angstrom_l2_factory_get_slot0() {
        let provider = eth_base_provider().await;

        let result = angstrom_l2_factory_get_slot0(
            &provider,
            ANGSTROM_L2_CONSTANTS_BASE_MAINNET.angstrom_l2_factory(),
            BlockId::number(BLOCK_NUMBER)
        )
        .await
        .unwrap();

        let expected = AngstromL2FactorySlot0 {
            withdraw_only: false,
            default_protocol_swap_fee_as_multiple_e6: U24::ZERO,
            default_protocol_tax_fee_e6: U24::ZERO,
            default_jit_tax_enabled: false
        };

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_angstrom_l2_factory_default_priority_fee_tax_floor() {
        let provider = eth_base_provider().await;

        let result = angstrom_l2_factory_default_priority_fee_tax_floor(
            &provider,
            ANGSTROM_L2_CONSTANTS_BASE_MAINNET.angstrom_l2_factory(),
            BlockId::number(BLOCK_NUMBER)
        )
        .await
        .unwrap();

        assert_eq!(result, U256::ZERO);
    }

    #[tokio::test]
    async fn test_angstrom_l2_factory_is_verified_hook() {
        let provider = eth_base_provider().await;

        let result = angstrom_l2_factory_is_verified_hook(
            &provider,
            ANGSTROM_L2_CONSTANTS_BASE_MAINNET.angstrom_l2_factory(),
            HOOK_ADDRESS,
            BlockId::number(BLOCK_NUMBER)
        )
        .await
        .unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_angstrom_l2_factory_all_hooks_length() {
        let provider = eth_base_provider().await;

        let result = angstrom_l2_factory_all_hooks_length(
            &provider,
            ANGSTROM_L2_CONSTANTS_BASE_MAINNET.angstrom_l2_factory(),
            BlockId::number(BLOCK_NUMBER)
        )
        .await
        .unwrap();

        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_angstrom_l2_factory_all_hooks_at() {
        let provider = eth_base_provider().await;

        let result = angstrom_l2_factory_all_hooks_at(
            &provider,
            ANGSTROM_L2_CONSTANTS_BASE_MAINNET.angstrom_l2_factory(),
            0,
            BlockId::number(BLOCK_NUMBER)
        )
        .await
        .unwrap();

        assert_eq!(result, HOOK_ADDRESS);
    }

    #[tokio::test]
    async fn test_angstrom_l2_factory_all_hooks() {
        let provider = eth_base_provider().await;

        let result = angstrom_l2_factory_all_hooks(
            &provider,
            ANGSTROM_L2_CONSTANTS_BASE_MAINNET.angstrom_l2_factory(),
            BlockId::number(BLOCK_NUMBER)
        )
        .await
        .unwrap();

        assert_eq!(result, vec![HOOK_ADDRESS]);
    }

    #[tokio::test]
    async fn test_angstrom_l2_factory_hook_address_for_pool_id() {
        let provider = eth_base_provider().await;

        let result = angstrom_l2_factory_hook_address_for_pool_id(
            &provider,
            ANGSTROM_L2_CONSTANTS_BASE_MAINNET.angstrom_l2_factory(),
            POOL_ID,
            BlockId::number(BLOCK_NUMBER)
        )
        .await
        .unwrap()
        .unwrap();

        assert_eq!(result, HOOK_ADDRESS);
    }
}
