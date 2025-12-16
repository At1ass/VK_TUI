//! Account API implementation
//!
//! Provides methods for working with account settings and counters.
//! References: https://dev.vk.com/method/account

use anyhow::Result;
use std::collections::HashMap;

use crate::client::VkClient;
use crate::types::*;

/// Account API namespace
pub struct AccountApi<'a> {
    client: &'a VkClient,
}

impl<'a> AccountApi<'a> {
    pub(crate) fn new(client: &'a VkClient) -> Self {
        Self { client }
    }

    /// Get counters (messages, friends, notifications, etc.)
    ///
    /// Returns unread counts for various sections of VK.
    /// Useful for displaying badges with unread counts in UI.
    ///
    /// # VK API
    /// Method: account.getCounters
    /// https://dev.vk.com/method/account.getCounters
    pub async fn get_counters(&self) -> Result<Counters> {
        let mut params = HashMap::new();
        params.insert(
            "filter",
            "messages,friends,notifications,groups".to_string(),
        );

        self.client.request("account.getCounters", params).await
    }

    /// Get profile info
    ///
    /// Returns current user's profile information including name, status, etc.
    ///
    /// # VK API
    /// Method: account.getProfileInfo
    /// https://dev.vk.com/method/account.getProfileInfo
    pub async fn get_profile_info(&self) -> Result<ProfileInfo> {
        let params = HashMap::new();

        self.client.request("account.getProfileInfo", params).await
    }

    /// Set online status
    ///
    /// Marks the user as online. The online status is automatically
    /// reset after a period of inactivity.
    ///
    /// # VK API
    /// Method: account.setOnline
    /// https://dev.vk.com/method/account.setOnline
    pub async fn set_online(&self) -> Result<()> {
        let params = HashMap::new();

        let _: i32 = self.client.request("account.setOnline", params).await?;
        Ok(())
    }

    /// Set offline status
    ///
    /// Marks the user as offline.
    ///
    /// # VK API
    /// Method: account.setOffline
    /// https://dev.vk.com/method/account.setOffline
    pub async fn set_offline(&self) -> Result<()> {
        let params = HashMap::new();

        let _: i32 = self.client.request("account.setOffline", params).await?;
        Ok(())
    }
}
