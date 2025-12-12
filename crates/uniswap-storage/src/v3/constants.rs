use alloy_primitives::{Address, ChainId, address};

#[derive(Debug, Clone, Copy)]
pub struct UniswapV3Constants {
    factory: Address,
    multicall: Option<Address>,
    multicall2: Option<Address>,
    proxy_admin: Option<Address>,
    tick_lens: Option<Address>,
    quoter: Option<Address>,
    swap_router: Option<Address>,
    nft_descriptor: Option<Address>,
    position_descriptor: Option<Address>,
    transparent_upgradeable_proxy: Option<Address>,
    position_manager: Option<Address>,
    v3_migrator: Option<Address>,
    quoter_v2: Option<Address>,
    swap_router_02: Option<Address>,
    permit2: Option<Address>,
    universal_router: Option<Address>,
    v3_staker: Option<Address>
}

impl UniswapV3Constants {
    pub fn by_chain(chain: ChainId) -> Option<Self> {
        match chain {
            1 => Some(UNISWAP_V3_CONSTANTS_MAINNET),
            130 => Some(UNISWAP_V3_CONSTANTS_UNICHAIN_MAINNET),
            8453 => Some(UNISWAP_V3_CONSTANTS_BASE_MAINNET),
            11155111 => Some(UNISWAP_V3_CONSTANTS_SEPOLIA_TESTNET),
            _ => None
        }
    }

    #[inline]
    pub fn factory(&self) -> Address {
        self.factory
    }

    #[inline]
    pub fn multicall(&self) -> Option<Address> {
        self.multicall
    }

    #[inline]
    pub fn multicall2(&self) -> Option<Address> {
        self.multicall2
    }

    #[inline]
    pub fn proxy_admin(&self) -> Option<Address> {
        self.proxy_admin
    }

    #[inline]
    pub fn tick_lens(&self) -> Option<Address> {
        self.tick_lens
    }

    #[inline]
    pub fn quoter(&self) -> Option<Address> {
        self.quoter
    }

    #[inline]
    pub fn swap_router(&self) -> Option<Address> {
        self.swap_router
    }

    #[inline]
    pub fn nft_descriptor(&self) -> Option<Address> {
        self.nft_descriptor
    }

    #[inline]
    pub fn position_descriptor(&self) -> Option<Address> {
        self.position_descriptor
    }

    #[inline]
    pub fn transparent_upgradeable_proxy(&self) -> Option<Address> {
        self.transparent_upgradeable_proxy
    }

    #[inline]
    pub fn position_manager(&self) -> Option<Address> {
        self.position_manager
    }

    #[inline]
    pub fn v3_migrator(&self) -> Option<Address> {
        self.v3_migrator
    }

    #[inline]
    pub fn quoter_v2(&self) -> Option<Address> {
        self.quoter_v2
    }

    #[inline]
    pub fn swap_router_02(&self) -> Option<Address> {
        self.swap_router_02
    }

    #[inline]
    pub fn permit2(&self) -> Option<Address> {
        self.permit2
    }

    #[inline]
    pub fn universal_router(&self) -> Option<Address> {
        self.universal_router
    }

    #[inline]
    pub fn v3_staker(&self) -> Option<Address> {
        self.v3_staker
    }
}

pub const UNISWAP_V3_CONSTANTS_MAINNET: UniswapV3Constants = UniswapV3Constants {
    factory: address!("0x1F98431c8aD98523631AE4a59f267346ea31F984"),
    multicall: Some(address!("0x1F98415757620B543A52E61c46B32eB19261F984")),
    multicall2: Some(address!("0x5BA1e12693Dc8F9c48aAD8770482f4739bEeD696")),
    proxy_admin: Some(address!("0xB753548F6E010e7e680BA186F9Ca1BdAB2E90cf2")),
    tick_lens: Some(address!("0xbfd8137f7d1516D3ea5cA83523914859ec47F573")),
    quoter: Some(address!("0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6")),
    swap_router: Some(address!("0xE592427A0AEce92De3Edee1F18E0157C05861564")),
    nft_descriptor: Some(address!("0x42B24A95702b9986e82d421cC3568932790A48Ec")),
    position_descriptor: Some(address!("0x91ae842A5Ffd8d12023116943e72A606179294f3")),
    transparent_upgradeable_proxy: Some(address!("0xEe6A57eC80ea46401049E92587E52f5Ec1c24785")),
    position_manager: Some(address!("0xC36442b4a4522E871399CD717aBDD847Ab11FE88")),
    v3_migrator: Some(address!("0xA5644E29708357803b5A882D272c41cC0dF92B34")),
    quoter_v2: Some(address!("0x61fFE014bA17989E743c5F6cB21bF9697530B21e")),
    swap_router_02: Some(address!("0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45")),
    permit2: Some(address!("0x000000000022D473030F116dDEE9F6B43aC78BA3")),
    universal_router: Some(address!("0x66a9893cc07d91d95644aedd05d03f95e1dba8af")),
    v3_staker: Some(address!("0xe34139463bA50bD61336E0c446Bd8C0867c6fE65"))
};

