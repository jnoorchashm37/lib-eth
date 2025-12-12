use op_alloy_network::Optimism;
use reth_optimism_node::OpNode;

use crate::{AllExtensions, EthNetworkExt};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnichainMainnetExt<Extension = ()>(Extension)
where
    Extension: AllExtensions;

impl<Extension: AllExtensions> EthNetworkExt for UnichainMainnetExt<Extension> {
    type AlloyNetwork = Optimism;
    type RethNode = OpNode;
    type TypeExt = Extension;

    const CHAIN_ID: u64 = 130;
}
