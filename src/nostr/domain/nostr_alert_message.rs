#[derive(Clone)]
pub struct NostrAlertMessage {
    pub client_pk: String,
    pub val: String, //TODO: make into a custom type
    pub id: i64,
}
