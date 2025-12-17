//! Parser for command mode (colon-commands).
use crate::state::{App, AsyncAction, AttachmentInfo, Focus};

pub fn handle_command(app: &mut App, cmd: &str) -> Option<crate::message::Message> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        "q" | "quit" | "qa" | "quitall" => {
            app.running_state = crate::state::RunningState::Done;
        }
        "b" | "back" => {
            app.focus = Focus::ChatList;
            app.current_peer_id = None;
        }
        "s" | "search" => {
            if parts.len() > 1 {
                let query = parts[1..].join(" ");
                app.status = Some(format!("Search: {} (not yet implemented)", query));
            } else {
                app.status = Some("Usage: :search <query>".into());
            }
        }
        "m" | "msg" => {
            if parts.len() > 1 {
                let text = parts[1..].join(" ");
                if let Some(peer_id) = app.current_peer_id {
                    app.send_action(AsyncAction::SendMessage(peer_id, text));
                } else {
                    app.status = Some("No chat selected".into());
                }
            } else {
                app.status = Some("Usage: :msg <text>".into());
            }
        }
        "ap" | "attach" => {
            if parts.len() > 2 && parts[1] == "photo" {
                let path = parts[2..].join(" ");
                if let Some(peer_id) = app.current_peer_id {
                    app.send_action(AsyncAction::SendPhoto(peer_id, path));
                } else {
                    app.status = Some("No chat selected".into());
                }
            } else if parts.len() > 2 && parts[1] == "doc" {
                let path = parts[2..].join(" ");
                if let Some(peer_id) = app.current_peer_id {
                    app.send_action(AsyncAction::SendDoc(peer_id, path));
                } else {
                    app.status = Some("No chat selected".into());
                }
            } else {
                app.status = Some("Usage: :attach photo|doc <path>".into());
            }
        }
        "dl" | "download" => {
            if let Some(msg) = app.current_message() {
                let downloadable: Vec<AttachmentInfo> = msg
                    .attachments
                    .iter()
                    .filter(|a| a.url.is_some())
                    .cloned()
                    .collect();
                if downloadable.is_empty() {
                    app.status = Some("No downloadable attachments".into());
                } else {
                    app.send_action(AsyncAction::DownloadAttachments(downloadable));
                    app.status = Some("Downloading attachments...".into());
                }
            }
        }
        "h" | "help" => {
            app.show_help = true;
        }
        "r" | "reply" => {
            app.status = Some("Reply not yet implemented".into());
        }
        "f" | "forward" | "fwd" => {
            app.status = Some("Forward not yet implemented".into());
        }
        "p" | "pin" => {
            app.status = Some("Pin/unpin not yet implemented".into());
        }
        _ => {
            app.status = Some(format!("Unknown command: {}", parts[0]));
        }
    }

    None
}
