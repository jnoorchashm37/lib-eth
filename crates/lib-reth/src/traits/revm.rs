use alloy_eips::BlockId;
use revm::{
    Context, DatabaseRef, MainBuilder, MainContext,
    context::{BlockEnv, CfgEnv, Evm, TxEnv},
    handler::{EthFrame, EthPrecompiles, instructions::EthInstructions},
    interpreter::interpreter::EthInterpreter
};
use revm_database::{CacheDB, WrapDatabaseAsync, async_db::DatabaseAsyncRef};

pub type RevmEvm<DB> = Evm<
    Context<BlockEnv, TxEnv, CfgEnv, DB>,
    (),
    EthInstructions<EthInterpreter, Context<BlockEnv, TxEnv, CfgEnv, DB>>,
    EthPrecompiles,
    EthFrame
>;

/// revm utils
pub trait EthRevm {
    type InnerDb: DatabaseRef;

    /// `makes the inner database fetcher`
    fn make_inner_db(&self, block_id: BlockId) -> eyre::Result<Self::InnerDb>;

    /// `makes a new cache db`
    fn make_cache_db(&self, block_id: BlockId) -> eyre::Result<CacheDB<Self::InnerDb>> {
        Ok(CacheDB::new(self.make_inner_db(block_id)?))
    }

    /// `makes a new cache db`
    fn make_empty_evm(&self, block_id: BlockId) -> eyre::Result<RevmEvm<CacheDB<Self::InnerDb>>> {
        let cache = self.make_cache_db(block_id)?;
        let evm = Context::mainnet().with_db(cache).build_mainnet();
        Ok(evm)
    }
}

/// async revm utils
pub trait AsyncEthRevm {
    type InnerDb: DatabaseAsyncRef;

    /// `makes the inner database fetcher`
    fn make_inner_db(
        &self,
        block_number: u64,
        handle: tokio::runtime::Handle
    ) -> eyre::Result<WrapDatabaseAsync<Self::InnerDb>>;

    /// `makes a new cache db`
    fn make_cache_db(
        &self,
        block_number: u64,
        handle: tokio::runtime::Handle
    ) -> eyre::Result<CacheDB<WrapDatabaseAsync<Self::InnerDb>>> {
        Ok(CacheDB::new(self.make_inner_db(block_number, handle)?))
    }

    /// `makes a new evm with a cache db`
    fn make_evm(
        &self,
        block_number: u64,
        handle: tokio::runtime::Handle
    ) -> eyre::Result<RevmEvm<CacheDB<WrapDatabaseAsync<Self::InnerDb>>>> {
        let cache = self.make_cache_db(block_number, handle)?;
        let evm = Context::mainnet().with_db(cache).build_mainnet();

        Ok(evm)
    }
}
