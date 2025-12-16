use serde::Deserialize;

/// Upload server info
#[derive(Debug, Deserialize)]
pub struct UploadServer {
    pub upload_url: String,
}

/// Photo upload response
#[derive(Debug, Deserialize)]
pub struct PhotoUploadResponse {
    pub server: i64,
    pub photo: String,
    pub hash: String,
}

/// Saved photo info
#[derive(Debug, Deserialize)]
pub struct SavedPhoto {
    pub id: i64,
    pub owner_id: i64,
}

/// Saved document info
#[derive(Debug, Deserialize)]
pub struct SavedDoc {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub doc: Option<DocInfo>,
}

/// Document info returned from docs.save
#[derive(Debug, Deserialize)]
pub struct DocInfo {
    pub id: i64,
    pub owner_id: i64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub ext: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Document upload response (docs.getMessagesUploadServer upload result)
#[derive(Debug, Deserialize)]
pub struct UploadDocResponse {
    pub file: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_descr: Option<String>,
}
