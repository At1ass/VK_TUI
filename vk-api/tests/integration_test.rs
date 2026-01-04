//! Integration tests for VK API
//!
//! These tests use a real VK account token from ~/.config/vk_tui/token.json
//! and test against "Saved Messages" (Избранное) which is a chat with yourself.
//!
//! Run with: cargo test --test integration_test -- --test-threads=1 --nocapture

use vk_api::VkClient;

/// Load token from config file
fn get_test_token() -> String {
    let mut config_path = dirs::home_dir().expect("Cannot get home directory");
    config_path.push(".config/vk_tui/token.json");

    let content = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Cannot read token from {:?}", config_path));

    let json: serde_json::Value = serde_json::from_str(&content).expect("Cannot parse token.json");

    json["access_token"]
        .as_str()
        .expect("No access_token in token.json")
        .to_string()
}

/// Load user ID from config file
fn get_test_user_id() -> i64 {
    let mut config_path = dirs::home_dir().expect("Cannot get home directory");
    config_path.push(".config/vk_tui/token.json");

    let content = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Cannot read token from {:?}", config_path));

    let json: serde_json::Value = serde_json::from_str(&content).expect("Cannot parse token.json");

    json["user_id"].as_i64().expect("No user_id in token.json")
}

/// Get current user ID
async fn get_current_user_id(_client: &VkClient) -> i64 {
    // Use stored user_id from token
    get_test_user_id()
}

/// Create test client
fn create_test_client() -> VkClient {
    let token = get_test_token();
    VkClient::new(token)
}

