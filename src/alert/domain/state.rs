use serde::{Deserialize, Serialize};

use crate::mempool::{BlockTip, RecommendedFees};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub fees: Option<RecommendedFees>,
    pub block_tip: Option<BlockTip>,
    pub transaction_found: Option<bool>,
}
