use serde::Deserialize;

/// Upload server info
#[derive(Debug, Deserialize)]
pub struct UploadServer {
    pub upload_url: String,
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
    pub id: i64,
    pub owner_id: i64,
}
