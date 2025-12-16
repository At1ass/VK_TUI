use serde::{Deserialize, Serialize};

/// VK Group/Community
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub screen_name: String,

    #[serde(default)]
    pub photo_50: Option<String>,

    #[serde(default)]
    pub photo_100: Option<String>,

    #[serde(default)]
    pub photo_200: Option<String>,

    #[serde(default)]
    pub is_closed: Option<i32>,

    #[serde(default)]
    pub verified: Option<bool>,

    #[serde(default, rename = "type")]
    pub group_type: Option<String>,
}
