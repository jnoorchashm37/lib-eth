use std::{
    path::{Path, PathBuf},
    sync::Arc
};

use eth_network_exts::EthNetworkExt;
use exe_runners::{TaskSpawner, TokioTaskExecutor};
use reth_db::mdbx::{DatabaseArguments, MaxReadTransactionDuration};
use reth_node_types::NodeTypes;

use crate::reth_libmdbx::node_types::{NodeClientSpec, RethNodeClient};

#[derive(Debug, Clone)]
pub struct RethNodeClientBuilder<Ext: EthNetworkExt> {
    db_path:             String,
    max_tasks:           usize,
    db_args:             Option<DatabaseArguments>,
    chain:               Arc<<Ext::RethNode as NodeTypes>::ChainSpec>,
    ipc_path_or_rpc_url: Option<String>
}

impl<Ext: EthNetworkExt> RethNodeClientBuilder<Ext>
where
    Ext::RethNode: NodeClientSpec
{
    pub fn new(
        db_path: &str,
        max_tasks: usize,
        chain: Arc<<Ext::RethNode as NodeTypes>::ChainSpec>,
        ipc_path_or_rpc_url: Option<&str>
    ) -> Self {
        Self {
            db_path: db_path.to_string(),
            max_tasks,
            db_args: None,
            chain,
            ipc_path_or_rpc_url: ipc_path_or_rpc_url.map(|a| a.to_string())
        }
    }

    pub fn with_db_args(mut self, db_args: DatabaseArguments) -> Self {
        self.db_args = Some(db_args);
        self
    }

    pub fn build(self) -> eyre::Result<RethNodeClient<Ext>> {
        self.build_with_task_executor(TokioTaskExecutor::default())
    }

    pub fn build_with_task_executor<T: TaskSpawner + Clone + 'static>(
        self,
        task_executor: T
    ) -> eyre::Result<RethNodeClient<Ext>> {
        let (db_path, static_files_path, rocksdb_path) = self.db_paths()?;

        let db_args = self.db_args.unwrap_or_else(|| {
            DatabaseArguments::new(Default::default()).with_max_read_transaction_duration(Some(
                MaxReadTransactionDuration::Set(std::time::Duration::from_secs(120))
            ))
        });

        let db_config = DbConfig { db_path, static_files_path, rocksdb_path, db_args };

        <Ext::RethNode as NodeClientSpec>::new_with_db::<_, Ext>(
            db_config,
            self.max_tasks,
            task_executor,
            self.chain,
            self.ipc_path_or_rpc_url
        )
    }

    /// (db_path, static_files, rocksdb)
    fn db_paths(&self) -> eyre::Result<(PathBuf, PathBuf, PathBuf)> {
        let db_dir = Path::new(&self.db_path);

        if !db_dir.exists() {
            eyre::bail!("db path does not exist: {}", self.db_path);
        }

        let db_path = db_dir.join("db");

        if !db_path.exists() {
            eyre::bail!("no 'db' subdirectory found in directory '{db_dir:?}'")
        }

        let static_files_path = db_dir.join("static_files");
        if !static_files_path.exists() {
            eyre::bail!("no 'static_files' subdirectory found in directory '{db_dir:?}'")
        }

        let rocksdb_path = db_dir.join("rocksdb");

        Ok((db_path, static_files_path, rocksdb_path))
    }
}

pub struct DbConfig {
    pub db_path:           PathBuf,
    pub static_files_path: PathBuf,
    pub rocksdb_path:      PathBuf,
    pub db_args:           DatabaseArguments
}
