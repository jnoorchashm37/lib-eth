use alloy_primitives::{U256, aliases::U24};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AngstromL2FactorySlot0 {
    pub withdraw_only: bool,
    pub default_protocol_swap_fee_as_multiple_e6: U24,
    pub default_protocol_tax_fee_e6: U24
}

impl From<U256> for AngstromL2FactorySlot0 {
    fn from(value: U256) -> Self {
        Self {
            withdraw_only: (value & U256::from(0xFF)) != U256::ZERO,
            default_protocol_swap_fee_as_multiple_e6: U24::from(U256::to::<u32>(&((value >> 8) & U256::from(0xFFFFFF)))),
            default_protocol_tax_fee_e6: U24::from(U256::to::<u32>(&((value >> 32) & U256::from(0xFFFFFF))))
        }
    }
}

impl From<&U256> for AngstromL2FactorySlot0 {
    fn from(value: &U256) -> Self {
        AngstromL2FactorySlot0::from(*value)
    }
}
