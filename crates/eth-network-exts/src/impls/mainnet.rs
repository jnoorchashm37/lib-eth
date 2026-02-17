use std::marker::PhantomData;

use alloy_network::Ethereum;
use reth_node_ethereum::EthereumNode;

use crate::{AllExtensions, EthNetworkExt};

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MainnetExt<Extension = ()>(PhantomData<Extension>);

impl<Extension: AllExtensions> EthNetworkExt for MainnetExt<Extension> {
    type AlloyNetwork = Ethereum;
    type RethNode = EthereumNode;
    type TypeExt = Extension;

    const CHAIN_ID: u64 = 1;
}