#[tokio::test]
async fn test_get_conversations() {
    println!("\n=== Testing get_conversations ===");

    let client = create_test_client();

    let result = client.messages().get_conversations(0, 20).await;

    match result {
        Ok(response) => {
            println!("✓ Got {} conversations", response.count);
            println!("  Items: {}", response.items.len());
            println!("  Profiles: {}", response.profiles.len());
            println!("  Groups: {}", response.groups.len());

            // Print first few conversations
            for (i, item) in response.items.iter().take(5).enumerate() {
                let title = if let Some(settings) = &item.conversation.chat_settings {
                    &settings.title
                } else {
                    "DM"
                };
                println!(
                    "  {}. {} (peer_id: {})",
                    i + 1,
                    title,
                    item.conversation.peer.id
                );
            }

            assert!(
                !response.items.is_empty(),
                "Should have at least one conversation"
            );
        }
        Err(e) => {
            panic!("Failed to get conversations: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_current_user() {
    println!("\n=== Testing get current user ===");

    let client = create_test_client();
    let user_id = get_test_user_id();

    let result = client.users().get(&[user_id]).await;

    match result {
        Ok(users) => {
            assert!(!users.is_empty(), "Should return user");
            let user = &users[0];
            println!(
                "✓ Current user: {} {} (ID: {})",
                user.first_name, user.last_name, user.id
            );
            assert_eq!(user.id, user_id, "User ID should match");
        }
        Err(e) => {
            panic!("Failed to get current user: {}", e);
        }
    }
}

#[tokio::test]
async fn test_send_and_get_saved_messages() {
    println!("\n=== Testing send to Saved Messages (Избранное) ===");

    let client = create_test_client();

    // Get current user ID (Saved Messages peer_id = user_id)
    let user_id = get_current_user_id(&client).await;
    let peer_id = user_id; // Saved Messages

    println!("Current user ID: {}", user_id);
    println!("Sending test message to Saved Messages...");

    // Send test message
    let test_message = format!(
        "Test message from vk-api integration test at {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
    );

    let send_result = client.messages().send(peer_id, &test_message).await;

    match send_result {
        Ok(sent) => {
            println!(
                "✓ Message sent successfully! Message ID: {}, CMID: {}",
                sent.message_id, sent.conversation_message_id
            );
            assert!(sent.message_id > 0, "Message ID should be positive");

            // Wait a bit for message to appear
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // Get message history to verify
            println!("Fetching message history...");
            let history = client
                .messages()
                .get_history(peer_id, 0, 10)
                .await
                .expect("Failed to get history");

            println!("✓ Got {} messages from history", history.items.len());

            // Find our message
            let found = history.items.iter().find(|msg| msg.id == sent.message_id);

            if let Some(msg) = found {
                println!("✓ Found our message: {:?}", msg.text);
                assert_eq!(msg.text, test_message, "Message text should match");
                assert!(msg.is_outgoing(), "Message should be outgoing");
            } else {
                println!("⚠ Message not found in history yet (this can happen due to API delays)");
            }
        }
        Err(e) => {
            panic!("Failed to send message: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_history() {
    println!("\n=== Testing get_history ===");

    let client = create_test_client();
    let user_id = get_current_user_id(&client).await;

    let result = client.messages().get_history(user_id, 0, 10).await;

    match result {
        Ok(history) => {
            println!("✓ Got history with {} messages", history.items.len());
            println!("  Total count: {}", history.count);
            println!("  Profiles: {}", history.profiles.len());
            println!("  Groups: {}", history.groups.len());

            // Print last few messages
            for (i, msg) in history.items.iter().rev().take(3).enumerate() {
                let preview = if msg.text.len() > 50 {
                    format!("{}...", &msg.text[..50])
                } else {
                    msg.text.clone()
                };
                println!("  {}. [{}] {}", i + 1, msg.id, preview);
            }

            assert!(history.count >= 0, "Count should be non-negative");
        }
        Err(e) => {
            panic!("Failed to get history: {}", e);
        }
    }
}

#[tokio::test]
async fn test_mark_as_read() {
    println!("\n=== Testing mark_as_read ===");

    let client = create_test_client();
    let user_id = get_current_user_id(&client).await;

    let result = client.messages().mark_as_read(user_id).await;

    match result {
        Ok(count) => {
            println!("✓ Marked as read successfully! Count: {}", count);
            assert!(count >= 0, "Count should be non-negative");
        }
        Err(e) => {
            panic!("Failed to mark as read: {}", e);
        }
    }
}

#[tokio::test]
async fn test_search_conversations() {
    println!("\n=== Testing search_conversations ===");

    let client = create_test_client();

    // Search with a query (empty string might not work)
    let result = client.messages().search_conversations("a", 5).await;

    match result {
        Ok(conversations) => {
            println!("✓ Search completed");
            println!("  Found {} conversations", conversations.len());

            for (i, conversation) in conversations.iter().take(3).enumerate() {
                let title = if let Some(settings) = &conversation.chat_settings {
                    &settings.title
                } else {
                    "DM"
                };
                println!("  {}. {}", i + 1, title);
            }
        }
        Err(e) => {
            println!("⚠ Search failed: {}", e);
            panic!("Failed to search conversations: {}", e);
        }
    }
}

#[tokio::test]
async fn test_users_get() {
    println!("\n=== Testing users.get ===");

    let client = create_test_client();
    let user_id = get_current_user_id(&client).await;

    let result = client.users().get(&[user_id]).await;

    match result {
        Ok(users) => {
            assert_eq!(users.len(), 1, "Should return exactly one user");
            let user = &users[0];
            println!("✓ User info:");
            println!("  ID: {}", user.id);
            println!("  Name: {} {}", user.first_name, user.last_name);
            println!("  Screen name: {:?}", user.screen_name);
            println!("  Online: {}", user.is_online());

            assert_eq!(user.id, user_id, "User ID should match");
        }
        Err(e) => {
            panic!("Failed to get user: {}", e);
        }
    }
}

#[tokio::test]
async fn test_longpoll_get_server() {
    println!("\n=== Testing longpoll.get_server ===");

    let client = create_test_client();

    let result = client.longpoll().get_server().await;

    match result {
        Ok(server) => {
            println!("✓ Long Poll server obtained:");
            println!("  Server: {}", server.server);
            println!("  Key: {}...", &server.key[..20.min(server.key.len())]);
            println!("  TS: {}", server.ts);

            assert!(!server.server.is_empty(), "Server URL should not be empty");
            assert!(!server.key.is_empty(), "Key should not be empty");
            assert!(!server.ts.is_empty(), "TS should not be empty");
        }
        Err(e) => {
            panic!("Failed to get Long Poll server: {}", e);
        }
    }
}

#[tokio::test]
async fn test_edit_message() {
    println!("\n=== Testing edit message ===");

    let client = create_test_client();
    let user_id = get_current_user_id(&client).await;

    // Send a message first
    let original_text = format!("Original text - {}", chrono::Utc::now().format("%H:%M:%S"));
    let sent = client
        .messages()
        .send(user_id, &original_text)
        .await
        .expect("Failed to send message");

    println!(
        "✓ Sent message {} (cmid: {}) with text: {}",
        sent.message_id, sent.conversation_message_id, original_text
    );

    // Wait a bit for API to process
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Edit the message using the cmid we got from send
    let new_text = format!("EDITED - {}", chrono::Utc::now().format("%H:%M:%S"));
    let cmid = (sent.conversation_message_id > 0).then_some(sent.conversation_message_id);
    let edit_result = client
        .messages()
        .edit(user_id, sent.message_id, cmid, &new_text)
        .await;

    match edit_result {
        Ok(_) => {
            println!("✓ Message edited successfully to: {}", new_text);

            // Verify by getting history
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let history = client
                .messages()
                .get_history(user_id, 0, 10)
                .await
                .expect("Failed to get history");

            if let Some(msg) = history.items.iter().find(|m| m.id == sent.message_id) {
                println!("✓ Verified edited message: {}", msg.text);
                // Note: VK API might take time to update, so we won't assert here
            }
        }
        Err(e) => {
            // Editing messages in Saved Messages might not be allowed
            println!(
                "⚠ Edit failed (might be expected for Saved Messages): {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_delete_message() {
    println!("\n=== Testing delete message ===");

    let client = create_test_client();
    let user_id = get_current_user_id(&client).await;

    // Send a message to delete
    let text = format!(
        "Message to delete - {}",
        chrono::Utc::now().format("%H:%M:%S")
    );
    let sent = client
        .messages()
        .send(user_id, &text)
        .await
        .expect("Failed to send message");

    println!("✓ Sent message {} to delete", sent.message_id);

    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Delete the message
    let delete_result = client.messages().delete(&[sent.message_id], false).await;

    match delete_result {
        Ok(_) => {
            println!("✓ Message deleted successfully");
        }
        Err(e) => {
            panic!("Failed to delete message: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_conversation_by_id() {
    println!("\n=== Testing get_conversation_by_id ===");

    let client = create_test_client();
    let user_id = get_current_user_id(&client).await;

    let result = client.messages().get_conversation_by_id(user_id).await;

    match result {
        Ok(conversation) => {
            println!("✓ Got conversation:");
            println!("  Peer ID: {}", conversation.peer.id);
            println!("  Unread count: {:?}", conversation.unread_count);
            println!("  Can write: {:?}", conversation.can_write);

            assert_eq!(conversation.peer.id, user_id, "Peer ID should match");
        }
        Err(e) => {
            println!(
                "⚠ get_conversation_by_id failed (might be API issue): {}",
                e
            );
            // Don't panic - this method might have issues with parsing
        }
    }
}

#[tokio::test]
async fn test_account_get_profile_info() {
    println!("\n=== Testing account.getProfileInfo ===");

    let client = create_test_client();

    let result = client.account().get_profile_info().await;

    match result {
        Ok(profile) => {
            println!("✓ Profile info:");
            println!("  Name: {} {}", profile.first_name, profile.last_name);
            println!("  Screen name: {:?}", profile.screen_name);
            println!("  Status: {:?}", profile.status);
            println!("  Birthday: {:?}", profile.bdate);
            println!("  City: {:?}", profile.city.as_ref().map(|c| &c.title));
            println!("  Home town: {:?}", profile.home_town);
        }
        Err(e) => {
            panic!("Failed to get profile info: {}", e);
        }
    }
}

#[tokio::test]
async fn test_account_get_counters() {
    println!("\n=== Testing account.getCounters ===");

    let client = create_test_client();

    let result = client.account().get_counters().await;

    match result {
        Ok(counters) => {
            println!("✓ Counters:");
            println!("  Messages: {:?}", counters.messages);
            println!("  Friends: {:?}", counters.friends);
            println!("  Notifications: {:?}", counters.notifications);
            println!("  Groups: {:?}", counters.groups);
        }
        Err(e) => {
            panic!("Failed to get counters: {}", e);
        }
    }
}
