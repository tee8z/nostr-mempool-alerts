use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};

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
