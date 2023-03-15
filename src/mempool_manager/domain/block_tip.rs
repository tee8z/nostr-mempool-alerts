use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockTip {
    pub height: u64,
    pub hash: String,
}