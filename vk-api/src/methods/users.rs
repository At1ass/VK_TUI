//! Users API implementation
//!
//! Provides methods for working with VK users.
//! References: https://dev.vk.com/method/users

use anyhow::Result;
use std::collections::HashMap;

use crate::client::VkClient;
use crate::types::*;

/// Users API namespace
pub struct UsersApi<'a> {
    client: &'a VkClient,
}

impl<'a> UsersApi<'a> {
    pub(crate) fn new(client: &'a VkClient) -> Self {
        Self { client }
    }

    /// Get user info by IDs
    ///
    /// # Arguments
    /// * `user_ids` - User IDs (max: 1000)
    ///
    /// # VK API
    /// Method: users.get
    /// https://dev.vk.com/method/users.get
    pub async fn get(&self, user_ids: &[i64]) -> Result<Vec<User>> {
        let mut params = HashMap::new();
        let ids: Vec<String> = user_ids.iter().map(|id| id.to_string()).collect();
        params.insert("user_ids", ids.join(","));
        params.insert(
            "fields",
            "photo_50,photo_100,online,last_seen,screen_name,verified".to_string(),
        );

        self.client.request("users.get", params).await
    }

    /// Search users
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `count` - Number of results (max: 1000, default: 20)
    ///
    /// # VK API
    /// Method: users.search
    /// https://dev.vk.com/method/users.search
    pub async fn search(&self, query: &str, count: u32) -> Result<Vec<User>> {
        let mut params = HashMap::new();
        params.insert("q", query.to_string());
        params.insert("count", count.to_string());
        params.insert(
            "fields",
            "photo_50,photo_100,online,screen_name".to_string(),
        );

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            items: Vec<User>,
        }

        let response: Response = self.client.request("users.search", params).await?;
        Ok(response.items)
    }

    /// Get user subscriptions (communities and users)
    ///
    /// # VK API
    /// Method: users.getSubscriptions
    /// https://dev.vk.com/method/users.getSubscriptions
    pub async fn get_subscriptions(&self, user_id: i64) -> Result<Subscriptions> {
        let mut params = HashMap::new();
        params.insert("user_id", user_id.to_string());
        params.insert("extended", "0".to_string());

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            #[serde(default)]
            users: UsersResponse,
            #[serde(default)]
            groups: GroupsResponse,
        }

        #[derive(Debug, serde::Deserialize, Default)]
        struct UsersResponse {
            #[serde(default)]
            items: Vec<i64>,
        }

        #[derive(Debug, serde::Deserialize, Default)]
        struct GroupsResponse {
            #[serde(default)]
            items: Vec<i64>,
        }

        let response: Response = self
            .client
            .request("users.getSubscriptions", params)
            .await?;

        Ok(Subscriptions {
            users: response.users.items,
            groups: response.groups.items,
        })
    }
}

/// User subscriptions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Subscriptions {
    pub users: Vec<i64>,
    pub groups: Vec<i64>,
}