pub const UNISWAP_V3_CONSTANTS_SEPOLIA_TESTNET: UniswapV3Constants = UniswapV3Constants {
    factory: address!("0x0227628f3F023bb0B980b67D528571c95c6DaC1c"),
    multicall: None,
    multicall2: Some(address!("0xD7F33bCdb21b359c8ee6F0251d30E94832baAd07")),
    proxy_admin: Some(address!("0x0b343475d44EC2b4b8243EBF81dc888BF0A14b36")),
    tick_lens: None,
    quoter: None,
    swap_router: None,
    nft_descriptor: Some(address!("0x3B5E3c5E595D85fbFBC2a42ECC091e183E76697C")),
    position_descriptor: Some(address!("0x5bE4DAa6982C69aD20A57F1e68cBcA3D37de6207")),
    transparent_upgradeable_proxy: None,
    position_manager: Some(address!("0x1238536071E1c677A632429e3655c799b22cDA52")),
    v3_migrator: Some(address!("0x729004182cF005CEC8Bd85df140094b6aCbe8b15")),
    quoter_v2: Some(address!("0xEd1f6473345F45b75F8179591dd5bA1888cf2FB3")),
    swap_router_02: Some(address!("0x3bFA4769FB09eefC5a80d6E87c3B9C650f7Ae48E")),
    permit2: Some(address!("0x000000000022D473030F116dDEE9F6B43aC78BA3")),
    universal_router: Some(address!("0x3A9D48AB9751398BbFa63ad67599Bb04e4BdF98b")),
    v3_staker: None
};

pub const UNISWAP_V3_CONSTANTS_BASE_MAINNET: UniswapV3Constants = UniswapV3Constants {
    factory: address!("0x33128a8fC17869897dcE68Ed026d694621f6FDfD"),
    multicall: Some(address!("0x091e99cb1C49331a94dD62755D168E941AbD0693")),
    multicall2: None,
    proxy_admin: Some(address!("0x3334d83e224aF5ef9C2E7DDA7c7C98Efd9621fA9")),
    tick_lens: Some(address!("0x0CdeE061c75D43c82520eD998C23ac2991c9ac6d")),
    quoter: None,
    swap_router: None,
    nft_descriptor: Some(address!("0xF9d1077fd35670d4ACbD27af82652a8d84577d9F")),
    position_descriptor: Some(address!("0x4f225937EDc33EFD6109c4ceF7b560B2D6401009")),
    transparent_upgradeable_proxy: Some(address!("0x4615C383F85D0a2BbED973d83ccecf5CB7121463")),
    position_manager: Some(address!("0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1")),
    v3_migrator: Some(address!("0x23cF10b1ee3AdfCA73B0eF17C07F7577e7ACd2d7")),
    quoter_v2: Some(address!("0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a")),
    swap_router_02: Some(address!("0x2626664c2603336E57B271c5C0b26F421741e481")),
    permit2: Some(address!("0x000000000022D473030F116dDEE9F6B43aC78BA3")),
    universal_router: Some(address!("0x6fF5693b99212Da76ad316178A184AB56D299b43")),
    v3_staker: Some(address!("0x42bE4D6527829FeFA1493e1fb9F3676d2425C3C1"))
};

pub const UNISWAP_V3_CONSTANTS_UNICHAIN_MAINNET: UniswapV3Constants = UniswapV3Constants {
    factory: address!("0x1f98400000000000000000000000000000000003"),
    multicall: Some(address!("0xb7610f9b733e7d45184be3a1bc966960ccc54f0b")),
    multicall2: None,
    proxy_admin: None,
    tick_lens: Some(address!("0xd5d76fa166ab8d8ad4c9f61aaa81457b66cbe443")),
    quoter: None,
    swap_router: None,
    nft_descriptor: Some(address!("0x0dfa04b28ab68ffd0e6e17fac6ec16d4846a2004")),
    position_descriptor: None,
    transparent_upgradeable_proxy: None,
    position_manager: Some(address!("0x943e6e07a7e8e791dafc44083e54041d743c46e9")),
    v3_migrator: Some(address!("0xb9d0c246f306b1aaf02ae6ba112d5ef25e5b60dc")),
    quoter_v2: Some(address!("0x385a5cf5f83e99f7bb2852b6a19c3538b9fa7658")),
    swap_router_02: Some(address!("0x73855d06de49d0fe4a9c42636ba96c62da12ff9c")),
    permit2: None,
    universal_router: None,
    v3_staker: None
};
