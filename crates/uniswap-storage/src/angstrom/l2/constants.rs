use alloy_primitives::{Address, ChainId, address};
use alloy_sol_types::Eip712Domain;

use crate::v4::{UNISWAP_V4_CONSTANTS_BASE_MAINNET, UNISWAP_V4_CONSTANTS_UNICHAIN_MAINNET, UniswapV4Constants};

#[derive(Debug, Clone)]
pub struct AngstromL2Constants {
    angstrom_l2_factory:    Address,
    hook_address_miner:     Address,
    angstrom_deploy_block:  u64,
    chain_id:               u64,
    angstrom_eip712_domain: Eip712Domain,
    uniswap_constants:      UniswapV4Constants
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
    pub fn angstrom_eip712_domain(&self) -> Eip712Domain {
        self.angstrom_eip712_domain.clone()
    }

    #[inline]
    pub fn uniswap_constants(&self) -> UniswapV4Constants {
        self.uniswap_constants
    }
}

pub const ANGSTROM_L2_CONSTANTS_BASE_MAINNET: AngstromL2Constants = AngstromL2Constants {
    angstrom_l2_factory:    address!("0x000000000000a0220e791bAe32b16C0406980C42"),
    hook_address_miner:     address!("0x801988754b99f142C5C5CA19ca9656CFf31A898a"),
    angstrom_deploy_block:  36617377,
    chain_id:               8453,
    angstrom_eip712_domain: alloy_sol_types::eip712_domain!(
        name: "Angstrom",
        version: "v1",
        chain_id: 8453,
        verifying_contract: Address::ZERO,
    ),
    uniswap_constants:      UNISWAP_V4_CONSTANTS_BASE_MAINNET
};

pub const ANGSTROM_L2_CONSTANTS_UNICHAIN_MAINNET: AngstromL2Constants = AngstromL2Constants {
    angstrom_l2_factory:    address!("0x000000000000a0220e791bae32b16c0406980c42"),
    hook_address_miner:     address!("0x98bD6680eE62C006730A5a4dFB2f0Da6645A10f5"),
    angstrom_deploy_block:  29275745,
    chain_id:               130,
    angstrom_eip712_domain: alloy_sol_types::eip712_domain!(
        name: "Angstrom",
        version: "v1",
        chain_id: 130,
        verifying_contract: Address::ZERO,
    ),
    uniswap_constants:      UNISWAP_V4_CONSTANTS_UNICHAIN_MAINNET
};
