use std::sync::Arc;

use alloy_provider::{IpcConnect, RootProvider, WsConnect, builder};
use eth_network_exts::EthNetworkExt;
use reth_node_types::NodeTypes;
use reth_provider::{
    BlockNumReader, CanonStateSubscriptions, DatabaseProviderFactory, StateProviderFactory, TryIntoHistoricalStateProvider
};
use reth_rpc_eth_api::{EthApiTypes, FullEthApiServer, RpcNodeCore, helpers::FullEthApi};

use crate::{reth_libmdbx::DbConfig, traits::EthStream};

pub mod node;
#[cfg(feature = "op-reth-db")]
pub mod op_node;

pub(crate) fn provider_runtime() -> eyre::Result<reth_tasks::Runtime> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => reth_tasks::RuntimeBuilder::new(
            reth_tasks::RuntimeConfig::default().with_tokio(reth_tasks::TokioConfig::existing_handle(handle))
        )
        .build()
        .map_err(Into::into),
        Err(_) => reth_tasks::RuntimeBuilder::new(reth_tasks::RuntimeConfig::default())
            .build()
            .map_err(Into::into)
    }
}

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

    fn new_with_db<Ext>(
        db_config: DbConfig,
        max_tasks: usize,
        task_executor: reth_tasks::Runtime,
        chain: Arc<<Self as NodeTypes>::ChainSpec>,
        ipc_path_or_rpc_url: Option<String>
    ) -> eyre::Result<RethNodeClient<Ext>>
    where
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
mod revm_impl {

    use reth_provider::StateProviderFactory;
    use reth_revm::database::StateProviderDatabase;

    use super::*;
    use crate::traits::{EthRevm, EthRevmParams, reth_revm_utils::RethLibmdbxDatabaseRef};

    impl<Ext> EthRevm for RethNodeClient<Ext>
    where
        Ext: EthNetworkExt,
        Ext::RethNode: NodeClientSpec
    {
        type InnerDb = RethLibmdbxDatabaseRef;
        type Params = EthRevmParams;

        fn make_inner_db(&self, params: &EthRevmParams) -> eyre::Result<Self::InnerDb> {
            let state_provider = self.eth_db_provider().state_by_block_id(params.block_id)?;

            let this = StateProviderDatabase::new(state_provider);
            Ok(RethLibmdbxDatabaseRef::new(this))
        }
    }
}
