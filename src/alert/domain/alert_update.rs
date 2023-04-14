use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use super::{Alert, State};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertUpdate {
    pub id: i64,
    pub active: bool,
    pub block_state: Option<Json<State>>,
}

impl fmt::Display for AlertUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let alert_update_str =
            serde_json::to_string(self).expect("error marshalling into a string for alert_update");
        write!(f, "{}", alert_update_str)
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
