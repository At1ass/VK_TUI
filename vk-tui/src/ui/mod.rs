use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Position,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::state::{App, AttachmentKind, DeliveryStatus, Focus, ForwardStage, Mode, Screen};

/// Main view function - renders the entire UI
pub fn view(app: &App, frame: &mut Frame) {
    match app.screen {
        Screen::Auth => render_auth_screen(app, frame),
        Screen::Main => render_main_screen(app, frame),
    }

    // Forward popup on top
    if app.forward.is_some() {
        render_forward_popup(app, frame);
    }

    // Forwarded-view popup on top
    if app.forward_view.is_some() {
        render_forward_view_popup(app, frame);
    }

    // Render help popup on top if visible
    if app.show_help {
        render_help_popup(app, frame);
    }

    // Render command completion popup on top if visible
    if !matches!(app.completion_state, crate::state::CompletionState::Inactive)
        && app.mode == Mode::Command
    {
        render_command_completion(app, frame);
    }

    // Render global search popup on top if visible
    if app.global_search.is_some() {
        render_global_search_popup(app, frame);
    }
}

/// Render authentication screen
fn render_auth_screen(app: &App, frame: &mut Frame) {
    let area = frame.area();

    // Center the auth dialog
    let dialog_width = 80.min(area.width.saturating_sub(4));
    let dialog_height = 16.min(area.height.saturating_sub(4));

    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" VK TUI - Authorization ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Instructions
            Constraint::Length(2), // URL
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Input label
            Constraint::Length(3), // Input field
            Constraint::Min(1),    // Status
        ])
        .split(inner);

    // Instructions
    let instructions = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("1. Press "),
            Span::styled(
                "Ctrl+O",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to open the auth URL in browser"),
        ]),
        Line::from("2. Authorize the application"),
        Line::from("3. Copy the redirect URL and paste it below"),
    ])
    .style(Style::default().fg(Color::White));
    frame.render_widget(instructions, chunks[0]);

    // Auth URL (truncated if needed)
    let auth_url = app.auth_url();
    let url_display = if auth_url.len() > chunks[1].width as usize - 2 {
        format!("{}...", &auth_url[..chunks[1].width as usize - 5])
    } else {
        auth_url
    };
    let url = Paragraph::new(url_display)
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: false });
    frame.render_widget(url, chunks[1]);

    // Input label
    let label = Paragraph::new("Paste redirect URL here and press Enter:")
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(label, chunks[3]);

    // Input field
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let input = Paragraph::new(app.token_input.as_str())
        .block(input_block)
        .style(Style::default().fg(Color::White));
    frame.render_widget(input, chunks[4]);

    // Show cursor - calculate visual width for UTF-8
    let cursor_x = visual_width(&app.token_input, app.token_cursor);
    frame.set_cursor_position((chunks[4].x + cursor_x as u16 + 1, chunks[4].y + 1));

    // Status
    if let Some(status) = &app.status {
        let status_style = if status.contains("Error") {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        let status_text = Paragraph::new(status.as_str())
            .style(status_style)
            .alignment(Alignment::Center);
        frame.render_widget(status_text, chunks[5]);
    }
}

/// Render main chat screen
fn render_main_screen(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.area());

    render_chat_list(app, frame, chunks[0]);
    render_chat_area(app, frame, chunks[1]);
}

