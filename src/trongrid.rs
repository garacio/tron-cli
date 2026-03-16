use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::Network;

/// TronGrid REST API client for endpoints not available via gRPC.
pub struct TronGridClient {
    http: reqwest::Client,
    base_url: String,
}

impl TronGridClient {
    pub fn new(network: Network, api_key: Option<&str>) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = api_key {
            headers.insert(
                "TRON-PRO-API-KEY",
                key.parse().context("invalid API key header value")?,
            );
        }
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        let base_url = network.api_endpoint().to_string();
        Ok(Self { http, base_url })
    }

    /// Fetch transaction history for an address (all types).
    pub async fn transactions(
        &self,
        address: &str,
        limit: u32,
        fingerprint: Option<&str>,
    ) -> Result<TxListResponse> {
        let mut url = format!(
            "{}/v1/accounts/{}/transactions?limit={}&order_by=block_timestamp,desc",
            self.base_url, address, limit,
        );
        if let Some(fp) = fingerprint {
            url.push_str("&fingerprint=");
            url.push_str(&urlencoding::encode(fp));
        }
        let resp: TxListResponse = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }

    /// Fetch TRC20 transfer history for an address.
    pub async fn trc20_transactions(
        &self,
        address: &str,
        limit: u32,
        fingerprint: Option<&str>,
    ) -> Result<Trc20TxListResponse> {
        let mut url = format!(
            "{}/v1/accounts/{}/transactions/trc20?limit={}&order_by=block_timestamp,desc",
            self.base_url, address, limit,
        );
        if let Some(fp) = fingerprint {
            url.push_str("&fingerprint=");
            url.push_str(&urlencoding::encode(fp));
        }
        let resp: Trc20TxListResponse = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }

    /// Fetch full transaction info by txid.
    pub async fn transaction_info(&self, txid: &str) -> Result<TxInfoResponse> {
        let url = format!(
            "{}/wallet/gettransactioninfobyid",
            self.base_url,
        );
        let resp: TxInfoResponse = self
            .http
            .post(&url)
            .json(&serde_json::json!({ "value": txid }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }

    /// Broadcast a signed transaction via REST (TronGrid blocks gRPC broadcast).
    pub async fn broadcast_hex(&self, hex_transaction: &str) -> Result<BroadcastResponse> {
        let url = format!("{}/wallet/broadcasthex", self.base_url);
        let resp: BroadcastResponse = self
            .http
            .post(&url)
            .json(&serde_json::json!({ "transaction": hex_transaction }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if !resp.result.unwrap_or(false) {
            let msg = resp.message.as_deref().unwrap_or("unknown error");
            let code = resp.code.as_deref().unwrap_or("UNKNOWN");
            anyhow::bail!("broadcast failed: {msg} (code: {code})");
        }
        Ok(resp)
    }

    /// Fetch raw transaction by txid.
    pub async fn transaction_by_id(&self, txid: &str) -> Result<RawTxResponse> {
        let url = format!(
            "{}/wallet/gettransactionbyid",
            self.base_url,
        );
        let resp: RawTxResponse = self
            .http
            .post(&url)
            .json(&serde_json::json!({ "value": txid }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }
}

// ── Response types ──

#[derive(Debug, Deserialize)]
pub struct TxListResponse {
    pub data: Vec<TxEntry>,
    pub success: bool,
    pub meta: Option<Meta>,
}

#[derive(Debug, Deserialize)]
pub struct Trc20TxListResponse {
    pub data: Vec<Trc20TxEntry>,
    pub success: bool,
    pub meta: Option<Meta>,
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    pub fingerprint: Option<String>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct TxEntry {
    #[serde(rename = "txID")]
    pub tx_id: String,
    pub block_timestamp: i64,
    pub ret: Option<Vec<TxRet>>,
    pub raw_data: Option<RawData>,
    /// Net fee in SUN.
    pub net_fee: Option<i64>,
    /// Energy fee in SUN.
    pub energy_fee: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TxRet {
    #[serde(rename = "contractRet")]
    pub contract_ret: Option<String>,
    pub fee: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RawData {
    pub contract: Option<Vec<ContractCall>>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ContractCall {
    #[serde(rename = "type")]
    pub contract_type: Option<String>,
    pub parameter: Option<ContractParameter>,
}

#[derive(Debug, Deserialize)]
pub struct ContractParameter {
    pub value: Option<serde_json::Value>,
    pub type_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Trc20TxEntry {
    pub transaction_id: String,
    pub block_timestamp: i64,
    pub from: String,
    pub to: String,
    pub value: String,
    pub token_info: Option<TokenInfo>,
    #[serde(rename = "type")]
    pub transfer_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenInfo {
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub name: Option<String>,
    pub address: Option<String>,
}

// ── Transaction info response ──

#[derive(Debug, Deserialize)]
pub struct TxInfoResponse {
    pub id: Option<String>,
    pub fee: Option<i64>,
    #[serde(rename = "blockNumber")]
    pub block_number: Option<i64>,
    #[serde(rename = "blockTimeStamp")]
    pub block_timestamp: Option<i64>,
    pub receipt: Option<Receipt>,
    #[serde(rename = "contractResult")]
    pub contract_result: Option<Vec<String>>,
    pub result: Option<String>,
    #[serde(rename = "resMessage")]
    pub res_message: Option<String>,
    pub log: Option<Vec<EventLog>>,
}

#[derive(Debug, Deserialize)]
pub struct Receipt {
    pub energy_usage: Option<i64>,
    pub energy_fee: Option<i64>,
    pub energy_usage_total: Option<i64>,
    pub net_usage: Option<i64>,
    pub net_fee: Option<i64>,
    pub result: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventLog {
    pub address: Option<String>,
    pub topics: Option<Vec<String>>,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawTxResponse {
    #[serde(rename = "txID")]
    pub tx_id: Option<String>,
    pub raw_data: Option<RawData>,
    pub ret: Option<Vec<TxRet>>,
    pub raw_data_hex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BroadcastResponse {
    pub result: Option<bool>,
    pub code: Option<String>,
    pub message: Option<String>,
    pub txid: Option<String>,
}
