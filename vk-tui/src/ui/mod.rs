use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{App, DeliveryStatus, Focus, Screen};

/// Main view function - renders the entire UI
pub fn view(app: &App, frame: &mut Frame) {
    match app.screen {
        Screen::Auth => render_auth_screen(app, frame),
        Screen::Main => render_main_screen(app, frame),
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

    let items: Vec<ListItem> = app
        .chats
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

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|msg| {
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

            let mut lines = vec![Line::from(vec![
                Span::styled(time, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(&msg.from_name, name_style),
                Span::raw(": "),
                Span::raw(&msg.text),
                if read_indicator.is_empty() {
                    Span::raw("")
                } else {
                    Span::styled(
                        format!(" {}", read_indicator),
                        Style::default().fg(Color::DarkGray),
                    )
                },
            ])];

            for att in &msg.attachments {
                let label = match &att.kind {
                    crate::app::AttachmentKind::Photo => "[photo]".to_string(),
                    crate::app::AttachmentKind::File => "[file]".to_string(),
                    crate::app::AttachmentKind::Other(k) => format!("[{}]", k),
                };
                let mut detail = format!("{} {}", label, att.title);
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

            ListItem::new(lines)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let chat_title = app
        .current_chat()
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

    frame.render_stateful_widget(list, area, &mut state);
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
    let default_help = match app.focus {
        Focus::Input => "Enter send | /sendfile PATH | /sendimg PATH|--clipboard | Esc back",
        _ => {
            "j/k nav | h/l panels | i/Enter select | Ctrl+L open link | Ctrl+D save attach | Esc back"
        }
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
