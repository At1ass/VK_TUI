use serde::Deserialize;

use super::common::{deserialize_ts, deserialize_ts_option};

/// Long Poll server info
#[derive(Debug, Clone, Deserialize)]
pub struct LongPollServer {
    pub key: String,
    pub server: String,

    #[serde(deserialize_with = "deserialize_ts")]
    pub ts: String,
}

/// Long Poll response
#[derive(Debug, Deserialize)]
pub struct LongPollResponse {
    #[serde(default, deserialize_with = "deserialize_ts_option")]
    pub ts: Option<String>,

    pub updates: Option<Vec<serde_json::Value>>,
    pub failed: Option<i32>,
}
