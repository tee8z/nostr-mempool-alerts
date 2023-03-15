use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};
use super::{RecommendedFees, TransactionID, BlockTip};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MempoolData {
    pub block: BlockTip,
    pub transactions: Vec<TransactionID>,
    pub fees: RecommendedFees,
}