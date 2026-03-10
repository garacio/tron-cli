use clap::ValueEnum;

/// USDT TRC20 contract addresses per network.
pub const USDT_MAINNET: &str = "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t";
// Shasta testnet may not have an official USDT contract; use Nile for TRC20 testing.
pub const USDT_SHASTA: &str = "TG3XXyExBkFU9nQGx1Tx1z1GXWEwe3yUYe";
pub const USDT_NILE: &str = "TXYZopYRdj2D9XRtbG411XZZ3kM5VkAeBf";

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum Network {
    #[default]
    Mainnet,
    Shasta,
    Nile,
}

impl Network {
    pub fn grpc_endpoint(&self) -> &'static str {
        match self {
            Network::Mainnet => "http://grpc.trongrid.io:50051",
            Network::Shasta => "http://grpc.shasta.trongrid.io:50051",
            Network::Nile => "http://grpc.nile.trongrid.io:50051",
        }
    }

    pub fn api_endpoint(&self) -> &'static str {
        match self {
            Network::Mainnet => "https://api.trongrid.io",
            Network::Shasta => "https://api.shasta.trongrid.io",
            Network::Nile => "https://nile.trongrid.io",
        }
    }

    pub fn usdt_contract(&self) -> &'static str {
        match self {
            Network::Mainnet => USDT_MAINNET,
            Network::Shasta => USDT_SHASTA,
            Network::Nile => USDT_NILE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grpc_endpoints_use_http() {
        for net in [Network::Mainnet, Network::Shasta, Network::Nile] {
            let ep = net.grpc_endpoint();
            assert!(ep.starts_with("http://"), "gRPC endpoint must use http: {ep}");
            assert!(ep.contains(":50051"), "gRPC endpoint must use port 50051: {ep}");
        }
    }

    #[test]
    fn api_endpoints_use_https() {
        for net in [Network::Mainnet, Network::Shasta, Network::Nile] {
            let ep = net.api_endpoint();
            assert!(ep.starts_with("https://"), "API endpoint must use https: {ep}");
        }
    }

    #[test]
    fn usdt_contracts_are_tron_addresses() {
        for net in [Network::Mainnet, Network::Shasta, Network::Nile] {
            let addr = net.usdt_contract();
            assert!(addr.starts_with('T'), "USDT contract must start with T: {addr}");
            assert_eq!(addr.len(), 34, "USDT contract must be 34 chars: {addr}");
        }
    }

    #[test]
    fn mainnet_usdt_is_known() {
        assert_eq!(Network::Mainnet.usdt_contract(), "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t");
    }
}
