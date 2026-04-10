use std::sync::Arc;

use alloy_primitives::U256;
use eth_network_exts::EthNetworkExt;
use op_alloy_network::Optimism;
use reth_db::{DatabaseEnv, open_db_read_only};
use reth_network_api::noop::NoopNetwork;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_optimism_chainspec::OpChainSpec;
use reth_optimism_evm::OpEvmConfig;
use reth_optimism_node::{
    OpNode,
    txpool::{OpPooledTransaction, OpTransactionValidator}
};
use reth_optimism_rpc::{
    OpEthApi,
    eth::{receipt::OpReceiptConverter, transaction::OpTxInfoMapper}
};
use reth_provider::{
    ProviderFactory,
    providers::{BlockchainProvider, RocksDBProvider, StaticFileProvider}
};
use reth_rpc::{DebugApi, EthApi, EthFilter, TraceApi};
use reth_rpc_eth_api::{RpcConverter, node::RpcNodeCoreAdapter};
use reth_rpc_eth_types::{EthConfig, EthFilterConfig};
use reth_tasks::{Runtime, pool::BlockingTaskGuard};
use reth_transaction_pool::{
    CoinbaseTipOrdering, Pool, PoolConfig, TransactionValidationTaskExecutor, blobstore::NoopBlobStore,
    validate::EthTransactionValidatorBuilder
};

use crate::reth_libmdbx::{DbConfig, NodeClientSpec, RethNodeClient};

type OpRethApi = OpEthApi<
    RpcNodeCoreAdapter<OpRethDbProvider, OpRethTxPool, NoopNetwork, OpEvmConfig>,
    RpcConverter<
        Optimism,
        OpEvmConfig,
        OpReceiptConverter<OpRethDbProvider>,
        (),
        OpTxInfoMapper<OpRethDbProvider>,
        (),
        (),
        reth_optimism_evm::tx::OpTxEnvConverter
    >
>;
type OpRethFilter = EthFilter<OpRethApi>;
type OpRethTrace = TraceApi<OpRethApi>;
type OpRethDebug = DebugApi<OpRethApi>;
type OpRethTxPool = Pool<
    TransactionValidationTaskExecutor<OpTransactionValidator<OpRethDbProvider, OpPooledTransaction, OpEvmConfig>>,
    CoinbaseTipOrdering<OpPooledTransaction>,
    NoopBlobStore
>;

type OpRethDbProvider = BlockchainProvider<NodeTypesWithDBAdapter<OpNode, Arc<DatabaseEnv>>>;

impl NodeClientSpec for OpNode {
    type Api = OpRethApi;
    type DbProvider = OpRethDbProvider;
    type Debug = OpRethDebug;
    type Filter = OpRethFilter;
    type Trace = OpRethTrace;
    type TxPool = OpRethTxPool;

    fn new_with_db<Ext>(
        db_config: DbConfig,
        max_tasks: usize,
        task_executor: Runtime,
        chain_spec: Arc<Self::ChainSpec>,
        ipc_path_or_rpc_url: Option<String>
    ) -> eyre::Result<RethNodeClient<Ext>>
    where
        Ext: EthNetworkExt<RethNode = Self>
    {
        let db = Arc::new(open_db_read_only(db_config.db_path, db_config.db_args)?);

        let static_file_provider = StaticFileProvider::read_only(db_config.static_files_path.clone())?;
        let rocksdb_provider = RocksDBProvider::builder(&db_config.rocksdb_path)
            .with_default_tables()
            .build()?;
        let provider_factory = ProviderFactory::new(
            db,
            chain_spec.clone(),
            static_file_provider,
            rocksdb_provider,
            super::provider_runtime()?
        )?;

        let blockchain_provider = BlockchainProvider::new(provider_factory.clone())?;

        let evm_config = OpEvmConfig::optimism(chain_spec.clone());

        let transaction_validator = EthTransactionValidatorBuilder::new(blockchain_provider.clone(), evm_config.clone())
            .build_with_tasks(task_executor.clone(), NoopBlobStore::default())
            .map(OpTransactionValidator::new);

        let tx_pool = Pool::new(
            transaction_validator,
            CoinbaseTipOrdering::default(),
            NoopBlobStore::default(),
            PoolConfig::default()
        );

        let rpc_converter = RpcConverter::new(OpReceiptConverter::new(blockchain_provider.clone()))
            .with_mapper(OpTxInfoMapper::new(blockchain_provider.clone()))
            .with_tx_env_converter(reth_optimism_evm::tx::OpTxEnvConverter);

        let eth_api_inner =
            EthApi::builder(blockchain_provider.clone(), tx_pool.clone(), NoopNetwork::default(), evm_config)
                .task_spawner(task_executor.clone())
                .with_rpc_converter(rpc_converter)
                .build_inner();
        let api = OpEthApi::new(eth_api_inner, None, U256::from(1_000_000u64), None);

        let tracing_call_guard = BlockingTaskGuard::new(max_tasks);
        let trace = TraceApi::new(api.clone(), tracing_call_guard.clone(), EthConfig::default());

        let debug = DebugApi::new(api.clone(), tracing_call_guard, &task_executor, futures::stream::empty());
        let filter = EthFilter::new(api.clone(), EthFilterConfig::default(), task_executor.clone());

        Ok(RethNodeClient {
            api,
            trace,
            filter,
            debug,
            tx_pool,
            db_provider: blockchain_provider,
            chain_spec,
            ipc_path_or_rpc_url
        })
    }
}

