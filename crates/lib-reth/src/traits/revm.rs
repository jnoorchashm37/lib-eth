use alloy_eips::BlockId;
use alloy_network::{Ethereum, Network};
use alloy_primitives::ChainId;
use revm::{
    Context, DatabaseRef, ExecuteEvm, Journal, MainBuilder, MainContext,
    context::{
        BlockEnv, CfgEnv, Evm, Transaction, TxEnv,
        result::{EVMError, ExecutionResult, HaltReason, InvalidTransaction}
    },
    handler::{EthFrame, EthPrecompiles, instructions::EthInstructions},
    interpreter::interpreter::EthInterpreter,
    state::EvmState
};
use revm_database::CacheDB;
type NetworkRevmContext<DB, TX, CFG, CHAIN> = Context<BlockEnv, TX, CFG, CacheDB<DB>, Journal<CacheDB<DB>>, CHAIN>;

type MainnetRevmEvm<DB, TX, CFG, CHAIN, INSP> = Evm<
    NetworkRevmContext<DB, TX, CFG, CHAIN>,
    INSP,
    EthInstructions<EthInterpreter, NetworkRevmContext<DB, TX, CFG, CHAIN>>,
    EthPrecompiles,
    EthFrame
>;

/*

    // type ExecutionResult = ExecutionResult<HaltReason>;
    // type State = EvmState;
    type Error = EVMError<<CTX::Db as Database>::Error, InvalidTransaction>;
    // type Tx = <CTX as ContextTr>::Tx;
    // type Block = <CTX as ContextTr>::Block;



    // type Tx = <CTX as ContextTr>::Tx;
    // type Block = <CTX as ContextTr>::Block;
    // type State = EvmState;
    type Error = OpError<CTX>;
    // type ExecutionResult = ExecutionResult<OpHaltReason>;

*/

pub trait RevmNetworkSpec: Network {
    type TX: Transaction;
    type CFG;
    type CHAIN;

    type EvmHaltReason;
    type EvmError<DB: DatabaseRef>: Send + Sync + std::error::Error;

    type EVM<DB: DatabaseRef, INSP>: ExecuteEvm<
            Tx = Self::TX,
            State = EvmState,
            Block = BlockEnv,
            ExecutionResult = ExecutionResult<Self::EvmHaltReason>,
            Error = Self::EvmError<DB>
        >;

    fn convert_build_tx(
        tx: TxEnv,
        #[cfg(feature = "op-revm")] op_mods: impl FnMut(&mut op_revm::OpTransaction<TxEnv>)
    ) -> Self::TX;

    fn build_context<DB: DatabaseRef>(
        db: CacheDB<DB>,
        chain_id: u64
    ) -> NetworkRevmContext<DB, Self::TX, Self::CFG, Self::CHAIN>;

    fn build_evm<DB: DatabaseRef>(db: CacheDB<DB>, chain_id: u64) -> Self::EVM<DB, ()>;

    fn build_evm_with_inspector<DB: DatabaseRef, INSP>(
        db: CacheDB<DB>,
        chain_id: u64,
        inspector: INSP
    ) -> Self::EVM<DB, INSP>;
}

impl RevmNetworkSpec for Ethereum {
    type CFG = CfgEnv;
    type CHAIN = ();
    type EVM<DB: DatabaseRef, INSP> = MainnetRevmEvm<DB, Self::TX, Self::CFG, Self::CHAIN, INSP>;
    type EvmError<DB: DatabaseRef> = EVMError<DB::Error, InvalidTransaction>;
    type EvmHaltReason = HaltReason;
    type TX = TxEnv;

    fn convert_build_tx(
        tx: TxEnv,
        #[cfg(feature = "op-revm")] _: impl FnMut(&mut op_revm::OpTransaction<TxEnv>)
    ) -> Self::TX {
        tx
    }

    fn build_context<DB: DatabaseRef>(
        db: CacheDB<DB>,
        chain_id: u64
    ) -> NetworkRevmContext<DB, Self::TX, Self::CFG, Self::CHAIN> {
        Context::mainnet()
            .modify_cfg_chained(|cfg| cfg.chain_id = chain_id)
            .with_db(db)
    }

    fn build_evm<DB: DatabaseRef>(db: CacheDB<DB>, chain_id: u64) -> Self::EVM<DB, ()> {
        Self::build_context(db, chain_id).build_mainnet()
    }

    fn build_evm_with_inspector<DB: DatabaseRef, INSP>(
        db: CacheDB<DB>,
        chain_id: u64,
        inspector: INSP
    ) -> Self::EVM<DB, INSP> {
        Self::build_context(db, chain_id).build_mainnet_with_inspector(inspector)
    }
}

#[cfg(feature = "op-revm")]
mod op_impl {
    use op_alloy_network::Optimism;
    use op_revm::{
        DefaultOp, L1BlockInfo, OpBuilder, OpEvm, OpHaltReason, OpSpecId, OpTransaction, OpTransactionError,
        precompiles::OpPrecompiles
    };

    use super::*;

    type OptimismRevmEvm<DB, TX, CFG, CHAIN, INSP> = OpEvm<
        NetworkRevmContext<DB, TX, CFG, CHAIN>,
        INSP,
        EthInstructions<EthInterpreter, NetworkRevmContext<DB, TX, CFG, CHAIN>>,
        OpPrecompiles
    >;

    impl RevmNetworkSpec for Optimism {
        type CFG = CfgEnv<OpSpecId>;
        type CHAIN = L1BlockInfo;
        type EVM<DB: DatabaseRef, INSP> = OptimismRevmEvm<DB, Self::TX, Self::CFG, Self::CHAIN, INSP>;
        type EvmError<DB: DatabaseRef> = EVMError<DB::Error, OpTransactionError>;
        type EvmHaltReason = OpHaltReason;
        type TX = OpTransaction<TxEnv>;

        fn convert_build_tx(
            tx: TxEnv,
            #[cfg(feature = "op-revm")] mut op_mods: impl FnMut(&mut op_revm::OpTransaction<TxEnv>)
        ) -> Self::TX {
            let mut tx = OpTransaction::new(tx);
            op_mods(&mut tx);
            tx
        }

        fn build_context<DB: DatabaseRef>(
            db: CacheDB<DB>,
            chain_id: u64
        ) -> NetworkRevmContext<DB, Self::TX, Self::CFG, Self::CHAIN> {
            Context::op()
                .modify_cfg_chained(|cfg| cfg.chain_id = chain_id)
                .with_db(db)
        }

        fn build_evm<DB: DatabaseRef>(db: CacheDB<DB>, chain_id: u64) -> Self::EVM<DB, ()> {
            Self::build_context(db, chain_id).build_op()
        }

        fn build_evm_with_inspector<DB: DatabaseRef, INSP>(
            db: CacheDB<DB>,
            chain_id: u64,
            inspector: INSP
        ) -> Self::EVM<DB, INSP> {
            Self::build_context(db, chain_id).build_op_with_inspector(inspector)
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
    fn make_empty_evm(&self, params: &Self::Params) -> eyre::Result<N::EVM<Self::InnerDb, ()>> {
        let db = self.make_cache_db(params)?;
        Ok(N::build_evm(db, params.chain_id()))
    }

    /// `makes a new cache db`
    fn make_empty_evm_with_inspector<INSP>(
        &self,
        params: &Self::Params,
        inspector: INSP
    ) -> eyre::Result<N::EVM<Self::InnerDb, INSP>> {
        let db = self.make_cache_db(params)?;
        Ok(N::build_evm_with_inspector(db, params.chain_id(), inspector))
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
