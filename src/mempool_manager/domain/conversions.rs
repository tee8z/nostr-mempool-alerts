use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversions {
    pub time: i64,
    #[serde(rename = "USD")]
    pub usd: i64,
    #[serde(rename = "EUR")]
    pub eur: i64,
    #[serde(rename = "GBP")]
    pub gbp: i64,
    #[serde(rename = "CAD")]
    pub cad: i64,
    #[serde(rename = "CHF")]
    pub chf: i64,
    #[serde(rename = "AUD")]
    pub aud: i64,
    #[serde(rename = "JPY")]
    pub jpy: i64,
}
