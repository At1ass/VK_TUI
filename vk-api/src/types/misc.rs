use serde::{Deserialize, Serialize};

/// City information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct City {
    pub id: i64,
    pub title: String,
}

/// Country information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Country {
    pub id: i64,
    pub title: String,
}

/// Account counters (unread messages, friend requests, etc.)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Counters {
    #[serde(default)]
    pub messages: Option<u32>,

    #[serde(default)]
    pub friends: Option<u32>,

    #[serde(default)]
    pub notifications: Option<u32>,

    #[serde(default)]
    pub groups: Option<u32>,
}

/// Profile information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileInfo {
    pub first_name: String,
    pub last_name: String,

    #[serde(default)]
    pub screen_name: Option<String>,

    #[serde(default)]
    pub status: Option<String>,

    #[serde(default)]
    pub bdate: Option<String>,

    #[serde(default)]
    pub city: Option<City>,

    #[serde(default)]
    pub country: Option<Country>,

    #[serde(default)]
    pub home_town: Option<String>,
}

/// Can write status for conversations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CanWrite {
    pub allowed: bool,

    #[serde(default)]
    pub reason: Option<i32>,
}
