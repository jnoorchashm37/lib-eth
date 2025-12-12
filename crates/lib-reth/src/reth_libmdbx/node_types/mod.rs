use std::sync::Arc;

use alloy_provider::{IpcConnect, RootProvider, WsConnect, builder};
use eth_network_exts::EthNetworkExt;
use reth_node_types::NodeTypes;
use reth_provider::{
    BlockNumReader, CanonStateSubscriptions, DatabaseProviderFactory, StateProviderFactory, TryIntoHistoricalStateProvider
};
use reth_rpc_eth_api::{EthApiTypes, FullEthApiServer, RpcNodeCore, helpers::FullEthApi};
use reth_tasks::TaskSpawner;

#[cfg(feature = "revm")]
use crate::traits::BlockNumberOrHash;
use crate::{reth_libmdbx::DbConfig, traits::EthStream};

pub mod node;
#[cfg(feature = "op-reth-libmdbx")]
pub mod op_node;

pub trait NodeClientSpec: NodeTypes + Send + Sync {
    type Api: FullEthApi + FullEthApiServer + EthApiTypes + RpcNodeCore + Clone + Send + Sync;
    type Filter: Clone + Send + Sync;
    type Trace: Clone + Send + Sync;
    type Debug: Clone + Send + Sync;
    type TxPool: Clone + Send + Sync;
    type DbProvider: DatabaseProviderFactory<Provider: TryIntoHistoricalStateProvider + BlockNumReader>
        + StateProviderFactory
        + CanonStateSubscriptions
        + Send
        + Sync
        + Clone
        + 'static;

    fn new_with_db<T, Ext>(
        db_config: DbConfig,
        max_tasks: usize,
        task_executor: T,
        chain: Arc<<Self as NodeTypes>::ChainSpec>,
        ipc_path_or_rpc_url: Option<String>
    ) -> eyre::Result<RethNodeClient<Ext>>
    where
        T: TaskSpawner + Clone + 'static,
        Ext: EthNetworkExt<RethNode = Self>;
}

pub struct RethNodeClient<Ext: EthNetworkExt>
where
    Ext::RethNode: NodeClientSpec
{
    api:                 <Ext::RethNode as NodeClientSpec>::Api,
    filter:              <Ext::RethNode as NodeClientSpec>::Filter,
    trace:               <Ext::RethNode as NodeClientSpec>::Trace,
    debug:               <Ext::RethNode as NodeClientSpec>::Debug,
    tx_pool:             <Ext::RethNode as NodeClientSpec>::TxPool,
    db_provider:         <Ext::RethNode as NodeClientSpec>::DbProvider,
    chain_spec:          Arc<<Ext::RethNode as NodeTypes>::ChainSpec>,
    ipc_path_or_rpc_url: Option<String>
}

impl<Ext: EthNetworkExt> RethNodeClient<Ext>
where
    Ext::RethNode: NodeClientSpec
{
    pub fn chain_spec(&self) -> Arc<<Ext::RethNode as NodeTypes>::ChainSpec> {
        self.chain_spec.clone()
    }

    pub fn eth_api(&self) -> <Ext::RethNode as NodeClientSpec>::Api {
        self.api.clone()
    }

    pub fn eth_filter(&self) -> <Ext::RethNode as NodeClientSpec>::Filter {
        self.filter.clone()
    }

    pub fn eth_trace(&self) -> <Ext::RethNode as NodeClientSpec>::Trace {
        self.trace.clone()
    }

    pub fn eth_debug(&self) -> <Ext::RethNode as NodeClientSpec>::Debug {
        self.debug.clone()
    }

    pub fn eth_tx_pool(&self) -> <Ext::RethNode as NodeClientSpec>::TxPool {
        self.tx_pool.clone()
    }

    pub fn eth_db_provider(&self) -> &<Ext::RethNode as NodeClientSpec>::DbProvider {
        &self.db_provider
    }
}

#[async_trait::async_trait]
impl<Ext: EthNetworkExt> EthStream<Ext::AlloyNetwork> for RethNodeClient<Ext>
where
    Ext::RethNode: NodeClientSpec
{
    async fn root_provider(&self) -> eyre::Result<RootProvider<Ext::AlloyNetwork>> {
        let conn_url = self
            .ipc_path_or_rpc_url
            .as_ref()
            .ok_or_else(|| eyre::eyre!("no ipc path or rpc url has been set"))?;

        let builder = builder::<Ext::AlloyNetwork>();

        let client = if conn_url.ends_with(".ipc") {
            builder
                .connect_ipc(IpcConnect::new(conn_url.to_string()))
                .await?
        } else if conn_url.starts_with("ws:") || conn_url.contains("wss:") {
            builder.connect_ws(WsConnect::new(conn_url)).await?
        } else if conn_url.starts_with("http:") || conn_url.contains("https:") {
            builder.connect_http(conn_url.parse()?)
        } else {
            builder.connect(conn_url).await?
        };

        Ok(client)
    }
}

#[cfg(feature = "revm")]
impl<Ext: EthNetworkExt> crate::traits::EthRevm for RethNodeClient<Ext>
where
    Ext::RethNode: NodeClientSpec
{
    type InnerDb = crate::traits::reth_revm_utils::RethLibmdbxDatabaseRef;

    fn make_inner_db<T: Into<BlockNumberOrHash>>(&self, block: T) -> eyre::Result<Self::InnerDb> {
        use reth_provider::StateProviderFactory;

        let block: BlockNumberOrHash = block.into();

        let state_provider = match block {
            BlockNumberOrHash::Number(num) => self
                .eth_db_provider()
                .state_by_block_number_or_tag(num.into())?,
            BlockNumberOrHash::Hash(hash) => self.eth_db_provider().state_by_block_hash(hash)?
        };

        let this = reth_revm::database::StateProviderDatabase::new(state_provider);
        Ok(crate::traits::reth_revm_utils::RethLibmdbxDatabaseRef::new(this))
    }
}
