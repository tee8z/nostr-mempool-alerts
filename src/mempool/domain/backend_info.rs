use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    pub hostname: String,
    pub version: String,
    pub git_commit: String,
    pub lightning: bool,
}
