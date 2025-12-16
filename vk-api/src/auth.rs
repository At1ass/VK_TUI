use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const VK_APP_ID: &str = "6287487"; // Standalone app ID (Kate Mobile)
const VK_AUTH_URL: &str = "https://oauth.vk.com/authorize";
const VK_API_VERSION: &str = "5.199";

/// Token data stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub user_id: i64,
    pub expires_at: Option<i64>,
}

/// Authentication manager
pub struct AuthManager {
    config_path: PathBuf,
    token: Option<TokenData>,
}

impl AuthManager {
    /// Create new auth manager
    pub fn new() -> Result<Self> {
        let config_dir = directories::ProjectDirs::from("", "", "vk_tui")
            .context("Could not determine config directory")?
            .config_dir()
            .to_path_buf();

        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("token.json");

        let token = if config_path.exists() {
            let data = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&data).ok()
        } else {
            None
        };

        Ok(Self { config_path, token })
    }

    /// Check if we have a valid token
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Get access token
    pub fn access_token(&self) -> Option<&str> {
        self.token.as_ref().map(|t| t.access_token.as_str())
    }

    /// Get user ID
    pub fn user_id(&self) -> Option<i64> {
        self.token.as_ref().map(|t| t.user_id)
    }

    /// Generate OAuth URL for user to authenticate
    pub fn get_auth_url() -> String {
        format!(
            "{}?client_id={}&display=page&redirect_uri=https://oauth.vk.com/blank.html&scope=messages,friends,photos,offline&response_type=token&v={}",
            VK_AUTH_URL, VK_APP_ID, VK_API_VERSION
        )
    }

    /// Save token from redirect URL
    pub fn save_token_from_url(&mut self, url: &str) -> Result<()> {
        // Normalize URL: users sometimes paste //oauth.vk.com/blank.html#...
        let normalized = if url.starts_with("//") {
            format!("https:{}", url)
        } else if !url.contains("://") && url.starts_with("oauth.vk.com/") {
            format!("https://{}", url)
        } else {
            url.to_string()
        };

        // URL format: https://oauth.vk.com/blank.html#access_token=...&expires_in=...&user_id=...
        let fragment = normalized
            .split('#')
            .nth(1)
            .context("No fragment in URL (expected #access_token=...)")?;

        let mut access_token = None;
        let mut user_id = None;
        let mut expires_in: Option<i64> = None;

        for pair in fragment.split('&') {
            let mut parts = pair.split('=');
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");

            match key {
                "access_token" => access_token = Some(value.to_string()),
                "user_id" => user_id = value.parse().ok(),
                "expires_in" => expires_in = value.parse().ok(),
                _ => {}
            }
        }

        let access_token = access_token.context("No access_token in URL")?;
        let user_id = user_id.context("No user_id in URL")?;

        let expires_at = expires_in.map(|e| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                + e
        });

        let token = TokenData {
            access_token,
            user_id,
            expires_at,
        };

        let data = serde_json::to_string_pretty(&token)?;

        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&self.config_path, data)?;

        self.token = Some(token);
        Ok(())
    }

    /// Clear saved token
    #[allow(dead_code)]
    pub fn logout(&mut self) -> Result<()> {
        if self.config_path.exists() {
            std::fs::remove_file(&self.config_path)?;
        }
        self.token = None;
        Ok(())
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            config_path: PathBuf::from("token.json"),
            token: None,
        })
    }
}
