use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TransactionID {
    pub tx_id: String,
}
impl From<String> for TransactionID {
    fn from(val: String) -> Self {
        Self { tx_id: val }
    }
}