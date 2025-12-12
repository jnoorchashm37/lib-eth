use alloy_primitives::{
    B256, U160, U256,
    aliases::{I24, U24},
    keccak256
};
use alloy_sol_types::{SolValue, sol};
use serde::{Deserialize, Serialize};

sol! {
    #[derive(Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
    struct V4PoolKey {
        address currency0;
        address currency1;
        uint24 fee;
        int24 tickSpacing;
        address hooks;
    }
}

impl From<V4PoolKey> for B256 {
    fn from(value: V4PoolKey) -> Self {
        keccak256(value.abi_encode())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct V4UserLiquidityPosition {
    pub token_id:   U256,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub liquidity:  u128,
    pub pool_key:   V4PoolKey
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UnpackedSlot0 {
    pub sqrt_price_x96: U160,
    pub tick:           I24,
    pub protocol_fee:   U24,
    pub lp_fee:         U24
}

pub trait UnpackSlot0 {
    fn unpack_slot0(&self) -> UnpackedSlot0;

    fn sqrt_price_x96(&self) -> U160;

    fn tick(&self) -> I24;

    fn protocol_fee(&self) -> U24;

    fn lp_fee(&self) -> U24;
}

const MASK_24_BITS: U256 = U256::from_limbs([0xFFFFFF, 0, 0, 0]);

const TICK_OFFSET: u32 = 160;
const PROTOCOL_FEE_OFFSET: u32 = 184;
const LP_FEE_OFFSET: u32 = 208;

impl UnpackSlot0 for U256 {
    fn unpack_slot0(&self) -> UnpackedSlot0 {
        UnpackedSlot0 {
            sqrt_price_x96: self.sqrt_price_x96(),
            tick:           self.tick(),
            protocol_fee:   self.protocol_fee(),
            lp_fee:         self.lp_fee()
        }
    }

    fn sqrt_price_x96(&self) -> U160 {
        // Extract the lowest 160 bits from U256
        // U160 has 3 limbs in little-endian order
        // We need to mask the third limb to only keep 32 bits (160 - 128 = 32)
        let limbs = self.as_limbs();
        U160::from_limbs([
            limbs[0],
            limbs[1],
            limbs[2] & 0xFFFFFFFF // Only keep the lower 32 bits
        ])
    }

    fn tick(&self) -> I24 {
        let raw = ((*self >> TICK_OFFSET) & MASK_24_BITS).to::<u32>();

        I24::unchecked_from(((raw << 8) as i32) >> 8)
    }

    fn protocol_fee(&self) -> U24 {
        U24::from(((*self >> PROTOCOL_FEE_OFFSET) & MASK_24_BITS).to::<u32>())
    }

    fn lp_fee(&self) -> U24 {
        U24::from(((*self >> LP_FEE_OFFSET) & MASK_24_BITS).to::<u32>())
    }
}

const TICK_LOWER_OFFSET: u32 = 8;
const TICK_UPPER_OFFSET: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UnpackedPositionInfo {
    /// key for lookups in the PositionManager's `poolKeys` map
    pub position_manager_pool_map_key: [u8; 25],
    pub tick_lower:                    I24,
    pub tick_upper:                    I24
}

pub trait UnpackPositionInfo {
    fn unpack_position_info(&self) -> UnpackedPositionInfo;

    fn position_manager_pool_map_key(&self) -> [u8; 25];

    fn tick_lower(&self) -> I24;

    fn tick_upper(&self) -> I24;
}

impl UnpackPositionInfo for U256 {
    fn unpack_position_info(&self) -> UnpackedPositionInfo {
        UnpackedPositionInfo {
            position_manager_pool_map_key: self.position_manager_pool_map_key(),
            tick_lower:                    self.tick_lower(),
            tick_upper:                    self.tick_upper()
        }
    }

    fn position_manager_pool_map_key(&self) -> [u8; 25] {
        let shifted: U256 = *self >> 56;
        let mut out = [0u8; 25];
        out.copy_from_slice(&shifted.to_be_bytes_vec()[7..]);
        out
    }

    fn tick_lower(&self) -> I24 {
        let raw = ((*self >> TICK_LOWER_OFFSET) & U256::from((1u128 << 24) - 1)).to::<u32>();
        I24::unchecked_from(((raw << 8) as i32) >> 8)
    }

    fn tick_upper(&self) -> I24 {
        let raw = ((*self >> TICK_UPPER_OFFSET) & U256::from((1u128 << 24) - 1)).to::<u32>();
        I24::unchecked_from(((raw << 8) as i32) >> 8)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{
        U160, U256,
        aliases::{I24, U24}
    };

    use super::*;

    #[test]
    fn test_unpack_position_info() {
        let position_info_packed =
            U256::from_str_radix("36752956352201235409813682138304141020772237719769761638105745524212318476800", 10)
                .unwrap();

        let unpacked = position_info_packed.unpack_position_info();

        assert_eq!(unpacked.tick_lower, I24::unchecked_from(-887270));
        assert_eq!(unpacked.tick_upper, I24::unchecked_from(887270));
        assert_eq!(unpacked.position_manager_pool_map_key, [0; 25]);
    }

    #[test]
    fn test_slot0_simple() {
        // Test with simple values first
        let mut slot0 = U256::ZERO;

        // Set tick = 100 at offset 160
        slot0 |= U256::from(100u32) << 160;

        println!("slot0 after setting tick: {slot0:?}");
        println!("Extracted tick raw: {:?}", (slot0 >> 160) & MASK_24_BITS);
        println!("Extracted tick value: {:?}", slot0.tick());

        assert_eq!(slot0.tick(), I24::unchecked_from(100));
    }

    #[test]
    fn test_unpack_slot0() {
        // Test case with known values
        // Layout: 24 bits empty | 24 bits lpFee | 12 bits protocolFee 1->0 | 12 bits
        // protocolFee 0->1 | 24 bits tick | 160 bits sqrtPriceX96

        // Example values:
        // sqrtPriceX96: 0x5f4e3d2c1b0a9876543210fedcba98 (160 bits)
        // tick: 100 (0x000064)
        // protocolFee: 0x001234 (upper 12 bits for 1->0: 0x001, lower 12 bits for 0->1:
        // 0x234) lpFee: 3000 (0x000BB8)

        let sqrt_price = U160::from_str_radix("5f4e3d2c1b0a9876543210fedcba98", 16).unwrap();
        let tick = I24::unchecked_from(100);
        let protocol_fee = U24::from(0x001234);
        let lp_fee = U24::from(3000);

        // Construct the packed slot0
        let mut slot0 = U256::ZERO;

        // Set sqrtPriceX96 (lowest 160 bits)
        slot0 |= U256::from_limbs([sqrt_price.as_limbs()[0], sqrt_price.as_limbs()[1], sqrt_price.as_limbs()[2], 0]);

        // Set tick (24 bits at offset 160)
        slot0 |= U256::from(tick.bits()) << 160;

        // Set protocolFee (24 bits at offset 184)
        slot0 |= U256::from(protocol_fee.to::<u32>()) << 184;

        // Set lpFee (24 bits at offset 208)
        slot0 |= U256::from(lp_fee.to::<u32>()) << 208;

        // Test unpacking
        let unpacked = slot0.unpack_slot0();

        assert_eq!(unpacked.sqrt_price_x96, sqrt_price);
        assert_eq!(unpacked.tick, tick);
        assert_eq!(unpacked.protocol_fee, protocol_fee);
        assert_eq!(unpacked.lp_fee, lp_fee);

        // Test individual getters
        assert_eq!(slot0.sqrt_price_x96(), sqrt_price);
        assert_eq!(slot0.tick(), tick);
        assert_eq!(slot0.protocol_fee(), protocol_fee);
        assert_eq!(slot0.lp_fee(), lp_fee);
    }

    #[test]
    fn test_unpack_slot0_negative_tick() {
        // Test with negative tick
        let sqrt_price = U160::from(1234567890u64);
        let tick = I24::unchecked_from(-1000);
        let protocol_fee = U24::from(500);
        let lp_fee = U24::from(100);

        // Construct the packed slot0
        let mut slot0 = U256::ZERO;
        slot0 |= U256::from_limbs([sqrt_price.as_limbs()[0], sqrt_price.as_limbs()[1], sqrt_price.as_limbs()[2], 0]);

        // For negative tick, we need to handle two's complement for 24 bits
        let tick_bits = tick.bits() & 0xFFFFFF;
        slot0 |= U256::from(tick_bits) << 160;
        slot0 |= U256::from(protocol_fee.to::<u32>()) << 184;
        slot0 |= U256::from(lp_fee.to::<u32>()) << 208;

        // Test unpacking
        assert_eq!(slot0.tick(), tick);
    }
}
