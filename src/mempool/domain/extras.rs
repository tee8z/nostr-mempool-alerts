use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Pool;

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
