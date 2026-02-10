use std::sync::{Arc, Mutex};

use alloy_primitives::{Address, B256, U256};
use reth_provider::StateProviderBox;
use reth_revm::database::StateProviderDatabase;
use revm::{
    DatabaseRef,
    context_interface::DBErrorMarker,
    state::{AccountInfo, Bytecode}
};

#[derive(Clone)]
pub struct RethLibmdbxDatabaseRef(Arc<Mutex<StateProviderDatabase<StateProviderBox>>>);

impl RethLibmdbxDatabaseRef {
    pub fn new(this: StateProviderDatabase<StateProviderBox>) -> Self {
        Self(Arc::new(Mutex::new(this)))
    }

    pub fn state_provider(&self) -> StateProviderDatabase<StateProviderBox> {
        let this = self.clone();
        Arc::try_unwrap(this.0)
            .expect("multiple references to RethLibmdbxDatabaseRef")
            .into_inner()
            .expect("mutex poisoned")
    }
}

impl DatabaseRef for RethLibmdbxDatabaseRef {
    type Error = RevmUtilError;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let db = self
            .0
            .lock()
            .map_err(|e| RevmUtilError(eyre::eyre!("{e}")))?;
        reth_revm::DatabaseRef::basic_ref(&*db, address).map_err(RevmUtilError::as_value)
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let db = self
            .0
            .lock()
            .map_err(|e| RevmUtilError(eyre::eyre!("{e}")))?;
        reth_revm::DatabaseRef::code_by_hash_ref(&*db, code_hash).map_err(RevmUtilError::as_value)
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let db = self
            .0
            .lock()
            .map_err(|e| RevmUtilError(eyre::eyre!("{e}")))?;
        reth_revm::DatabaseRef::storage_ref(&*db, address, index).map_err(RevmUtilError::as_value)
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        let db = self
            .0
            .lock()
            .map_err(|e| RevmUtilError(eyre::eyre!("{e}")))?;
        reth_revm::DatabaseRef::block_hash_ref(&*db, number).map_err(RevmUtilError::as_value)
    }
}

#[derive(Debug)]
pub struct RevmUtilError(pub eyre::ErrReport);

impl std::fmt::Display for RevmUtilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RevmUtilError {}

impl DBErrorMarker for RevmUtilError {}

trait AsValue<T> {
    fn as_value(value: T) -> Self
    where
        Self: Sized;
}

impl<T: ToString> AsValue<T> for RevmUtilError {
    fn as_value(value: T) -> Self {
        RevmUtilError(eyre::eyre!("{}", value.to_string()))
    }
}

impl From<eyre::ErrReport> for RevmUtilError {
    fn from(value: eyre::ErrReport) -> Self {
        Self(value)
    }
}

#[cfg(feature = "uniswap-storage")]
mod _uniswap_storage {
    use alloy_eips::BlockId;
    use alloy_primitives::{Address, StorageKey, StorageValue};
    use eth_network_exts::EthNetworkExt;
    use reth_provider::StateProvider;
    use reth_rpc_eth_api::helpers::EthState;
    use uniswap_storage::StorageSlotFetcher;

    use crate::{
        reth_libmdbx::{NodeClientSpec, RethNodeClient},
        traits::reth_revm_utils::RethLibmdbxDatabaseRef
    };

    #[async_trait::async_trait]
    impl StorageSlotFetcher for RethLibmdbxDatabaseRef {
        async fn storage_at(&self, address: Address, key: StorageKey, block_id: BlockId) -> eyre::Result<StorageValue> {
            let _ = block_id;
            let db = self.0.lock().map_err(|e| eyre::eyre!("{e}"))?;
            Ok(db.storage(address, key)?.unwrap_or_default())
        }
    }

    #[async_trait::async_trait]
    impl<Ext: EthNetworkExt> StorageSlotFetcher for RethNodeClient<Ext>
    where
        Ext::RethNode: NodeClientSpec
    {
        async fn storage_at(
            &self,
            address: Address,
            key: StorageKey,
            block_id: BlockId
        ) -> eyre::Result<StorageValue> {
            Ok(self
                .eth_api()
                .storage_at(address, key.into(), Some(block_id))
                .await?
                .into())
        }
    }
}
