use super::{BlockTip, RecommendedFees, TransactionID};
use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MempoolData {
    pub block: BlockTip,
    pub transactions: Option<Vec<TransactionID>>,
    pub fees: Option<RecommendedFees>,
}
