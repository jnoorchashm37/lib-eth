use alloy_primitives::{U256, aliases::U24};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AngstromL2FactorySlot0 {
    pub withdraw_only: bool,
    pub default_protocol_swap_fee_as_multiple_e6: U24,
    pub default_protocol_tax_fee_e6: U24,
    pub default_jit_tax_enabled: bool
}

impl From<U256> for AngstromL2FactorySlot0 {
    fn from(value: U256) -> Self {
        Self {
            withdraw_only: (value & U256::from(0xFF)) != U256::ZERO,
            default_protocol_swap_fee_as_multiple_e6: U24::from(U256::to::<u32>(&((value >> 8) & U256::from(0xFFFFFF)))),
            default_protocol_tax_fee_e6: U24::from(U256::to::<u32>(&((value >> 32) & U256::from(0xFFFFFF)))),
            default_jit_tax_enabled: ((value >> 56) & U256::from(0xFF)) != U256::ZERO
        }
    }
}

impl From<&U256> for AngstromL2FactorySlot0 {
    fn from(value: &U256) -> Self {
        AngstromL2FactorySlot0::from(*value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AngstromL2PoolFeeConfiguration {
    pub is_initialized:       bool,
    pub creator_tax_fee_e6:   u32,
    pub protocol_tax_fee_e6:  u32,
    pub creator_swap_fee_e6:  u32,
    pub protocol_swap_fee_e6: u32
}

impl From<U256> for AngstromL2PoolFeeConfiguration {
    fn from(value: U256) -> Self {
        let is_initialized = (value & U256::from(1)) != U256::ZERO;
        let creator_tax_fee_e6 = U256::to::<u32>(&((value >> 8) & U256::from(0xFFFFFF)));
        let protocol_tax_fee_e6 = U256::to::<u32>(&((value >> 32) & U256::from(0xFFFFFF)));
        let creator_swap_fee_e6 = U256::to::<u32>(&((value >> 56) & U256::from(0xFFFFFF)));
        let protocol_swap_fee_e6 = U256::to::<u32>(&((value >> 80) & U256::from(0xFFFFFF)));
        Self { is_initialized, creator_tax_fee_e6, protocol_tax_fee_e6, creator_swap_fee_e6, protocol_swap_fee_e6 }
    }
}

impl From<&U256> for AngstromL2PoolFeeConfiguration {
    fn from(value: &U256) -> Self {
        AngstromL2PoolFeeConfiguration::from(*value)
    }
}
