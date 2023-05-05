use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::mempool::{BlockTip, RecommendedFees};

use super::{AlertKind, State};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Alert {
    pub id: i64,
    pub kind: AlertKind,
    pub active: bool,
    pub should_send: bool,
    pub requestor_pk: String, // create a specific type for this
    pub threshold_num: Option<f64>,
    pub event_data_identifier: Option<String>, //holds utxo or transactionID depending on alert kind
    pub block_state: Option<Json<State>>,
}

impl Alert {
    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn transaction_found(&self) -> bool {
        self.block_state.clone().is_some()
            && self
                .block_state
                .clone()
                .unwrap()
                .transaction_found
                .is_some()
            && self.block_state.clone().unwrap().transaction_found.unwrap()
    }

    pub fn confirm_height_has_reached(&self, cur_block_tip: BlockTip) -> bool {
        let store_block_state = self.block_state.clone();
        if let Some(block_state) = store_block_state {
            if let Some(threshold_num) = self.threshold_num {
                let delta = cur_block_tip.height - block_state.block_tip.clone().unwrap().height;
                return threshold_num <= (delta) as f64;
            }
        }
        false
    }

    pub fn has_reached_fee_level(&self, cur_fees: RecommendedFees) -> bool {
        if self.block_state.is_some() && self.block_state.clone().unwrap().fees.is_some() {
            return self
                .block_state
                .clone()
                .unwrap()
                .fees
                .unwrap()
                .half_hour_fee
                <= cur_fees.half_hour_fee;
        }
        false
    }
}