pub fn get_op_superchain_spec(str: &str) -> Arc<OpChainSpec> {
    reth_optimism_chainspec::generated_chain_value_parser(str).unwrap()
}

#[cfg(all(test, not(feature = "ci")))]
mod tests {
    use alloy_rpc_types::Filter;
    use eth_network_exts::base_mainnet::BaseMainnetExt;
    use reth_optimism_chainspec::BASE_MAINNET;

    use crate::{reth_libmdbx::RethNodeClientBuilder, test_utils::stream_timeout, traits::EthStream};

    const BASE_MAINNET_DB_PATH: &str = "/var/lib/eth/base-mainnet/reth/";
    const BASE_MAINNET_IPC_PATH: &str = "/tmp/reth-base-mainnet.ipc";

    #[tokio::test]
    #[serial_test::serial]
    async fn can_build() {
        let builder = RethNodeClientBuilder::<BaseMainnetExt>::new(
            BASE_MAINNET_DB_PATH,
            1000,
            BASE_MAINNET.clone(),
            Some(BASE_MAINNET_IPC_PATH),
            None
        );
        assert!(builder.build().is_ok())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial]
    async fn test_block_stream() {
        let builder = RethNodeClientBuilder::<BaseMainnetExt>::new(
            BASE_MAINNET_DB_PATH,
            1000,
            BASE_MAINNET.clone(),
            Some(BASE_MAINNET_IPC_PATH),
            None
        );
        let client = builder.build().unwrap();

        let block_stream = client.block_stream().await.unwrap();
        assert!(stream_timeout(block_stream, 2, 30).await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial]
    async fn test_log_stream() {
        let builder = RethNodeClientBuilder::<BaseMainnetExt>::new(
            BASE_MAINNET_DB_PATH,
            1000,
            BASE_MAINNET.clone(),
            Some(BASE_MAINNET_IPC_PATH),
            None
        );
        let client = builder.build().unwrap();

        let log_stream = client.log_stream(Filter::new()).await.unwrap();
        assert!(stream_timeout(log_stream, 2, 30).await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial]
    async fn test_full_pending_transaction_stream() {
        let builder = RethNodeClientBuilder::<BaseMainnetExt>::new(
            BASE_MAINNET_DB_PATH,
            1000,
            BASE_MAINNET.clone(),
            Some(BASE_MAINNET_IPC_PATH),
            None
        );
        let client = builder.build().unwrap();

        let mempool_full_stream = client.full_pending_transaction_stream().await.unwrap();
        assert!(stream_timeout(mempool_full_stream, 2, 30).await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial_test::serial]
    async fn test_pending_transaction_hashes_stream() {
        let builder = RethNodeClientBuilder::<BaseMainnetExt>::new(
            BASE_MAINNET_DB_PATH,
            1000,
            BASE_MAINNET.clone(),
            Some(BASE_MAINNET_IPC_PATH),
            None
        );
        let client = builder.build().unwrap();

        let mempool_hash_stream = client.pending_transaction_hashes_stream().await.unwrap();
        assert!(stream_timeout(mempool_hash_stream, 2, 30).await.is_ok());
    }
}
