//! Friends API implementation
//!
//! Provides methods for working with VK friends.
//! References: https://dev.vk.com/method/friends

use anyhow::Result;
use std::collections::HashMap;

use crate::client::VkClient;
use crate::types::*;

/// Friends API namespace
pub struct FriendsApi<'a> {
    client: &'a VkClient,
}

impl<'a> FriendsApi<'a> {
    pub(crate) fn new(client: &'a VkClient) -> Self {
        Self { client }
    }

    /// Get friends list
    ///
    /// # Arguments
    /// * `user_id` - User ID (None for current user)
    ///
    /// # VK API
    /// Method: friends.get
    /// https://dev.vk.com/method/friends.get
    pub async fn get(&self, user_id: Option<i64>) -> Result<Vec<User>> {
        let mut params = HashMap::new();
        if let Some(uid) = user_id {
            params.insert("user_id", uid.to_string());
        }
        params.insert(
            "fields",
            "photo_50,photo_100,online,last_seen,screen_name".to_string(),
        );
        params.insert("order", "hints".to_string());

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            items: Vec<User>,
        }

        let response: Response = self.client.request("friends.get", params).await?;
        Ok(response.items)
    }

    /// Get online friends
    ///
    /// # VK API
    /// Method: friends.getOnline
    /// https://dev.vk.com/method/friends.getOnline
    pub async fn get_online(&self) -> Result<Vec<i64>> {
        let params = HashMap::new();

        self.client.request("friends.getOnline", params).await
    }

    /// Search in friends
    ///
    /// # Arguments
    /// * `query` - Search query
    ///
    /// # VK API
    /// Method: friends.search
    /// https://dev.vk.com/method/friends.search
    pub async fn search(&self, query: &str) -> Result<Vec<User>> {
        let mut params = HashMap::new();
        params.insert("q", query.to_string());
        params.insert(
            "fields",
            "photo_50,photo_100,online,screen_name".to_string(),
        );

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            items: Vec<User>,
        }

        let response: Response = self.client.request("friends.search", params).await?;
        Ok(response.items)
    }

    /// Get recently added friends
    ///
    /// # Arguments
    /// * `count` - Number of friends to return (max: 1000)
    ///
    /// # VK API
    /// Method: friends.getRecent
    /// https://dev.vk.com/method/friends.getRecent
    pub async fn get_recent(&self, count: u32) -> Result<Vec<i64>> {
        let mut params = HashMap::new();
        params.insert("count", count.to_string());

        self.client.request("friends.getRecent", params).await
    }
}
