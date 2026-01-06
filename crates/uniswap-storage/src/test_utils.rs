use alloy_network::Network;
use alloy_primitives::{Address, address};
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};
use op_alloy_network::Optimism;

pub async fn eth_provider() -> RootProvider {
    __eth_provider("ETH_WS_URL").await
}

async fn __eth_provider<N: Network>(env: &str) -> RootProvider<N> {
    dotenv::dotenv().ok();
    let url = std::env::var(env).expect(&format!("no {env} in .env"));
    ProviderBuilder::<_, _, N>::default()
        .connect_ws(WsConnect::new(url))
        .await
        .unwrap()
        .root()
        .clone()
}

pub async fn eth_base_provider() -> RootProvider<Optimism> {
    __eth_provider("BASE_WS_URL").await
}

pub const USDC: Address = address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
pub const WETH: Address = address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
