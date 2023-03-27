use serde::{Deserialize, Serialize};
use sqlx::{types::Json};

use super::{Alert,State};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertUpdate {
    pub id: i64,
    pub active: bool,
    pub block_state: Option<Json<State>>,
}

impl AlertUpdate {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).expect("error marshalling into a string for alert_update")
    }
}

impl From<Alert> for AlertUpdate {
    fn from(alert: Alert) -> Self {
        AlertUpdate {
            id: alert.id,
            active: alert.active,
            block_state: alert.block_state,
        }
    }
}
impl From<&Alert> for AlertUpdate {
    fn from(alert: &Alert) -> Self {
        AlertUpdate {
            id: alert.id,
            active: alert.active,
            block_state: alert.block_state.clone(),
        }
    }
}
