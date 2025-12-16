use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, MouseEvent};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;

/// Terminal events
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Event {
    /// Terminal tick (for periodic updates)
    Tick,
    /// Key press
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// VK event (new message, etc.)
    Vk(VkEvent),
}

/// Events from VK API
#[derive(Debug, Clone)]
pub enum VkEvent {
    /// New message received
    NewMessage {
        peer_id: i64,
        text: String,
        from_id: i64,
    },
    /// Message read
    MessageRead { peer_id: i64, message_id: i64 },
    /// User typing
    UserTyping { peer_id: i64, user_id: i64 },
    /// Connection status changed
    ConnectionStatus(bool),
}

/// Event handler that polls for terminal and VK events
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    /// Create new event handler with specified tick rate
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();

        // Spawn terminal event polling task
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                let tick_delay = tick_interval.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = tick_delay => {
                        if tx_clone.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    Some(Ok(evt)) = crossterm_event => {
                        let event = match evt {
                            CrosstermEvent::Key(key) => Event::Key(key),
                            CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
                            CrosstermEvent::Resize(w, h) => Event::Resize(w, h),
                            _ => continue,
                        };
                        if tx_clone.send(event).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    /// Get the sender for VK events
    #[allow(dead_code)]
    pub fn vk_sender(&self) -> mpsc::UnboundedSender<Event> {
        self._tx.clone()
    }

    /// Receive next event
    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}
