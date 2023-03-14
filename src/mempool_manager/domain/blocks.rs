use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedFees {
    pub fastest_fee: i64,
    pub half_hour_fee: i64,
    pub hour_fee: i64,
    pub economy_fee: i64,
    pub minimum_fee: i64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionID {
    pub tx_id: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockTip {
    pub height: u64,
    pub hash: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MempoolData {
    pub block: BlockTip,
    pub transactions: Vec<TransactionID>,
    pub fees: RecommendedFees
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MempoolRaw {
    pub mempool_info: MempoolInfo,
    pub v_bytes_per_second: i64,
    pub blocks: Vec<Block>,
    pub conversions: Conversions,
    #[serde(rename = "mempool-blocks")]
    pub mempool_blocks: Vec<Block2>,
    pub transactions: Vec<Transaction>,
    pub backend_info: BackendInfo,
    pub loading_indicators: LoadingIndicators,
    pub da: Da,
    pub fees: Fees,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MempoolInfo {
    pub loaded: bool,
    pub size: i64,
    pub bytes: i64,
    pub usage: i64,
    #[serde(rename = "total_fee")]
    pub total_fee: f64,
    pub maxmempool: i64,
    pub mempoolminfee: f64,
    pub minrelaytxfee: f64,
    pub unbroadcastcount: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub id: String,
    pub height: i64,
    pub version: i64,
    pub timestamp: i64,
    pub bits: i64,
    pub nonce: i64,
    pub difficulty: f64,
    #[serde(rename = "merkle_root")]
    pub merkle_root: String,
    #[serde(rename = "tx_count")]
    pub tx_count: i64,
    pub size: i64,
    pub weight: i64,
    pub previousblockhash: String,
    pub mediantime: i64,
    pub extras: Extras,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extras {
    pub reward: i64,
    pub coinbase_raw: String,
    pub orphans: Vec<Value>,
    pub median_fee: i64,
    pub fee_range: Vec<i64>,
    pub total_fees: i64,
    pub avg_fee: i64,
    pub avg_fee_rate: i64,
    pub utxo_set_change: i64,
    pub avg_tx_size: f64,
    pub total_inputs: i64,
    pub total_outputs: i64,
    pub total_output_amt: i64,
    pub segwit_total_txs: i64,
    pub segwit_total_size: i64,
    pub segwit_total_weight: i64,
    pub fee_percentiles: Value,
    pub virtual_size: f64,
    pub coinbase_address: String,
    pub coinbase_signature: String,
    pub coinbase_signature_ascii: String,
    pub header: String,
    pub utxo_set_size: Value,
    pub total_input_amt: Value,
    pub pool: Pool,
    pub match_rate: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pool {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversions {
    pub time: i64,
    #[serde(rename = "USD")]
    pub usd: i64,
    #[serde(rename = "EUR")]
    pub eur: i64,
    #[serde(rename = "GBP")]
    pub gbp: i64,
    #[serde(rename = "CAD")]
    pub cad: i64,
    #[serde(rename = "CHF")]
    pub chf: i64,
    #[serde(rename = "AUD")]
    pub aud: i64,
    #[serde(rename = "JPY")]
    pub jpy: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block2 {
    pub block_size: i64,
    #[serde(rename = "blockVSize")]
    pub block_vsize: f64,
    pub n_tx: i64,
    pub total_fees: i64,
    pub median_fee: f64,
    pub fee_range: Vec<f64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub txid: String,
    pub fee: i64,
    pub vsize: f64,
    pub value: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    pub hostname: String,
    pub version: String,
    pub git_commit: String,
    pub lightning: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadingIndicators {
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Da {
    pub progress_percent: f64,
    pub difficulty_change: f64,
    pub estimated_retarget_date: i64,
    pub remaining_blocks: i64,
    pub remaining_time: i64,
    pub previous_retarget: f64,
    pub previous_time: i64,
    pub next_retarget_height: i64,
    pub time_avg: i64,
    pub time_offset: i64,
    pub expected_blocks: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fees {
    pub fastest_fee: i64,
    pub half_hour_fee: i64,
    pub hour_fee: i64,
    pub economy_fee: i64,
    pub minimum_fee: i64,
}