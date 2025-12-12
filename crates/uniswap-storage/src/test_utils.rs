use alloy_primitives::{Address, address};
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};

pub async fn eth_provider() -> RootProvider {
    dotenv::dotenv().ok();

    let url = std::env::var("ETH_WS_URL").expect("no ETH_WS_URL in .env");
    ProviderBuilder::new()
        .connect_ws(WsConnect::new(url))
        .await
        .unwrap()
        .root()
        .clone()
}

pub const USDC: Address = address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
pub const WETH: Address = address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
