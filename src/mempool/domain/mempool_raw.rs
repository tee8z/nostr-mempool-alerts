use nostr_sdk::prelude::serde;
use serde::{Deserialize, Serialize};

use super::{
    BackendInfo, Block, Block2, Conversions, Da, Fees, LoadingIndicators, MempoolInfo, Transaction,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MempoolRaw {
    pub mempool_info: MempoolInfo,
    pub v_bytes_per_second: Option<i64>,
    pub blocks: Option<Vec<Block>>,
    pub block: Option<Block>,
    pub conversions: Option<Conversions>,
    #[serde(rename = "mempool-blocks")]
    pub mempool_blocks: Option<Vec<Block2>>,
    pub transactions: Option<Vec<Transaction>>,
    pub backend_info: Option<BackendInfo>,
    pub loading_indicators: Option<LoadingIndicators>,
    pub da: Da,
    pub fees: Fees,
}
//TODO: see if there is a better way to handle this error beside using .expect which will cause a panic to occur
impl From<tokio_tungstenite::tungstenite::Message> for MempoolRaw {
    fn from(raw_message: tokio_tungstenite::tungstenite::Message) -> Self {
        tracing::info!("raw_message: {:?}", raw_message);
    let data: String = String::from_utf8(raw_message.into()).map_err(|e| {
        tracing::error!(
            "error converting message raw data into a string: {:?}",
            e
        );
        e
    })
    .expect("error marshalling mempool websocket data to block root");
        tracing::info!("raw data: {:?}", data);
        
        let converted: MempoolRaw = serde_json::from_str(&data)
            .map_err(|e| {
                tracing::error!(
                    "error converting message raw string into mempool message: {:?}",
                    e
                );
                e
            })
            .expect("error marshalling mempool websocket data to block root");
        Self {
            mempool_info: converted.mempool_info,
            v_bytes_per_second: converted.v_bytes_per_second,
            blocks: converted.blocks,
            block: converted.block,
            conversions: converted.conversions,
            mempool_blocks: converted.mempool_blocks,
            transactions: converted.transactions,
            backend_info: converted.backend_info,
            loading_indicators: converted.loading_indicators,
            da: converted.da,
            fees: converted.fees,
        }
    }
}
