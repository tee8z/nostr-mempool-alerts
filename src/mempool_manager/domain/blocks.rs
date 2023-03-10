use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockExtraPool {
    pub id: u16,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockExtra {
    #[serde(rename = "coinbaseRaw")]
    pub coinbase_raw: String,
    #[serde(rename = "medianFee")]
    pub median_fee: u64,
    #[serde(rename = "feeRange")]
    pub fee_range: Vec<u64>,
    pub reward: u64,
    #[serde(rename = "totalFees")]
    pub total_fees: u64,
    #[serde(rename = "avgFee")]
    pub avg_fee: u64,
    #[serde(rename = "avgFeeRate")]
    pub avg_fee_rate: u64,
    pub pool: BlockExtraPool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Block {
    pub id: String,
    pub timestamp: u64,
    pub height: u32,
    pub version: u32,
    pub bits: u64,
    pub nonce: u64,
    pub difficulty: f64,
    pub merkle_root: String,
    pub tx_count: u32,
    pub size: u32,
    pub weight: u32,
    #[serde(rename = "previousblockhash")]
    pub previous_block_hash: String,
    pub extras: BlockExtra,
}
