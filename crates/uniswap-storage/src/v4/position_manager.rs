use alloy_primitives::{
    Address, B256, Bytes, U256,
    aliases::{I24, U24},
    keccak256
};
use alloy_sol_types::SolValue;
use itertools::concat;

use crate::{
    StorageSlotFetcher,
    v4::{UnpackPositionInfo, UnpackedPositionInfo, V4PoolKey}
};

pub const POSITION_MANAGER_OWNER_OF_SLOT: u8 = 2;
pub const POSITION_MANAGER_NEXT_TOKEN_ID_SLOT: u8 = 8;
pub const POSITION_MANAGER_POSITION_INFO_SLOT: u8 = 9;
pub const POSITION_MANAGER_POOL_KEYS_SLOT: u8 = 10;

pub fn position_manager_owner_of_slot(token_id: U256) -> B256 {
    keccak256((token_id, U256::from(POSITION_MANAGER_OWNER_OF_SLOT)).abi_encode())
}

pub fn position_manager_position_info_slot(token_id: U256) -> B256 {
    keccak256((token_id, U256::from(POSITION_MANAGER_POSITION_INFO_SLOT)).abi_encode())
}

pub fn position_manager_pool_key_and_info_slot(position_info: U256) -> B256 {
    let position_id = position_info.position_manager_pool_map_key();
    keccak256((position_id, U256::from(POSITION_MANAGER_POOL_KEYS_SLOT)).abi_encode())
}

pub async fn position_manager_position_info<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    position_manager_address: Address,
    block_number: Option<u64>,
    token_id: U256
) -> eyre::Result<U256> {
    let position_info_slot = position_manager_position_info_slot(token_id);

    let position_info = slot_fetcher
        .storage_at(position_manager_address, position_info_slot, block_number)
        .await?;

    Ok(position_info)
}

pub async fn position_manager_pool_key_and_info<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    position_manager_address: Address,
    block_number: Option<u64>,
    token_id: U256
) -> eyre::Result<(V4PoolKey, UnpackedPositionInfo)> {
    let position_info =
        position_manager_position_info(slot_fetcher, position_manager_address, block_number, token_id).await?;
    let pool_key_slot_base = U256::from_be_slice(position_manager_pool_key_and_info_slot(position_info).as_slice());

    let (slot0, slot1, slot2) = futures::try_join!(
        slot_fetcher.storage_at(position_manager_address, pool_key_slot_base.into(), block_number),
        slot_fetcher.storage_at(position_manager_address, (pool_key_slot_base + U256::from(1_u8)).into(), block_number),
        slot_fetcher.storage_at(position_manager_address, (pool_key_slot_base + U256::from(2_u8)).into(), block_number)
    )?;

    let concatted_bytes = Bytes::from(concat([slot0.to_be_bytes_vec(), slot1.to_be_bytes_vec(), slot2.to_be_bytes_vec()]));

    let currency0 = Address::from_slice(&concatted_bytes[12..32]);
    let currency1 = Address::from_slice(&concatted_bytes[44..64]);
    let fee = U24::from_be_slice(&concatted_bytes[41..44]);
    let tick_spacing = I24::try_from_be_slice(&concatted_bytes[38..41]).unwrap();
    let hooks = Address::from_slice(&concatted_bytes[76..96]);

    let pool_key = V4PoolKey { currency0, currency1, fee, tickSpacing: tick_spacing, hooks };

    Ok((pool_key, position_info.unpack_position_info()))
}

pub async fn position_manager_owner_of<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    position_manager_address: Address,
    block_number: Option<u64>,
    token_id: U256
) -> eyre::Result<Address> {
    let owner_of_slot = position_manager_owner_of_slot(token_id);

    let owner_of = slot_fetcher
        .storage_at(position_manager_address, owner_of_slot, block_number)
        .await?;

    Ok(Address::from_slice(&owner_of.to_be_bytes_vec()[12..32]))
}

pub async fn position_manager_next_token_id<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    position_manager_address: Address,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let next_token_id = slot_fetcher
        .storage_at(position_manager_address, U256::from(POSITION_MANAGER_NEXT_TOKEN_ID_SLOT).into(), block_number)
        .await?;

    Ok(next_token_id)
}

#[cfg(test)]
mod tests {

    use alloy_primitives::address;

    use super::*;
    use crate::{
        angstrom::mainnet::ANGSTROM_L1_CONSTANTS_MAINNET,
        test_utils::{USDC, WETH, eth_provider},
        v4::{UNISWAP_V4_CONSTANTS_MAINNET, V4PoolKey}
    };

    #[tokio::test]
    async fn test_position_manager_position_info() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let results = position_manager_position_info(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.position_manager(),
            Some(block_number),
            U256::from(96348_u64)
        )
        .await
        .unwrap();

        let expected =
            U256::from_str_radix("103579870420054752514615478328607582000546878402072520800256981255545022515712", 10)
                .unwrap();
        assert_eq!(results, expected);
    }

    #[tokio::test]
    async fn test_position_manager_pool_key_and_info() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let pool_key = V4PoolKey {
            currency0:   USDC,
            currency1:   WETH,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(10),
            hooks:       ANGSTROM_L1_CONSTANTS_MAINNET.angstrom_address()
        };

        let (results, _) = position_manager_pool_key_and_info(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.position_manager(),
            Some(block_number),
            U256::from(96348_u64)
        )
        .await
        .unwrap();

        assert_eq!(results, pool_key);
    }

    #[tokio::test]
    async fn test_position_manager_owner_of() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let results = position_manager_owner_of(
            &provider,
            UNISWAP_V4_CONSTANTS_MAINNET.position_manager(),
            Some(block_number),
            U256::from(96348_u64)
        )
        .await
        .unwrap();

        assert_eq!(results, address!("0xd73326c52ce804cef611c63659e5ea35e4e246e5"));
    }

    #[tokio::test]
    async fn test_position_manager_next_token_id() {
        let provider = eth_provider().await;
        let block_number = 23998000;

        let results =
            position_manager_next_token_id(&provider, UNISWAP_V4_CONSTANTS_MAINNET.position_manager(), Some(block_number))
                .await
                .unwrap();

        assert_eq!(results, U256::from(111343_u128));
    }
}
