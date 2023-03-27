use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};

use super::Extras;

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