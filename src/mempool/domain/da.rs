use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Da {
    pub progress_percent: f64,
    pub difficulty_change: f64,
    pub estimated_retarget_date: i64,
    pub remaining_blocks: i64,
    pub remaining_time: i64,
    pub previous_retarget: f64,
    pub previous_time: i64,
    pub next_retarget_height: i64,
    pub time_avg: i64,
    pub time_offset: i64,
    pub expected_blocks: f64,
}
