use serde::{Deserialize, Serialize};

/// User info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,

    #[serde(default)]
    pub photo_50: Option<String>,

    #[serde(default)]
    pub photo_100: Option<String>,

    #[serde(default)]
    pub online: Option<i32>,

    #[serde(default)]
    pub screen_name: Option<String>,
}

impl User {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_online(&self) -> bool {
        self.online == Some(1)
    }
}

/// Last seen info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LastSeen {
    pub time: i64,
    pub platform: i32,
}
