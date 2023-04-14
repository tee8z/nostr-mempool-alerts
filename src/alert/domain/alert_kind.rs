use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::mempool::{MempoolData, TransactionID};

use super::{Alert, State};
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[repr(i32)]
pub enum AlertKind {
    //    UtxoMovement(i32), -- TODO: implement this alert at a later date
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

impl AlertKindHandler for AlertKind {
    #[instrument(skip_all)]
    fn update_block_height_alert(&self, mut alert: Alert, new_block: MempoolData) -> Option<Alert> {
        let height = Some(new_block.block.height);
        if alert.threshold_num >= height {
            alert.should_send = true;
            alert.block_state = Some(sqlx::types::Json(State {
                fees: Some(new_block.fees),
                block_tip: Some(new_block.block),
                transaction_found: Some(false),
            }));
            alert.active = false;
            return Some(alert);
        }

        None
    }

    #[instrument(skip_all)]
    fn update_confirm_height_alert(
        &self,
        mut alert: Alert,
        new_block: MempoolData,
    ) -> Option<Alert> {
        alert.event_data_identifier.as_ref()?;

        let transaction_id = alert.event_data_identifier.clone().unwrap();
        if new_block
            .transactions
            .contains(&TransactionID::from(transaction_id))
        {
            alert.block_state = Some(sqlx::types::Json(State {
                fees: Some(new_block.fees),
                block_tip: Some(new_block.block),
                transaction_found: Some(true),
            }));
            return Some(alert);
        }
        if alert.transaction_found() && alert.confirm_height_has_reached(new_block.block) {
            alert.should_send = true;
            alert.active = false;
            return Some(alert);
        }
        None
    }

    #[instrument(skip_all)]
    fn update_fee_level_alert(&self, mut alert: Alert, new_block: MempoolData) -> Option<Alert> {
        if alert.has_reached_fee_level(new_block.fees) {
            alert.should_send = true;
            alert.block_state = Some(sqlx::types::Json(State {
                fees: Some(new_block.fees),
                block_tip: Some(new_block.block),
                transaction_found: Some(false),
            }));
            alert.active = false;
            return Some(alert);
        }
        None
    }
}
