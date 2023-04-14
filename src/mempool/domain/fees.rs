use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fees {
    pub fastest_fee: i64,
    pub half_hour_fee: i64,
    pub hour_fee: i64,
    pub economy_fee: i64,
    pub minimum_fee: i64,
}
