//! Long Poll API implementation
//!
//! Provides methods for working with VK Long Poll server for real-time updates.
//! References: https://dev.vk.com/api/user-long-poll/getting-started

use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::client::VkClient;
use crate::types::*;

/// Long Poll API namespace
pub struct LongPollApi<'a> {
    client: &'a VkClient,
}

impl<'a> LongPollApi<'a> {
    pub(crate) fn new(client: &'a VkClient) -> Self {
        Self { client }
    }

    /// Get Long Poll server info
    ///
    /// This method returns the server address, key, and timestamp for establishing
    /// a Long Poll connection to receive real-time updates.
    ///
    /// # VK API
    /// Method: messages.getLongPollServer
    /// https://dev.vk.com/method/messages.getLongPollServer
    pub async fn get_server(&self) -> Result<LongPollServer> {
        let mut params = HashMap::new();
        params.insert("lp_version", "3".to_string());

        self.client
            .request("messages.getLongPollServer", params)
            .await
    }

    /// Poll for updates
    ///
    /// Makes a request to the Long Poll server to get new events.
    /// This is a long-polling request that waits up to 25 seconds for new events.
    ///
    /// # Arguments
    /// * `server` - Long Poll server info obtained from `get_server()`
    ///
    /// # Returns
    /// LongPollResponse with updates and new timestamp
    ///
    /// # Mode flags
    /// - 2: Receive attachments
    /// - 8: Return extended events
    /// - 32: Return pts for messages.getLongPollHistory
    /// - 64: Return random_id in message events
    /// - 128: Return extra fields
    ///
    /// Total mode: 234 = 2 + 8 + 32 + 64 + 128
    ///
    /// # VK API
    /// https://dev.vk.com/api/user-long-poll/getting-started
    pub async fn poll(&self, server: &LongPollServer) -> Result<LongPollResponse> {
        // mode=234: attachments(2) + extended_events(8) + pts(32) + random_id(64) + extra_fields(128)
        let url = format!(
            "https://{}?act=a_check&key={}&ts={}&wait=25&mode=234&version=3",
            server.server, server.key, server.ts
        );

        let response = self
            .client
            .http_client()
            .get(&url)
            .send()
            .await
            .context("Long poll request failed")?;

        response
            .json()
            .await
            .context("Failed to parse long poll response")
    }

    /// Get history of missed events
    ///
    /// Retrieves events that occurred between the specified timestamp and the current time.
    /// Useful for catching up on missed events after a connection loss.
    ///
    /// # Arguments
    /// * `ts` - Timestamp to get history from
    /// * `pts` - Optional pts value from previous poll response
    ///
    /// # VK API
    /// Method: messages.getLongPollHistory
    /// https://dev.vk.com/method/messages.getLongPollHistory
    pub async fn get_history(&self, ts: &str, pts: Option<i64>) -> Result<LongPollHistory> {
        let mut params = HashMap::new();
        params.insert("ts", ts.to_string());

        if let Some(p) = pts {
            params.insert("pts", p.to_string());
        }

        params.insert("lp_version", "3".to_string());

        self.client
            .request("messages.getLongPollHistory", params)
            .await
    }
}

/// Long Poll history response
#[derive(Debug, serde::Deserialize)]
pub struct LongPollHistory {
    pub messages: Vec<Message>,

    #[serde(default)]
    pub profiles: Vec<User>,

    #[serde(default)]
    pub new_pts: Option<i64>,
}
