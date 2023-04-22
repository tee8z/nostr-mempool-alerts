use serde::{Deserialize, Serialize};

use super::AlertKind;

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestedAlert {
    pub kind: AlertKind,      //change to enum
    pub requestor_pk: String, // change to validator type

    pub threshold_num: Option<f64>,
    pub event_data_identifier: Option<String>, // possible a validator type?
}
impl TryFrom<String> for RequestedAlert {
    type Error = serde_json::Error;
    fn try_from(request: String) -> Result<Self, Self::Error> {
        tracing::info!("request:{:?}", request);
        match serde_json::from_str::<RequestedAlert>(&request) {
            Ok(val) => Ok(Self {
                kind: val.kind,
                requestor_pk: val.requestor_pk,
                threshold_num: val.threshold_num,
                event_data_identifier: val.event_data_identifier,
            }),
            Err(e) => Err(e),
        }
    }
}
