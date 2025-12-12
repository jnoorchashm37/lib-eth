use alloy_primitives::{U256, aliases::I24};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TickData {
    pub tick:                     I24,
    pub is_initialized:           bool,
    pub liquidity_net:            i128,
    pub liquidity_gross:          u128,
    pub fee_growth_outside0_x128: U256,
    pub fee_growth_outside1_x128: U256
}
