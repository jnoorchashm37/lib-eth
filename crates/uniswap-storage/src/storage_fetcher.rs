use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{Address, StorageKey, StorageValue};
use alloy_provider::{Provider, RootProvider};
use auto_impl::auto_impl;

#[auto_impl(&, Box, Arc)]
pub trait StorageSlotFetcherSync {
    fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue>;
}

#[async_trait::async_trait]
#[auto_impl(&, Box, Arc)]
pub trait StorageSlotFetcher: Sync {
    async fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue>;
}

#[async_trait::async_trait]
impl<N: Network> StorageSlotFetcher for RootProvider<N> {
    async fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
        Ok(self
            .get_storage_at(address, key.into())
            .block_id(block_id)
            .await?)
    }
}

#[cfg(feature = "revm")]
mod revm_impls {
    use revm_database::{AlloyDB, CacheDB, DatabaseRef, WrapDatabaseAsync, async_db::DatabaseAsyncRef};

    use super::*;

    #[async_trait::async_trait]
    impl<P: Provider<N>, N: Network> StorageSlotFetcher for AlloyDB<N, P> {
        async fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let _ = block_id;
            Ok(self.storage_async_ref(address, key.into()).await?)
        }
    }

    #[async_trait::async_trait]
    impl<S: StorageSlotFetcher + DatabaseAsyncRef> StorageSlotFetcher for WrapDatabaseAsync<S> {
        async fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let _ = block_id;
            self.storage_ref(address, key.into())
                .map_err(|e| eyre::eyre!("{e:?}"))
        }
    }

    #[async_trait::async_trait]
    impl<S: StorageSlotFetcher + DatabaseRef> StorageSlotFetcher for CacheDB<S> {
        async fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let _ = block_id;
            self.storage_ref(address, key.into())
                .map_err(|e| eyre::eyre!("{e:?}"))
        }
    }

    impl<S: StorageSlotFetcherSync + DatabaseAsyncRef> StorageSlotFetcherSync for WrapDatabaseAsync<S> {
        fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let _ = block_id;
            self.storage_ref(address, key.into())
                .map_err(|e| eyre::eyre!("{e:?}"))
        }
    }

    impl<S: StorageSlotFetcherSync + DatabaseRef> StorageSlotFetcherSync for CacheDB<S> {
        fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let _ = block_id;
            self.storage_ref(address, key.into())
                .map_err(|e| eyre::eyre!("{e:?}"))
        }
    }
}

#[cfg(feature = "local-reth")]
mod reth_db_impls {

    use reth_provider::StateProviderFactory;
    use reth_rpc::{EthApi, eth::RpcNodeCore};
    use reth_rpc_convert::RpcConvert;

    use super::*;

    #[async_trait::async_trait]
    impl<N, Rpc> StorageSlotFetcher for EthApi<N, Rpc>
    where
        N: RpcNodeCore,
        Rpc: RpcConvert
    {
        async fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let state_provider = self.provider().state_by_block_id(block_id)?;

            Ok(state_provider.storage(address, key)?.unwrap_or_default())
        }
    }

    impl<N, Rpc> StorageSlotFetcherSync for EthApi<N, Rpc>
    where
        N: RpcNodeCore,
        Rpc: RpcConvert
    {
        fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let state_provider = self.provider().state_by_block_id(block_id)?;

            Ok(state_provider.storage(address, key)?.unwrap_or_default())
        }
    }

    #[async_trait::async_trait]
    impl<N, Rpc> StorageSlotFetcher for reth_optimism_rpc::OpEthApi<N, Rpc>
    where
        N: RpcNodeCore,
        Rpc: RpcConvert
    {
        async fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let state_provider = self.eth_api().provider().state_by_block_id(block_id)?;

            Ok(state_provider.storage(address, key)?.unwrap_or_default())
        }
    }
}