/// Render the chat list panel
fn render_chat_list(app: &App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus == Focus::ChatList;

    // Determine which chats to show (filtered or all)
    let visible_chats: Vec<&crate::state::Chat> = if let Some(filter) = &app.chat_filter {
        filter
            .filtered_indices
            .iter()
            .filter_map(|&idx| app.chats.get(idx))
            .collect()
    } else {
        app.chats.iter().collect()
    };

    let items: Vec<ListItem> = visible_chats
        .iter()
        .map(|chat| {
            let unread = if chat.unread_count > 0 {
                format!(" ({})", chat.unread_count)
            } else {
                String::new()
            };

            let online_indicator = if chat.is_online { "●" } else { "○" };

            let line = Line::from(vec![
                Span::styled(
                    online_indicator,
                    Style::default().fg(if chat.is_online {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::raw(" "),
                Span::styled(
                    &chat.title,
                    Style::default().add_modifier(if chat.unread_count > 0 {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ),
                Span::styled(unread, Style::default().fg(Color::Cyan)),
            ]);

            let preview = Line::from(vec![Span::styled(
                truncate_str(&chat.last_message, area.width.saturating_sub(4) as usize),
                Style::default().fg(Color::DarkGray),
            )]);

            ListItem::new(vec![line, preview])
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if app.is_loading {
        " Chats (loading...) "
    } else {
        " Chats "
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_chat));

    frame.render_stateful_widget(list, area, &mut state);

    // Render filter input if active
    if let Some(filter) = &app.chat_filter {
        let filter_area = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(3),
            width: area.width.saturating_sub(2),
            height: 3,
        };

        let filter_text = format!("🔍 {}", filter.query);
        let filter_widget = Paragraph::new(filter_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Filter "),
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(Clear, filter_area);
        frame.render_widget(filter_widget, filter_area);

        // Render cursor
        let cursor_x = filter_area.x + 3 + filter.cursor as u16; // +3 for "🔍 "
        let cursor_y = filter_area.y + 1;
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}

/// Render the chat area (messages + input)
fn render_chat_area(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Messages
            Constraint::Length(3), // Input
            Constraint::Length(1), // Status
        ])
        .split(area);

    render_messages(app, frame, chunks[0]);
    render_input(app, frame, chunks[1]);
    render_status(app, frame, chunks[2]);
}

/// Render messages panel
fn render_messages(app: &App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus == Focus::Messages;

    let render_lines = |msg: &crate::state::ChatMessage| -> Vec<Line<'static>> {
        let name_style = if msg.is_outgoing {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let read_indicator = match msg.delivery {
            DeliveryStatus::Pending => "...",
            DeliveryStatus::Failed => "!",
            DeliveryStatus::Sent => {
                if msg.is_outgoing {
                    if msg.is_read { "✓✓" } else { "✓" }
                } else {
                    ""
                }
            }
        };

        // Format timestamp
        let time = format_timestamp(msg.timestamp);

        let mut first_line = vec![
            Span::styled(time, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            if msg.is_pinned {
                Span::styled("📌 ", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
            Span::styled(msg.from_name.clone(), name_style),
            Span::raw(": "),
            Span::raw(msg.text.clone()),
        ];

        // Add edited indicator
        if msg.is_edited {
            first_line.push(Span::styled(" (e)", Style::default().fg(Color::Yellow)));
        }

        // Add delivery status indicator
        if !read_indicator.is_empty() {
            first_line.push(Span::styled(
                format!(" {}", read_indicator),
                Style::default().fg(Color::DarkGray),
            ));
        }

        let mut lines = vec![Line::from(first_line)];

        if let Some(reply) = &msg.reply {
            lines.insert(
                0,
                Line::from(vec![
                    Span::styled("↩ ", Style::default().fg(Color::Gray)),
                    Span::styled(reply.from.clone(), Style::default().fg(Color::Gray)),
                    Span::raw(": "),
                    Span::styled(
                        truncate_str(&reply.text, 60),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
            );
        }

        if msg.fwd_count > 0 {
            lines.push(Line::from(vec![Span::styled(
                format!("↪ forwarded {}", msg.fwd_count),
                Style::default().fg(Color::Gray),
            )]));
        }

        for att in &msg.attachments {
            let label = match &att.kind {
                AttachmentKind::Photo => "[photo]".to_string(),
                AttachmentKind::Doc => "[file]".to_string(),
                AttachmentKind::Link => "[link]".to_string(),
                AttachmentKind::Audio => "[audio]".to_string(),
                AttachmentKind::Sticker => "[sticker]".to_string(),
                AttachmentKind::Other(k) => format!("[{}]", k),
            };
            let mut detail = format!("{} {}", label, att.title);
            if let Some(sub) = &att.subtitle {
                detail.push_str(&format!(" — {}", sub));
            }
            if let Some(size) = att.size {
                let kb = size as f64 / 1024.0;
                detail.push_str(&format!(" ({:.1} KB)", kb));
            }
            if let Some(url) = &att.url {
                detail.push(' ');
                detail.push_str(url);
            }
            lines.push(Line::from(Span::styled(
                detail,
                Style::default().fg(Color::Gray),
            )));
        }

        lines
    };

    // Reserve top area for pinned message if available
    let pinned_message = app.messages.iter().find(|m| m.is_pinned);
    let (pinned_area, list_area) = if pinned_message.is_some() {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(1)])
            .split(area);
        (Some(layout[0]), layout[1])
    } else {
        (None, area)
    };

    if let (Some(msg), Some(p_area)) = (pinned_message, pinned_area) {
        let height = render_lines(msg).len() as u16 + 2;
        let adj_height = height.min(p_area.height);
        let pin_block = Block::default()
            .title(" Pinned ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let inner_height = adj_height.saturating_sub(2).max(1);
        let inner_area = Rect::new(p_area.x, p_area.y, p_area.width, adj_height);
        frame.render_widget(pin_block, inner_area);
        let content = Paragraph::new(render_lines(msg))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false });
        frame.render_widget(
            content,
            Rect::new(
                inner_area.x + 1,
                inner_area.y + 1,
                inner_area.width.saturating_sub(2),
                inner_height,
            ),
        );
    }

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|msg| ListItem::new(render_lines(msg)))
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let chat_title = app
        .current_peer_id
        .and_then(|peer_id| app.chats.iter().find(|c| c.id == peer_id))
        .map(|c| c.title.as_str())
        .unwrap_or("Messages");

    let title = if app.is_loading && app.current_peer_id.is_some() {
        format!(" {} (loading...) ", chat_title)
    } else {
        format!(" {} ", chat_title)
    };

    let list = List::new(messages)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = ListState::default();
    state.select(Some(app.messages_scroll));

    frame.render_stateful_widget(list, list_area, &mut state);
}

/// Render input field
fn render_input(app: &App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus == Focus::Input;

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input = Paragraph::new(app.input.as_str())
        .block(
            Block::default()
                .title(" Message (Enter to send) ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(input, area);

    // Show cursor when focused - calculate visual width for UTF-8
    if is_focused {
        let cursor_x = visual_width(&app.input, app.input_cursor);
        frame.set_cursor_position((area.x + cursor_x as u16 + 1, area.y + 1));
    }
}

/// Calculate visual width of string up to char_pos
fn visual_width(s: &str, char_pos: usize) -> usize {
    use unicode_width::UnicodeWidthChar;
    s.chars()
        .take(char_pos)
        .map(|c| c.width().unwrap_or(0))
        .sum()
}

/// Render status bar
fn render_status(app: &App, frame: &mut Frame, area: Rect) {
    // In Command mode, show command prompt
    if app.mode == Mode::Command {
        let cmd_text = format!(":{}", app.command_input);
        let cmd_prompt = Paragraph::new(cmd_text).style(Style::default().fg(Color::Yellow));
        frame.render_widget(cmd_prompt, area);

        // Show cursor at command position
        let cursor_x = visual_width(&app.command_input, app.command_cursor);
        frame.set_cursor_position((area.x + cursor_x as u16 + 1, area.y)); // +1 for ':'
        return;
    }

    // Otherwise show status or help hints
    let default_help = match (app.mode, app.focus) {
        (Mode::Insert, _) => "Enter send | /sendfile PATH | /sendimg PATH | Esc back",
        (Mode::Normal, Focus::ChatList) => "j/k nav | l/Enter select | / search | : cmd | ? help",
        (Mode::Normal, Focus::Messages) => {
            "j/k nav | i insert | r reply | f forward | F view fwds | e edit | dd delete | p pin | o link | ? help"
        }
        (Mode::Normal, Focus::Input) => "i insert mode | Esc back",
        (Mode::Command, _) => "Enter execute | Esc cancel",
    };
    let status_text = app.status.as_deref().unwrap_or(default_help);

    let style = if app.status.as_ref().is_some_and(|s| s.contains("Error")) {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let help = Paragraph::new(status_text).style(style);

    frame.render_widget(help, area);
}

/// Truncate string to max length with ellipsis
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!(
            "{}...",
            s.chars()
                .take(max_len.saturating_sub(3))
                .collect::<String>()
        )
    }
}

/// Format unix timestamp to HH:MM
fn format_timestamp(ts: i64) -> String {
    use time::macros::format_description;
    use time::{Duration, OffsetDateTime};

    let now = OffsetDateTime::now_utc();
    let dt = OffsetDateTime::from_unix_timestamp(ts).unwrap_or(OffsetDateTime::UNIX_EPOCH);

    if (now - dt) < Duration::days(1) {
        dt.format(&format_description!("[hour]:[minute]"))
            .unwrap_or_else(|_| "--:--".into())
    } else {
        dt.format(&format_description!("[day].[month].[year]"))
            .unwrap_or_else(|_| "--.--.----".into())
    }
}

/// Create a centered rectangle
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

fn render_forward_popup(app: &App, frame: &mut Frame) {
    let Some(fwd) = &app.forward else {
        return;
    };

    let area = frame.area();
    let width = (area.width as f32 * 0.7).max(40.0).min(100.0) as u16;
    let height = (area.height as f32 * 0.7).max(12.0).min(30.0) as u16;
    let popup_area = centered_rect(width, height, area);

    frame.render_widget(Clear, popup_area);

    match &fwd.stage {
        ForwardStage::SelectTarget => {
            let block = Block::default()
                .title(" Forward to... ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            let inner = block.inner(popup_area);
            frame.render_widget(block, popup_area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Min(3),
                    Constraint::Length(1),
                ])
                .split(inner);

            let query = Paragraph::new(format!("Search: {}", fwd.query))
                .style(Style::default().fg(Color::White));
            frame.render_widget(query, chunks[0]);

            let hint =
                Paragraph::new("Type to search, j/k to move, Enter to select, Esc to cancel")
                    .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(hint, chunks[1]);

            let items: Vec<ListItem> = fwd
                .filtered
                .iter()
                .map(|chat| {
                    let unread = if chat.unread_count > 0 {
                        format!(" ({})", chat.unread_count)
                    } else {
                        String::new()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(&chat.title, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(unread),
                        Span::styled(
                            format!("  [{}]", chat.id),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Conversations "),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            let mut state = ListState::default();
            if !fwd.filtered.is_empty() {
                state.select(Some(fwd.selected));
            }

            frame.render_stateful_widget(list, chunks[2], &mut state);
        }
        ForwardStage::EnterComment { title, .. } => {
            let block = Block::default()
                .title(format!(" Forward to {} ", title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            let inner = block.inner(popup_area);
            frame.render_widget(block, popup_area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Min(2),
                    Constraint::Length(1),
                ])
                .split(inner);

            let info = Paragraph::new("Enter comment (optional), Enter to send, Esc to cancel")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(info, chunks[0]);

            let input_block = Block::default().borders(Borders::ALL);
            let input = Paragraph::new(fwd.comment.as_str())
                .block(input_block)
                .wrap(Wrap { trim: false });
            frame.render_widget(input, chunks[1]);

            let cursor_x = visual_width(&fwd.comment, fwd.comment.chars().count());
            frame.set_cursor_position((chunks[1].x + cursor_x as u16 + 1, chunks[1].y + 1));
        }
    }
}

fn render_forward_view_popup(app: &App, frame: &mut Frame) {
    let Some(view) = &app.forward_view else {
        return;
    };

    let area = frame.area();
    let width = (area.width as f32 * 0.7).max(50.0).min(110.0) as u16;
    let height = (area.height as f32 * 0.7).max(12.0).min(30.0) as u16;
    let popup_area = centered_rect(width, height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Forwarded messages (Esc to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if view.items.is_empty() {
        let empty = Paragraph::new("No forwarded messages")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(empty, inner);
        return;
    }

    let flattened = super::update::flatten_forwards(&view.items, 0);
    let items: Vec<ListItem> = flattened
        .iter()
        .map(|(indent, text)| {
            let pad = "  ".repeat(*indent);
            ListItem::new(Line::from(vec![Span::raw(format!("{}{}", pad, text))]))
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    state.select(Some(view.selected));
    frame.render_stateful_widget(list, inner, &mut state);
}

/// Render help popup
fn render_help_popup(app: &App, frame: &mut Frame) {
    let area = frame.area();

    // Create popup area (80% width, 80% height)
    let width = (area.width as f32 * 0.8).min(100.0) as u16;
    let height = (area.height as f32 * 0.8).min(40.0) as u16;
    let popup_area = centered_rect(width, height, area);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help (Esc or q to close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Help content based on current focus
    let help_text = match app.focus {
        Focus::ChatList => vec![
            Line::from(Span::styled(
                "Chat List Navigation",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("j, Down          - Move down"),
            Line::from("k, Up            - Move up"),
            Line::from("g                - Go to first chat"),
            Line::from("G                - Go to last chat"),
            Line::from("l, Enter         - Open selected chat"),
            Line::from("/                - Search conversations"),
            Line::from("h                - Switch to left panel"),
            Line::from("Tab              - Next panel"),
            Line::from(""),
            Line::from(Span::styled("Commands", Style::default().fg(Color::Yellow))),
            Line::from(""),
            Line::from(":                - Enter command mode"),
            Line::from("?                - Toggle this help"),
            Line::from("Ctrl+Q, Ctrl+C   - Quit application"),
        ],
        Focus::Messages => vec![
            Line::from(Span::styled(
                "Messages Navigation",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("j, Down          - Scroll down"),
            Line::from("k, Up            - Scroll up"),
            Line::from("g                - Go to first message"),
            Line::from("G                - Go to last message"),
            Line::from("Ctrl+U           - Page up"),
            Line::from("Ctrl+D           - Page down"),
            Line::from(""),
            Line::from(Span::styled("Actions", Style::default().fg(Color::Yellow))),
            Line::from(""),
            Line::from("i, l, Enter      - Enter insert mode (write message)"),
            Line::from("r                - Reply to message"),
            Line::from("f                - Forward message"),
            Line::from("F                - View forwarded (popup)"),
            Line::from("e                - Edit message"),
            Line::from("dd               - Delete message"),
            Line::from("yy               - Copy message text"),
            Line::from("p                - Pin/unpin message (coming soon)"),
            Line::from("o, Ctrl+L        - Open link in message"),
            Line::from("a                - Download attachments"),
            Line::from("/                - Search in chat (coming soon)"),
            Line::from("h, Esc           - Back to chat list"),
        ],
        Focus::Input => vec![
            Line::from(Span::styled(
                "Insert Mode",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Type normally to write message"),
            Line::from("Enter            - Send message"),
            Line::from("Esc              - Exit to normal mode"),
            Line::from("Ctrl+W           - Delete word"),
            Line::from("Ctrl+U           - Clear line"),
            Line::from("Backspace        - Delete character"),
            Line::from(""),
            Line::from(Span::styled(
                "Special Commands",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from("/sendfile <path> - Send file attachment"),
            Line::from("/sendimg <path>  - Send image"),
            Line::from("/sendimg --clipboard - Send from clipboard"),
        ],
    };

    // Add command mode help if not shown above
    let mut all_lines = help_text;
    all_lines.push(Line::from(""));
    all_lines.push(Line::from(Span::styled(
        "Command Mode (:)",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    all_lines.push(Line::from(""));
    all_lines.push(Line::from(":q, :quit        - Quit application"));
    all_lines.push(Line::from(":back, :b        - Return to chat list"));
    all_lines.push(Line::from(":search <q>, :s  - Search conversations"));
    all_lines.push(Line::from(":msg <text>, :m  - Quick send message"));
    all_lines.push(Line::from(":attach photo <path>, :ap - Send photo"));
    all_lines.push(Line::from(":attach doc <path>, :ad   - Send document"));
    all_lines.push(Line::from(":help, :h        - Show this help"));

    let paragraph = Paragraph::new(all_lines)
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

    frame.render_widget(paragraph, inner);
}

/// Render command completion popup
fn render_command_completion(app: &App, frame: &mut Frame) {
    use crate::state::CompletionState;

    // FSM pattern matching on state
    match &app.completion_state {
        CompletionState::Inactive => return,
        CompletionState::Commands { suggestions, selected } => {
            render_command_suggestions(suggestions, *selected, frame);
        }
        CompletionState::Subcommands { options, selected, .. } => {
            render_subcommand_suggestions(options, *selected, frame);
        }
        CompletionState::FilePaths { entries, selected, .. } => {
            render_filepath_suggestions(entries, *selected, frame);
        }
    }
}

/// Render command suggestions list
fn render_command_suggestions(
    suggestions: &[crate::state::CommandSuggestion],
    selected: usize,
    frame: &mut Frame,
) {
    let area = frame.area();

    // Calculate popup size
    let max_cmd_len = suggestions
        .iter()
        .map(|s| s.command.len())
        .max()
        .unwrap_or(10);
    let max_desc_len = suggestions
        .iter()
        .map(|s| s.description.len())
        .max()
        .unwrap_or(20);

    // Width: command + " - " + description + borders + padding
    let width = (max_cmd_len + 3 + max_desc_len + 4).min(80) as u16;

    // Height: each suggestion takes 2 lines (command + usage), plus borders
    let content_lines = suggestions.len() as u16 * 2; // 2 lines per item
    let height = (content_lines + 2).min(24); // +2 for borders, max 24 lines

    // Position: bottom-left, above status line
    let popup_area = Rect {
        x: area.x + 2,
        y: area.height.saturating_sub(height + 2), // Leave space for status line
        width,
        height,
    };

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Build list items
    let items: Vec<ListItem> = suggestions
        .iter()
        .map(|sug| {
            let spans = vec![
                Span::styled(
                    &sug.command,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" - "),
                Span::styled(&sug.description, Style::default().fg(Color::White)),
            ];

            // Add usage hint if available (on second line)
            let mut lines = vec![Line::from(spans)];
            if let Some(usage) = &sug.usage {
                lines.push(Line::from(vec![Span::styled(
                    format!("  {}", usage),
                    Style::default().fg(Color::DarkGray),
                )]));
            }

            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Commands (Tab/↓↑ to navigate, Enter to select, Esc to cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(selected));

    frame.render_stateful_widget(list, popup_area, &mut state);
}

/// Render subcommand suggestions list
fn render_subcommand_suggestions(
    options: &[crate::state::SubcommandOption],
    selected: usize,
    frame: &mut Frame,
) {
    let area = frame.area();

    // Calculate popup size
    let max_name_len = options
        .iter()
        .map(|o| o.name.len())
        .max()
        .unwrap_or(10);
    let max_desc_len = options
        .iter()
        .map(|o| o.description.len())
        .max()
        .unwrap_or(20);

    // Width: name + " - " + description + borders + padding
    let width = (max_name_len + 3 + max_desc_len + 4).min(80) as u16;

    // Height: one line per option + borders
    let height = (options.len() as u16 + 2).min(12);

    // Position: bottom-left, above status line
    let popup_area = Rect {
        x: area.x + 2,
        y: area.height.saturating_sub(height + 2),
        width,
        height,
    };

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Build list items
    let items: Vec<ListItem> = options
        .iter()
        .map(|opt| {
            let spans = vec![
                Span::styled(
                    &opt.name,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" - "),
                Span::styled(&opt.description, Style::default().fg(Color::White)),
            ];
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Options (Tab/↓↑ to navigate, Enter to select, Esc to cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(selected));

    frame.render_stateful_widget(list, popup_area, &mut state);
}

/// Render file path suggestions list
fn render_filepath_suggestions(
    entries: &[crate::state::PathEntry],
    selected: usize,
    frame: &mut Frame,
) {
    let area = frame.area();

    // Calculate popup size
    let max_name_len = entries
        .iter()
        .map(|e| e.name.len() + if e.is_dir { 1 } else { 0 }) // +1 for '/' on dirs
        .max()
        .unwrap_or(20);

    // Width: name + padding + borders
    let width = (max_name_len + 6).min(60) as u16;

    // Height: limit to reasonable number
    let height = ((entries.len() as u16).min(15) + 2); // +2 for borders

    // Position: bottom-left, above status line
    let popup_area = Rect {
        x: area.x + 2,
        y: area.height.saturating_sub(height + 2),
        width,
        height,
    };

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Build list items
    let items: Vec<ListItem> = entries
        .iter()
        .map(|entry| {
            let (icon, color) = if entry.is_dir {
                ("📁 ", Color::Blue)
            } else {
                ("📄 ", Color::White)
            };

            let display_name = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };

            let spans = vec![
                Span::raw(icon),
                Span::styled(display_name, Style::default().fg(color)),
            ];
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Files (Tab/↓↑ to navigate, Enter to select, Esc to cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(selected));

    frame.render_stateful_widget(list, popup_area, &mut state);
}
/// Render global search popup
fn render_global_search_popup(app: &App, frame: &mut Frame) {
    let Some(search) = &app.global_search else {
        return;
    };

    let area = frame.area();

    // Create centered popup (80% width, 70% height)
    let popup_width = (area.width * 80) / 100;
    let popup_height = (area.height * 70) / 100;
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Split popup into input and results
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input field
            Constraint::Min(1),    // Results list
        ])
        .split(popup_area);

    // Render input field
    let input_text = if search.is_loading {
        format!("🔍 {} (searching...)", search.query)
    } else {
        format!(
            "🔍 {} ({} results)",
            search.query,
            search.total_count
        )
    };

    let input_widget = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Global Search (Esc to cancel) "),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(Clear, chunks[0]);
    frame.render_widget(input_widget, chunks[0]);

    // Render cursor in input field
    let cursor_x = chunks[0].x + 3 + search.cursor as u16; // +3 for "🔍 "
    let cursor_y = chunks[0].y + 1;
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));

    // Render results list
    let results: Vec<ListItem> = search
        .results
        .iter()
        .map(|result| {
            let timestamp = format_timestamp(result.timestamp);
            let preview = if result.text.chars().count() > 60 {
                let truncated: String = result.text.chars().take(60).collect();
                format!("{}...", truncated)
            } else {
                result.text.clone()
            };

            let lines = vec![
                Line::from(vec![
                    Span::styled(
                        &result.chat_title,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" • "),
                    Span::styled(&result.from_name, Style::default().fg(Color::Green)),
                    Span::raw(" • "),
                    Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(Span::styled(
                    preview,
                    Style::default().fg(Color::White),
                )),
            ];

            ListItem::new(lines)
        })
        .collect();

    let results_widget = List::new(results)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(
                    " Results ({}/{}) ",
                    search.selected + 1,
                    search.results.len()
                )),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(search.selected));

    frame.render_widget(Clear, chunks[1]);
    frame.render_stateful_widget(results_widget, chunks[1], &mut list_state);
}

