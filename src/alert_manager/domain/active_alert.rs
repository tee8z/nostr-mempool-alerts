use serde::{Deserialize, Serialize};
use sqlx::{types::Json};

use super::{AlertKind, State};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Alert {
    pub id: i64,
    pub kind: AlertKind,
    pub active: bool,
    pub should_send: bool,
    pub requestor_pk: String, // create a specific type for this
    pub threshold_num: Option<u64>,
    pub event_data_identifier: Option<String>, //holds utxo or transactionID depending on alert kind
    pub block_state: Option<Json<State>>,
}

impl Alert {
    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
