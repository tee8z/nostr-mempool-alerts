use serde::{Serialize, Deserialize};

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
