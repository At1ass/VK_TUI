//! Application state management.

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use vk_api::{VkClient, auth::AuthManager};
use vk_core::{AsyncCommand, CommandExecutor, CoreEvent};

/// Global application state shared across Tauri.
pub struct AppState {
    pub auth: Arc<Mutex<AuthManager>>,
    pub vk_client: Arc<Mutex<Option<Arc<VkClient>>>>,
    pub command_tx: Arc<Mutex<Option<mpsc::UnboundedSender<AsyncCommand>>>>,
    pub event_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<CoreEvent>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            auth: Arc::new(Mutex::new(AuthManager::default())),
            vk_client: Arc::new(Mutex::new(None)),
            command_tx: Arc::new(Mutex::new(None)),
            event_rx: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize VK client and executor.
    pub async fn initialize_session(&self, token: String) -> Result<(), String> {
        let client = Arc::new(VkClient::new(token));

        // Validate session
        client
            .account()
            .get_profile_info()
            .await
            .map_err(|e| format!("Session validation failed: {}", e))?;

        // Create command/event channels
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AsyncCommand>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<CoreEvent>();

        // Store in state
        *self.vk_client.lock().await = Some(client.clone());
        *self.command_tx.lock().await = Some(cmd_tx);
        *self.event_rx.lock().await = Some(event_rx);

        // Spawn command executor
        let executor = CommandExecutor::new(client.clone(), event_tx.clone());
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                executor.execute(cmd).await;
            }
        });

        // Spawn LongPoll
        tokio::spawn(async move {
            Self::run_long_poll(client, event_tx).await;
        });

        Ok(())
    }

    /// Run VK LongPoll listener.
    async fn run_long_poll(client: Arc<VkClient>, event_tx: mpsc::UnboundedSender<CoreEvent>) {
        tracing::info!("Starting LongPoll...");
        let mut backoff = std::time::Duration::from_secs(1);

        let mut server = match client.longpoll().get_server().await {
            Ok(s) => {
                tracing::info!("Got LongPoll server: {}", s.server);
                let _ = event_tx.send(CoreEvent::VkEvent(vk_core::VkEvent::ConnectionStatus(true)));
                s
            }
            Err(e) => {
                let _ = event_tx.send(CoreEvent::Error(format!("LongPoll error: {}", e)));
                return;
            }
        };

        loop {
            match client.longpoll().poll(&server).await {
                Ok(response) => {
                    if let Some(failed) = response.failed {
                        match failed {
                            1 => {
                                if let Some(ts) = response.ts {
                                    server.ts = ts;
                                }
                            }
                            2..=4 => match client.longpoll().get_server().await {
                                Ok(new_server) => server = new_server,
                                Err(e) => {
                                    let _ = event_tx.send(CoreEvent::Error(format!("LongPoll error: {}", e)));
                                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                }
                            },
                            _ => {}
                        }
                        continue;
                    }

                    if let Some(ts) = response.ts {
                        server.ts = ts;
                    }

                    if let Some(updates) = response.updates {
                        for update in updates {
                            if let Some(event) = vk_core::longpoll::handle_update(&update) {
                                let _ = event_tx.send(CoreEvent::VkEvent(event));
                            }
                        }
                    }
                    backoff = std::time::Duration::from_secs(1);
                }
                Err(e) => {
                    let _ = event_tx.send(CoreEvent::VkEvent(vk_core::VkEvent::ConnectionStatus(false)));
                    let _ = event_tx.send(CoreEvent::Error(format!("LongPoll error: {}", e)));
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(std::time::Duration::from_secs(30));

                    match client.longpoll().get_server().await {
                        Ok(new_server) => {
                            server = new_server;
                            let _ = event_tx.send(CoreEvent::VkEvent(vk_core::VkEvent::ConnectionStatus(true)));
                            backoff = std::time::Duration::from_secs(1);
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
