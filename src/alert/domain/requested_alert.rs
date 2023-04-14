use serde::{Deserialize, Serialize};

use super::AlertKind;

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestedAlert {
    pub kind: AlertKind,      //TODO: change to enum
    pub requestor_pk: String, //TODO: change to validator type

    pub threshold_num: Option<u64>,
    pub event_data_identifier: Option<String>, //TODO: possible a validator type?
}
//TODO: see if there is a safer way to do this without using expect()
impl From<String> for RequestedAlert {
    fn from(request: String) -> Self {
        let val: RequestedAlert = serde_json::from_str(&request)
            .expect("error trying to convert request alert json into struct");
        Self {
            kind: val.kind,
            requestor_pk: val.requestor_pk,
            threshold_num: val.threshold_num,
            event_data_identifier: val.event_data_identifier,
        }
    }
}
