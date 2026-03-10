use anyhow::Result;
use secrecy::SecretString;
use tronic::{
    client::{Auth, Client},
    provider::grpc::GrpcProvider,
    signer::LocalSigner,
};

use crate::config::Network;

pub type TronClient = Client<GrpcProvider, LocalSigner>;

/// Build a client with a signer.
pub async fn build(
    network: Network,
    signer: LocalSigner,
    api_key: Option<&str>,
) -> Result<TronClient> {
    let endpoint = network.grpc_endpoint();
    let provider = match api_key {
        Some(key) => {
            GrpcProvider::builder()
                .auth(Auth::Bearer {
                    name: "TRON-PRO-API-KEY".to_string(),
                    secret: SecretString::from(key.to_string()),
                })
                .connect(endpoint)
                .await?
        }
        None => {
            GrpcProvider::builder()
                .connect(endpoint)
                .await?
        }
    };
    let client = Client::builder()
        .provider(provider)
        .signer(signer)
        .build();
    Ok(client)
}
