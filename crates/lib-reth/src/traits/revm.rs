use alloy_eips::BlockId;
use alloy_primitives::ChainId;
use revm::{
    Context, DatabaseRef, Journal, MainBuilder, MainContext,
    context::{BlockEnv, CfgEnv, Evm, TxEnv},
    handler::{EthFrame, EthPrecompiles, instructions::EthInstructions},
    interpreter::interpreter::EthInterpreter
};
use revm_database::CacheDB;
type NetworkRevmContext<DB, TX, CFG, CHAIN> = Context<BlockEnv, TX, CFG, CacheDB<DB>, Journal<CacheDB<DB>>, CHAIN>;

type MainnetRevmEvm<DB> = Evm<
    NetworkRevmContext<DB, TxEnv, CfgEnv, ()>,
    (),
    EthInstructions<EthInterpreter, NetworkRevmContext<DB, TxEnv, CfgEnv, ()>>,
    EthPrecompiles,
    EthFrame
>;

pub fn empty_mainnet_revm<DB: DatabaseRef>(db: CacheDB<DB>, chain_id: ChainId) -> MainnetRevmEvm<DB> {
    Context::mainnet()
        .modify_cfg_chained(|cfg| cfg.chain_id = chain_id)
        .with_db(db)
        .build_mainnet()
}

#[cfg(feature = "op-revm")]
pub use op_impl::empty_op_mainnet_revm;
#[cfg(feature = "op-revm")]
pub use op_revm::OpTransaction;

#[cfg(feature = "op-revm")]
mod op_impl {
    use op_revm::{DefaultOp, L1BlockInfo, OpBuilder, OpEvm, OpSpecId, OpTransaction, precompiles::OpPrecompiles};
    use revm::handler::EvmTr;

    use super::*;

    type OptimismRevmEvm<DB> = OpEvm<
        NetworkRevmContext<DB, OpTransaction<TxEnv>, CfgEnv<OpSpecId>, L1BlockInfo>,
        (),
        EthInstructions<EthInterpreter, NetworkRevmContext<DB, OpTransaction<TxEnv>, CfgEnv<OpSpecId>, L1BlockInfo>>,
        OpPrecompiles
    >;

    pub fn empty_op_mainnet_revm<DB: DatabaseRef>(
        db: CacheDB<DB>,
        chain_id: ChainId,
        disable_nonce_check: bool
    ) -> OptimismRevmEvm<DB> {
        let mut evm = Context::op()
            .modify_cfg_chained(|cfg| cfg.chain_id = chain_id)
            .with_db(db)
            .build_op();

        if disable_nonce_check {
            evm.ctx_mut()
                .modify_cfg(|cfg| cfg.disable_nonce_check = true);
        }

        evm
    }
}

#[auto_impl::auto_impl(&, Box, Arc)]
pub trait EthRevm {
    type InnerDb: DatabaseRef;
    type Params: EthRevmInput;

    /// `makes the inner database fetcher`
    fn make_inner_db(&self, params: &Self::Params) -> eyre::Result<Self::InnerDb>;

    /// `makes a new cache db`
    fn make_cache_db(&self, params: &Self::Params) -> eyre::Result<CacheDB<Self::InnerDb>> {
        Ok(CacheDB::new(self.make_inner_db(params)?))
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
