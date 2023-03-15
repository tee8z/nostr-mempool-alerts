use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedFees {
    pub fastest_fee: i64,
    pub half_hour_fee: i64,
    pub hour_fee: i64,
    pub economy_fee: i64,
    pub minimum_fee: i64,
}
