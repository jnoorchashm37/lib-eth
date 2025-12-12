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

pub const V4_POOL_MANAGER_ADDRESS: Address = address!("0x000000000004444c5dc75cb358380d2e3de08a90");
pub const V4_POSITION_MANAGER_ADDRESS: Address = address!("0xbd216513d74c8cf14cf4747e6aaa6420ff64ee9e");

pub const ANGSTROM_ADDRESS: Address = address!("0x0000000AA8c2Fb9b232F78D2B286dC2aE53BfAD4");
