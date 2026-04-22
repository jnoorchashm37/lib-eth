use alloy_primitives::{Address, ChainId, address};

use crate::v4::{UNISWAP_V4_CONSTANTS_BASE_MAINNET, UNISWAP_V4_CONSTANTS_UNICHAIN_MAINNET, UniswapV4Constants};

#[derive(Debug, Clone)]
pub struct AngstromL2Constants {
    angstrom_l2_factory:   Address,
    hook_address_miner:    Address,
    angstrom_deploy_block: u64,
    chain_id:              u64,
    uniswap_constants:     UniswapV4Constants
}

impl AngstromL2Constants {
    pub fn by_chain(chain_id: ChainId) -> Option<Self> {
        match chain_id {
            130 => Some(ANGSTROM_L2_CONSTANTS_UNICHAIN_MAINNET),
            8453 => Some(ANGSTROM_L2_CONSTANTS_BASE_MAINNET),
            _ => None
        }
    }

    #[inline]
    pub fn angstrom_l2_factory(&self) -> Address {
        self.angstrom_l2_factory
    }

    #[inline]
    pub fn hook_address_miner(&self) -> Address {
        self.hook_address_miner
    }

    #[inline]
    pub fn angstrom_deploy_block(&self) -> u64 {
        self.angstrom_deploy_block
    }

    #[inline]
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    #[inline]
    pub fn uniswap_constants(&self) -> UniswapV4Constants {
        self.uniswap_constants
    }
}

pub const ANGSTROM_L2_CONSTANTS_BASE_MAINNET: AngstromL2Constants = AngstromL2Constants {
    angstrom_l2_factory:   address!("0x0000000000fd3b85c30f942e8d878e858e69cd05"),
    hook_address_miner:    address!("0x1C9e501116879d6A6748582047eBcb8bbcCC7d53"),
    angstrom_deploy_block: 43873127,
    chain_id:              8453,
    uniswap_constants:     UNISWAP_V4_CONSTANTS_BASE_MAINNET
};

pub const ANGSTROM_L2_CONSTANTS_UNICHAIN_MAINNET: AngstromL2Constants = AngstromL2Constants {
    angstrom_l2_factory:   address!("0x000000000000a0220e791bAe32b16C0406980C42"),
    hook_address_miner:    address!("0x98bD6680eE62C006730A5a4dFB2f0Da6645A10f5"),
    angstrom_deploy_block: 29275745,
    chain_id:              130,
    uniswap_constants:     UNISWAP_V4_CONSTANTS_UNICHAIN_MAINNET
};
