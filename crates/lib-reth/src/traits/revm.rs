use alloy_eips::BlockId;
use alloy_network::{Ethereum, Network};
use alloy_primitives::ChainId;
use revm::{
    Context, DatabaseRef, Journal, MainBuilder, MainContext,
    context::{BlockEnv, CfgEnv, Evm, TxEnv},
    handler::{EthFrame, EthPrecompiles, instructions::EthInstructions},
    interpreter::interpreter::EthInterpreter
};
use revm_database::CacheDB;

type NetworkRevmContext<DB, TX, CFG, CHAIN> = Context<BlockEnv, TX, CFG, CacheDB<DB>, Journal<CacheDB<DB>>, CHAIN>;

type MainnetRevmEvm<DB, TX, CFG, CHAIN> = Evm<
    NetworkRevmContext<DB, TX, CFG, CHAIN>,
    (),
    EthInstructions<EthInterpreter, NetworkRevmContext<DB, TX, CFG, CHAIN>>,
    EthPrecompiles,
    EthFrame
>;

pub trait RevmNetworkSpec: Network {
    type TX;
    type CFG;
    type CHAIN;
    type EVM<DB: DatabaseRef>;

    fn build_context<DB: DatabaseRef>(
        db: CacheDB<DB>,
        chain_id: u64
    ) -> NetworkRevmContext<DB, Self::TX, Self::CFG, Self::CHAIN>;

    fn build_evm<DB: DatabaseRef>(db: CacheDB<DB>, chain_id: u64) -> Self::EVM<DB>;
}

impl RevmNetworkSpec for Ethereum {
    type CFG = CfgEnv;
    type CHAIN = ();
    type EVM<DB: DatabaseRef> = MainnetRevmEvm<DB, Self::TX, Self::CFG, Self::CHAIN>;
    type TX = TxEnv;

    fn build_context<DB: DatabaseRef>(
        db: CacheDB<DB>,
        chain_id: u64
    ) -> NetworkRevmContext<DB, Self::TX, Self::CFG, Self::CHAIN> {
        Context::mainnet()
            .modify_cfg_chained(|cfg| cfg.chain_id = chain_id)
            .with_db(db)
    }

    fn build_evm<DB: DatabaseRef>(db: CacheDB<DB>, chain_id: u64) -> Self::EVM<DB> {
        Self::build_context(db, chain_id).build_mainnet()
    }
}

#[cfg(feature = "op-revm")]
mod op_impl {
    use op_alloy_network::Optimism;
    use op_revm::{DefaultOp, L1BlockInfo, OpBuilder, OpEvm, OpSpecId, OpTransaction, precompiles::OpPrecompiles};

    use super::*;

    type OptimismRevmEvm<DB, TX, CFG, CHAIN> = OpEvm<
        NetworkRevmContext<DB, TX, CFG, CHAIN>,
        (),
        EthInstructions<EthInterpreter, NetworkRevmContext<DB, TX, CFG, CHAIN>>,
        OpPrecompiles
    >;

    impl RevmNetworkSpec for Optimism {
        type CFG = CfgEnv<OpSpecId>;
        type CHAIN = L1BlockInfo;
        type EVM<DB: DatabaseRef> = OptimismRevmEvm<DB, Self::TX, Self::CFG, Self::CHAIN>;
        type TX = OpTransaction<TxEnv>;

        fn build_context<DB: DatabaseRef>(
            db: CacheDB<DB>,
            chain_id: u64
        ) -> NetworkRevmContext<DB, Self::TX, Self::CFG, Self::CHAIN> {
            Context::op()
                .modify_cfg_chained(|cfg| cfg.chain_id = chain_id)
                .with_db(db)
        }

        fn build_evm<DB: DatabaseRef>(db: CacheDB<DB>, chain_id: u64) -> Self::EVM<DB> {
            Self::build_context(db, chain_id).build_op()
        }
    }
}

#[auto_impl::auto_impl(&, Box, Arc)]
pub trait EthRevm<N: RevmNetworkSpec> {
    type InnerDb: DatabaseRef;
    type Params: EthRevmInput;

    /// `makes the inner database fetcher`
    fn make_inner_db(&self, params: &Self::Params) -> eyre::Result<Self::InnerDb>;

    /// `makes a new cache db`
    fn make_cache_db(&self, params: &Self::Params) -> eyre::Result<CacheDB<Self::InnerDb>> {
        Ok(CacheDB::new(self.make_inner_db(params)?))
    }

    /// `makes a new cache db`
    fn make_empty_evm(&self, params: &Self::Params) -> eyre::Result<N::EVM<Self::InnerDb>> {
        let db = self.make_cache_db(params)?;
        Ok(N::build_evm(db, params.chain_id()))
    }
}

pub trait EthRevmInput {
    fn block_id(&self) -> BlockId;

    fn chain_id(&self) -> ChainId;

    fn handle(&self) -> Option<tokio::runtime::Handle>;
}

pub struct EthRevmParams {
    pub block_id: BlockId,
    pub chain_id: ChainId
}

impl EthRevmInput for EthRevmParams {
    fn block_id(&self) -> BlockId {
        self.block_id
    }

    fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    fn handle(&self) -> Option<tokio::runtime::Handle> {
        None
    }
}

pub struct AsyncEthRevmParams {
    pub block_id: BlockId,
    pub chain_id: ChainId,
    pub handle:   tokio::runtime::Handle
}

impl EthRevmInput for AsyncEthRevmParams {
    fn block_id(&self) -> BlockId {
        self.block_id
    }

    fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    fn handle(&self) -> Option<tokio::runtime::Handle> {
        Some(self.handle.clone())
    }
}
