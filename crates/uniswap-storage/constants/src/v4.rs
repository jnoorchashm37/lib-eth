use alloy_primitives::{Address, ChainId, address};

#[derive(Debug, Clone, Copy)]
pub struct UniswapV4Constants {
    pool_manager:        Address,
    position_descriptor: Option<Address>,
    position_manager:    Address,
    quoter:              Address,
    state_view:          Address,
    universal_router:    Address,
    permit2:             Address
}

impl UniswapV4Constants {
    pub fn by_chain(chain: ChainId) -> Option<Self> {
        match chain {
            1 => Some(UNISWAP_V4_CONSTANTS_MAINNET),
            130 => Some(UNISWAP_V4_CONSTANTS_UNICHAIN_MAINNET),
            8453 => Some(UNISWAP_V4_CONSTANTS_BASE_MAINNET),
            11155111 => Some(UNISWAP_V4_CONSTANTS_SEPOLIA_TESTNET),
            _ => None
        }
    }

    #[inline]
    pub fn pool_manager(&self) -> Address {
        self.pool_manager
    }

    #[inline]
    pub fn position_descriptor(&self) -> Option<Address> {
        self.position_descriptor
    }

    #[inline]
    pub fn position_manager(&self) -> Address {
        self.position_manager
    }

    #[inline]
    pub fn quoter(&self) -> Address {
        self.quoter
    }

    #[inline]
    pub fn state_view(&self) -> Address {
        self.state_view
    }

    #[inline]
    pub fn universal_router(&self) -> Address {
        self.universal_router
    }

    #[inline]
    pub fn permit2(&self) -> Address {
        self.permit2
    }
}

pub const UNISWAP_V4_CONSTANTS_MAINNET: UniswapV4Constants = UniswapV4Constants {
    pool_manager:        address!("0x000000000004444c5dc75cB358380D2e3dE08A90"),
    position_descriptor: Some(address!("0xd1428ba554f4c8450b763a0b2040a4935c63f06c")),
    position_manager:    address!("0xbd216513d74c8cf14cf4747e6aaa6420ff64ee9e"),
    quoter:              address!("0x52f0e24d1c21c8a0cb1e5a5dd6198556bd9e1203"),
    state_view:          address!("0x7ffe42c4a5deea5b0fec41c94c136cf115597227"),
    universal_router:    address!("0x66a9893cc07d91d95644aedd05d03f95e1dba8af"),
    permit2:             address!("0x000000000022D473030F116dDEE9F6B43aC78BA3")
};

pub const UNISWAP_V4_CONSTANTS_BASE_MAINNET: UniswapV4Constants = UniswapV4Constants {
    pool_manager:        address!("0x498581ff718922c3f8e6a244956af099b2652b2b"),
    position_descriptor: Some(address!("0x25d093633990dc94bedeed76c8f3cdaa75f3e7d5")),
    position_manager:    address!("0x7c5f5a4bbd8fd63184577525326123b519429bdc"),
    quoter:              address!("0x0d5e0f971ed27fbff6c2837bf31316121532048d"),
    state_view:          address!("0xa3c0c9b65bad0b08107aa264b0f3db444b867a71"),
    universal_router:    address!("0x6ff5693b99212da76ad316178a184ab56d299b43"),
    permit2:             address!("0x000000000022D473030F116dDEE9F6B43aC78BA3")
};

pub const UNISWAP_V4_CONSTANTS_UNICHAIN_MAINNET: UniswapV4Constants = UniswapV4Constants {
    pool_manager:        address!("0x1f98400000000000000000000000000000000004"),
    position_descriptor: Some(address!("0x9fb28449a191cd8c03a1b7abfb0f5996ecf7f722")),
    position_manager:    address!("0x4529a01c7a0410167c5740c487a8de60232617bf"),
    quoter:              address!("0x333e3c607b141b18ff6de9f258db6e77fe7491e0"),
    state_view:          address!("0x86e8631a016f9068c3f085faf484ee3f5fdee8f2"),
    universal_router:    address!("0xef740bf23acae26f6492b10de645d6b98dc8eaf3"),
    permit2:             address!("0x000000000022D473030F116dDEE9F6B43aC78BA3")
};

pub const UNISWAP_V4_CONSTANTS_SEPOLIA_TESTNET: UniswapV4Constants = UniswapV4Constants {
    pool_manager:        address!("0xE03A1074c86CFeDd5C142C4F04F1a1536e203543"),
    position_descriptor: None,
    position_manager:    address!("0x429ba70129df741B2Ca2a85BC3A2a3328e5c09b4"),
    quoter:              address!("0x61b3f2011a92d183c7dbadbda940a7555ccf9227"),
    state_view:          address!("0xe1dd9c3fa50edb962e442f60dfbc432e24537e4c"),
    universal_router:    address!("0x3A9D48AB9751398BbFa63ad67599Bb04e4BdF98b"),
    permit2:             address!("0x000000000022D473030F116dDEE9F6B43aC78BA3")
};
