use serde::{Deserialize, Serialize};

/// Message attachment
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Attachment {
    #[serde(rename = "type")]
    pub attachment_type: String,

    #[serde(default)]
    pub photo: Option<Photo>,

    #[serde(default)]
    pub doc: Option<Doc>,
}

/// Photo attachment
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Photo {
    pub id: i64,
    pub owner_id: i64,

    #[serde(default)]
    pub sizes: Vec<PhotoSize>,
}

/// Photo size info
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PhotoSize {
    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub width: Option<u32>,

    #[serde(default)]
    pub height: Option<u32>,
}

/// Document attachment
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Doc {
    pub id: i64,
    pub owner_id: i64,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub size: Option<u64>,

    #[serde(default, rename = "ext")]
    pub extension: Option<String>,
}
