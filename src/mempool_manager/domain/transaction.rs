use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub txid: String,
    pub fee: i64,
    pub vsize: f64,
    pub value: i64,
}