use serde::{Deserialize, Serialize};

/// VK API response wrapper
#[derive(Debug, Deserialize)]
pub struct VkResponse<T> {
    pub response: Option<T>,
    pub error: Option<VkError>,
}

/// VK API error
#[derive(Debug, Deserialize)]
pub struct VkError {
    pub error_code: i32,
    pub error_msg: String,
}

/// Peer info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Peer {
    pub id: i64,
    #[serde(rename = "type")]
    pub peer_type: String,
    #[serde(default)]
    pub local_id: Option<i64>,
}

/// Deserialize ts which can be either string or number
pub(crate) fn deserialize_ts<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        _ => Err(D::Error::custom("expected string or number for ts")),
    }
}

/// Deserialize optional ts
pub(crate) fn deserialize_ts_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = serde::Deserialize::deserialize(deserializer)?;
    match value {
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(serde_json::Value::Null) | None => Ok(None),
        _ => Ok(None),
    }
}
