//! Parser for command mode (colon-commands).
use crate::state::{
    App, AsyncAction, AttachmentInfo, CommandSuggestion, CompletionState, Focus, PathEntry,
    SubcommandOption,
};

pub fn handle_command(app: &mut App, cmd: &str) -> Option<crate::message::Message> {
    // Remove leading ':' if present
    let cmd = cmd.trim_start_matches(':');
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

/// Generate command suggestions based on input
pub fn generate_suggestions(input: &str) -> Vec<CommandSuggestion> {
    let all_commands = vec![
        CommandSuggestion {
            command: "quit".to_string(),
            description: "Quit application".to_string(),
            usage: Some(":q, :quit, :qa, :quitall".to_string()),
        },
        CommandSuggestion {
            command: "back".to_string(),
            description: "Return to chat list".to_string(),
            usage: Some(":b, :back".to_string()),
        },
        CommandSuggestion {
            command: "search".to_string(),
            description: "Search conversations".to_string(),
            usage: Some(":search <query>, :s <query>".to_string()),
        },
        CommandSuggestion {
            command: "msg".to_string(),
            description: "Quick send message".to_string(),
            usage: Some(":msg <text>, :m <text>".to_string()),
        },
        CommandSuggestion {
            command: "attach photo".to_string(),
            description: "Attach photo from file".to_string(),
            usage: Some(":attach photo <path>, :ap <path>".to_string()),
        },
        CommandSuggestion {
            command: "attach doc".to_string(),
            description: "Attach document".to_string(),
            usage: Some(":attach doc <path>, :ad <path>".to_string()),
        },
        CommandSuggestion {
            command: "download".to_string(),
            description: "Download attachments from selected message".to_string(),
            usage: Some(":download, :dl".to_string()),
        },
        CommandSuggestion {
            command: "help".to_string(),
            description: "Show help popup".to_string(),
            usage: Some(":help, :h".to_string()),
        },
        CommandSuggestion {
            command: "reply".to_string(),
            description: "Reply to selected message".to_string(),
            usage: Some(":reply, :r".to_string()),
        },
        CommandSuggestion {
            command: "forward".to_string(),
            description: "Forward selected message".to_string(),
            usage: Some(":forward, :f, :fwd".to_string()),
        },
        CommandSuggestion {
            command: "pin".to_string(),
            description: "Pin/unpin selected message".to_string(),
            usage: Some(":pin, :p".to_string()),
        },
    ];

    // If input is empty, return all commands
    if input.trim().is_empty() {
        return all_commands;
    }

    // Filter commands that match the input (search by start of any word in command name)
    let input_lower = input.to_lowercase();
    all_commands
        .into_iter()
        .filter(|cmd| {
            let cmd_lower = cmd.command.to_lowercase();
            // Match if command starts with input OR any word in command starts with input
            cmd_lower.starts_with(&input_lower)
                || cmd_lower
                    .split_whitespace()
                    .any(|word| word.starts_with(&input_lower))
        })
        .collect()
}

/// Generate subcommand completions for a given command
fn generate_subcommand_completions(command: &str, input: &str) -> CompletionState {
    let options = match command {
        "attach" => vec![
            SubcommandOption {
                name: "photo".to_string(),
                description: "Attach photo from file".to_string(),
            },
            SubcommandOption {
                name: "doc".to_string(),
                description: "Attach document".to_string(),
            },
        ],
        _ => vec![],
    };

    // Filter by input
    let input_lower = input.to_lowercase();
    let filtered: Vec<_> = options
        .into_iter()
        .filter(|opt| opt.name.to_lowercase().starts_with(&input_lower))
        .collect();

    if !filtered.is_empty() {
        CompletionState::Subcommands {
            command: command.to_string(),
            options: filtered,
            selected: 0,
        }
    } else {
        CompletionState::Inactive
    }
}

/// Generate file path completions
fn generate_filepath_completions(input: &str, base: &str) -> CompletionState {
    use std::fs;
    use std::path::{Path, PathBuf};

    // Expand ~ to home directory
    let expanded_input = if input.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            input.replacen('~', &home, 1)
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    };

    // Determine directory to list and filename prefix
    let (dir_to_list, prefix) = if expanded_input.is_empty() {
        (PathBuf::from(base), String::new())
    } else {
        let path = Path::new(&expanded_input);

        // If path exists and is a directory with trailing slash, list its contents
        if path.is_dir() && (expanded_input.ends_with('/') || expanded_input.ends_with('\\')) {
            (path.to_path_buf(), String::new())
        } else {
            // Otherwise, list parent directory and filter by filename
            let parent = path.parent().unwrap_or(Path::new(base));
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            (parent.to_path_buf(), file_name)
        }
    };

    // Read directory entries
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(&dir_to_list) {
        for entry in read_dir.flatten() {
            if let (Ok(name), Ok(metadata)) = (entry.file_name().into_string(), entry.metadata()) {
                // Skip hidden files (starting with .)
                if name.starts_with('.') && !prefix.starts_with('.') {
                    continue;
                }

                // Filter by prefix
                if prefix.is_empty() || name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    entries.push(PathEntry {
                        name: name.clone(),
                        full_path: entry.path().display().to_string(),
                        is_dir: metadata.is_dir(),
                    });
                }
            }
        }
    }

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    if !entries.is_empty() {
        CompletionState::FilePaths {
            context: "attach".to_string(),
            base_path: dir_to_list.display().to_string(),
            entries,
            selected: 0,
        }
    } else {
        CompletionState::Inactive
    }
}

/// Determine completion state based on input
/// This is the FSM transition logic with context-aware parsing
pub fn determine_completion_state(input: &str) -> CompletionState {
    // Remove leading ':' if present
    let trimmed = input.trim_start_matches(':');
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let ends_with_space = input.ends_with(' ');

    // FSM state transitions based on command context
    match (parts.as_slice(), ends_with_space) {
        // Stage 1: Command name completion
        // Examples: ":att" or ":attach"
        ([], _) | ([_], false) => {
            let cmd = parts.first().copied().unwrap_or("");
            let suggestions = generate_suggestions(cmd);

            // If input exactly matches a command, don't show completion
            let input_lower = cmd.trim().to_lowercase();
            if suggestions.len() == 1 && suggestions[0].command.to_lowercase() == input_lower {
                return CompletionState::Inactive;
            }

            if !suggestions.is_empty() {
                CompletionState::Commands {
                    suggestions,
                    selected: 0,
                }
            } else {
                CompletionState::Inactive
            }
        }

        // Stage 2: Subcommand completion for "attach"
        // Examples: ":attach " or ":attach ph"
        (["attach"], true) => generate_subcommand_completions("attach", ""),
        (["attach", sub], false) => generate_subcommand_completions("attach", sub),

        // Stage 3: File path completion for "attach photo|doc"
        // Examples: ":attach photo " or ":attach photo /home/user/fi"
        (["attach", "photo" | "doc"], true) => generate_filepath_completions("", "."),
        (["attach", "photo" | "doc", path @ ..], _) => {
            let path_str = path.join(" ");
            generate_filepath_completions(&path_str, ".")
        }

        // Future extensions:
        // (["search"], true) => generate_search_scope_completions(),
        // (["forward"], true) => generate_chat_completions(),

        // Default: no completion
        _ => CompletionState::Inactive,
    }
}
