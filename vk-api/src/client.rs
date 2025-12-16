use anyhow::{Context, Result};
use reqwest::Client;
use std::{collections::HashMap, time::Duration};

use crate::methods::{AccountApi, FriendsApi, LongPollApi, MessagesApi, UsersApi};
use crate::types::*;
use crate::{API_URL as VK_API_URL, API_VERSION as VK_API_VERSION};

/// VK API client
pub struct VkClient {
    client: Client,
    access_token: String,
}

const USER_AGENT: &str = concat!("vk-api-rust/", env!("CARGO_PKG_VERSION"));

impl VkClient {
    /// Create new VK API client
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::builder()
                .user_agent(USER_AGENT)
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .pool_max_idle_per_host(2)
                .build()
                .expect("Failed to build HTTP client"),
            access_token,
        }
    }

    /// Make API request
    pub(crate) async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: HashMap<&str, String>,
    ) -> Result<T> {
        let mut params = params;
        params.insert("access_token", self.access_token.clone());
        params.insert("v", VK_API_VERSION.to_string());

        let url = format!("{}/{}", VK_API_URL, method);

        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to send request")?;

        let text = response
            .text()
            .await
            .context("Failed to read response body")?;
        let vk_response: VkResponse<T> = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}; body: {}", e, text))?;

        if let Some(error) = vk_response.error {
            anyhow::bail!("VK API error {}: {}", error.error_code, error.error_msg);
        }

        vk_response.response.context("Empty response from VK API")
    }

    /// Get access token (for internal use)
    pub(crate) fn token(&self) -> &str {
        &self.access_token
    }

    /// Get HTTP client (for internal use)
    pub(crate) fn http_client(&self) -> &Client {
        &self.client
    }

    // ========== API Namespaces ==========

    /// Access Messages API methods
    pub fn messages(&self) -> MessagesApi<'_> {
        MessagesApi::new(self)
    }

    /// Access Users API methods
    pub fn users(&self) -> UsersApi<'_> {
        UsersApi::new(self)
    }

    /// Access Friends API methods
    pub fn friends(&self) -> FriendsApi<'_> {
        FriendsApi::new(self)
    }

    /// Access Long Poll API methods
    pub fn longpoll(&self) -> LongPollApi<'_> {
        LongPollApi::new(self)
    }

    /// Access Account API methods
    pub fn account(&self) -> AccountApi<'_> {
        AccountApi::new(self)
    }
}
