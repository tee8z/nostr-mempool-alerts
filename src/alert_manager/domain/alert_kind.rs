use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::mempool_manager::{MempoolData, TransactionID};

use super::{Alert, State};
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[repr(i32)]
pub enum AlertKind {
    //    UtxoMovement(i32), -- TODO at a later date
    ConfirmHeight = 1001,
    FeeLevel = 1002,
    BlockHeight = 1003,
}
impl AlertKind {
    pub fn to_int(&self) -> i32 {
        match &self {
            AlertKind::ConfirmHeight => 1001,
            AlertKind::FeeLevel => 1002,
            AlertKind::BlockHeight => 1003,
        }
    }
}

//TODO: maybe we remove the possible panic! here and just return Options?
impl From<i32> for AlertKind {
    fn from(id: i32) -> Self {
        match id {
            1001 => AlertKind::ConfirmHeight,
            1002 => AlertKind::FeeLevel,
            1003 => AlertKind::BlockHeight,
            _ => panic!("unsupported id for alert kind"),
        }
    }
}

impl From<String> for AlertKind {
    fn from(val: String) -> Self {
        match val.as_ref() {
            "CONFIRM_HEIGHT" => AlertKind::ConfirmHeight,
            "FEE_LEVEL" => AlertKind::FeeLevel,
            "BLOCK_HEIGHT" => AlertKind::BlockHeight,
            _ => panic!("unsupported val for alert kind"),
        }
    }
}

pub trait AlertKindHandler {
    fn update_block_height_alert(&self, alert: Alert, new_block: MempoolData) -> Option<Alert>;
    fn update_confirm_height_alert(&self, alert: Alert, new_block: MempoolData) -> Option<Alert>;
    fn update_fee_level_alert(&self, alert: Alert, new_block: MempoolData) -> Option<Alert>;
}
//functions should determine if the alert is still active and a notification should be sent
impl AlertKindHandler for AlertKind {
    #[instrument(skip_all)]
    fn update_block_height_alert(&self, mut alert: Alert, new_block: MempoolData) -> Option<Alert> {
        let height = Some(new_block.block.height.clone());
        if alert.threshold_num >= height.clone() {
            alert.should_send = true;
            alert.block_state = Some(sqlx::types::Json(State {
                fees: Some(new_block.fees),
                block_tip: Some(new_block.block),
                transaction_found: Some(false),
            }));
            alert.active = false;
            return Some(alert);
        }

        return None;
    }

    #[instrument(skip_all)]
    fn update_confirm_height_alert(
        &self,
        mut alert: Alert,
        new_block: MempoolData,
    ) -> Option<Alert> {
        if !alert.event_data_identifier.is_some() {
            return None;
        }
        let transaction_id = alert.event_data_identifier.clone().unwrap();
        if new_block
            .transactions
            .contains(&TransactionID::from(transaction_id))
        {
            alert.should_send = true;
            alert.block_state = Some(sqlx::types::Json(State {
                fees: Some(new_block.fees),
                block_tip: Some(new_block.block),
                transaction_found: Some(true),
            }));
            alert.active = false;
            return Some(alert.clone());
        }
        return None;
    }

    #[instrument(skip_all)]
    fn update_fee_level_alert(&self, mut alert: Alert, new_block: MempoolData) -> Option<Alert> {
        if alert.threshold_num.is_some()
            && (alert.threshold_num.unwrap() as i64) <= new_block.fees.half_hour_fee
        {
            alert.should_send = true;
            alert.block_state = Some(sqlx::types::Json(State {
                fees: Some(new_block.fees),
                block_tip: Some(new_block.block),
                transaction_found: Some(false),
            }));
            alert.active = false;
            return Some(alert);
        }
        return None;
    }
}
