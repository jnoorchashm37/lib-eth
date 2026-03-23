mod impls;
pub use impls::*;

pub trait EthNetworkExt: Send + Sync {
    type AlloyNetwork: alloy_network::Network + Unpin;
    type RethNode: reth_node_types::NodeTypes;
    /// an arbitrary type extension
    type TypeExt;

    const CHAIN_ID: u64;

    fn is_op_chain() -> bool {
        match Self::CHAIN_ID {
            1 | 11155111 => false,
            8453 | 130 => true,
            _ => unreachable!()
        }
    }
}

pub trait AllExtensions: std::fmt::Debug + Send + Sync + Clone + Copy + Unpin + 'static {}

impl<T> AllExtensions for T where T: std::fmt::Debug + Send + Sync + Clone + Copy + Unpin + 'static {}
