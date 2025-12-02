//! Terminal UI rendering with ratatui

use crate::board::{Cell, BOARD_HEIGHT, BOARD_WIDTH};
use crate::game::{Game, GameState};
use crate::menu::{Menu, MenuItemType, MenuScreen};
use crate::mode::GameMode;
use crate::settings::Settings;
use crate::tetromino::TetrominoType;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const EMPTY: &str = "  ";

/// Total width needed: hold(12) + board(22) + next/stats(16) = 50
const GAME_WIDTH: u16 = 50;
/// Total height needed: board(20) + 2 buffer rows + 2 for borders = 24
const GAME_HEIGHT: u16 = 24;
/// Number of rows to show above the visible board (spawn area)
const VISIBLE_BUFFER: usize = 2;

/// Render the main menu
pub fn render_menu(frame: &mut Frame, menu: &Menu) {
    let area = frame.area();

    // Determine menu size based on screen type
    let (menu_width, menu_height) = match menu.screen {
        MenuScreen::Main | MenuScreen::ModeSelect => (44u16, 18u16),
        MenuScreen::Settings => (44u16, 16u16),
        MenuScreen::SettingsKeys => (50u16, 24u16),
        MenuScreen::SettingsVisual | MenuScreen::SettingsGameplay | MenuScreen::SettingsAudio => (50u16, 14u16),
        MenuScreen::Multiplayer => (44u16, 14u16),
        MenuScreen::HostGame | MenuScreen::JoinGame => (60u16, 14u16),
        _ => (44u16, 16u16),
    };

    let menu_area = center_rect(area, menu_width, menu_height);

    // Title area height depends on screen
    let show_big_title = matches!(menu.screen, MenuScreen::Main | MenuScreen::ModeSelect);
    let title_height = if show_big_title { 6u16 } else { 3u16 };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_height),
            Constraint::Min(8),
        ])
        .split(menu_area);

    // Render title
    if show_big_title {
        let title_lines = vec![
            Line::styled("████████╗███████╗████████╗██████╗ ███████╗", Style::default().fg(Color::Cyan)),
            Line::styled("╚══██╔══╝██╔════╝╚══██╔══╝██╔══██╗██╔════╝", Style::default().fg(Color::Cyan)),
            Line::styled("   ██║   █████╗     ██║   ██████╔╝███████╗", Style::default().fg(Color::Cyan)),
            Line::styled("   ██║   ██╔══╝     ██║   ██╔══██╗╚════██║", Style::default().fg(Color::Cyan)),
            Line::styled("   ██║   ███████╗   ██║   ██║  ██║███████║", Style::default().fg(Color::Cyan)),
            Line::styled("   ╚═╝   ╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝", Style::default().fg(Color::Cyan)),
        ];
        let title = Paragraph::new(title_lines).alignment(Alignment::Center);
        frame.render_widget(title, layout[0]);
    } else {
        // Smaller title for settings screens
        let screen_title = match menu.screen {
            MenuScreen::Settings => "SETTINGS",
            MenuScreen::SettingsKeys => "KEY BINDINGS",
            MenuScreen::SettingsVisual => "VISUAL SETTINGS",
            MenuScreen::SettingsGameplay => "GAMEPLAY SETTINGS",
            MenuScreen::SettingsAudio => "AUDIO SETTINGS",
            MenuScreen::Multiplayer => "MULTIPLAYER",
            MenuScreen::HostGame => "HOST GAME",
            MenuScreen::JoinGame => "JOIN GAME",
            _ => "TETRS",
        };
        let title_lines = vec![
            Line::raw(""),
            Line::styled(screen_title, Style::default().fg(Color::Cyan).bold()),
        ];
        let title = Paragraph::new(title_lines).alignment(Alignment::Center);
        frame.render_widget(title, layout[0]);
    }

    // Menu items
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    let inner = block.inner(layout[1]);
    frame.render_widget(block, layout[1]);

    let mut lines = Vec::new();
    lines.push(Line::raw("")); // Spacing

    for (i, item) in menu.items.iter().enumerate() {
        let is_selected = i == menu.selected;
        let is_rebinding = menu.rebinding == Some(i);

        let line = render_menu_item(item, is_selected, is_rebinding, menu_width - 4);
        lines.push(line);
        lines.push(Line::raw("")); // Spacing between items
    }

    // Controls hint based on screen and current item type
    lines.push(Line::raw(""));
    let hint = get_controls_hint(menu);
    lines.push(Line::styled(hint, Style::default().fg(Color::DarkGray)));

    let menu_text = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(menu_text, inner);
}

/// Render a single menu item based on its type
fn render_menu_item(item: &crate::menu::MenuItem, is_selected: bool, is_rebinding: bool, _width: u16) -> Line<'static> {
    let prefix = if is_selected { "▶ " } else { "  " };

    let base_style = if is_selected {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::White)
    };

    match &item.item_type {
        MenuItemType::Button(_) => {
            Line::styled(format!("{}{}", prefix, item.label), base_style)
        }
        MenuItemType::Toggle { value, .. } => {
            let value_str = if *value { "ON" } else { "OFF" };
            let value_color = if *value { Color::Green } else { Color::Red };
            Line::from(vec![
                Span::styled(format!("{}{}: ", prefix, item.label), base_style),
                Span::styled(format!("< {} >", value_str), Style::default().fg(value_color).bold()),
            ])
        }
        MenuItemType::Cycle { options, current, .. } => {
            let value_str = &options[*current];
            Line::from(vec![
                Span::styled(format!("{}{}: ", prefix, item.label), base_style),
                Span::styled(format!("< {} >", value_str), Style::default().fg(Color::Cyan)),
            ])
        }
        MenuItemType::Number { value, .. } => {
            Line::from(vec![
                Span::styled(format!("{}{}: ", prefix, item.label), base_style),
                Span::styled(format!("< {} >", value), Style::default().fg(Color::Cyan)),
            ])
        }
        MenuItemType::KeyBind { keys, .. } => {
            if is_rebinding {
                Line::from(vec![
                    Span::styled(format!("{}{}: ", prefix, item.label), base_style),
                    Span::styled("Press a key...", Style::default().fg(Color::Yellow).bold()),
                ])
            } else {
                let keys_str = if keys.is_empty() {
                    "None".to_string()
                } else {
                    keys.join(", ")
                };
                Line::from(vec![
                    Span::styled(format!("{}{}: ", prefix, item.label), base_style),
                    Span::styled(format!("[{}]", keys_str), Style::default().fg(Color::Magenta)),
                ])
            }
        }
        MenuItemType::TextInput { value, placeholder } => {
            let display = if value.is_empty() {
                Span::styled(placeholder.clone(), Style::default().fg(Color::DarkGray))
            } else {
                Span::styled(value.clone(), Style::default().fg(Color::Green))
            };
            let cursor = if is_selected { "_" } else { "" };
            Line::from(vec![
                Span::styled(format!("{}{}: ", prefix, item.label), base_style),
                display,
                Span::styled(cursor.to_string(), Style::default().fg(Color::Yellow)),
            ])
        }
        MenuItemType::Label { text } => {
            if text.is_empty() {
                Line::styled(format!("  {}", item.label), Style::default().fg(Color::Gray))
            } else {
                Line::from(vec![
                    Span::styled(format!("  {}: ", item.label), Style::default().fg(Color::Gray)),
                    Span::styled(text.clone(), Style::default().fg(Color::Cyan)),
                ])
            }
        }
        _ => Line::styled(format!("{}{}", prefix, item.label), base_style),
    }
}

/// Get the controls hint based on current menu state
fn get_controls_hint(menu: &Menu) -> String {
    if menu.rebinding.is_some() {
        return "Key=Set | Shift+Key=Add more | Enter=Done | Esc=Cancel".to_string();
    }

    if let Some(item) = menu.items.get(menu.selected) {
        match &item.item_type {
            MenuItemType::Button(_) => "↑↓ Select  Enter Confirm  Esc Back".to_string(),
            MenuItemType::Toggle { .. } | MenuItemType::Cycle { .. } | MenuItemType::Number { .. } => {
                "↑↓ Select  ←→ Adjust  Esc Back".to_string()
            }
            MenuItemType::KeyBind { .. } => {
                "↑↓ Select  Enter Rebind  Del Clear  Esc Back".to_string()
            }
            MenuItemType::TextInput { .. } => {
                "Type to enter  Backspace to delete  Esc Back".to_string()
            }
            MenuItemType::Label { .. } => {
                "↑↓ Select  Esc Back".to_string()
            }
            _ => "↑↓ Select  Enter Confirm  Esc Back".to_string(),
        }
    } else {
        "↑↓ Select  Enter Confirm  Esc Back".to_string()
    }
}

/// Render the entire game UI
pub fn render_game(frame: &mut Frame, game: &Game, settings: &Settings) {
    let area = frame.area();
    let (block_char, _) = settings.visual.block_chars();

    // Center the game area
    let game_area = center_rect(area, GAME_WIDTH, GAME_HEIGHT);

    // Create main layout: hold | board | next + stats
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // Hold box
            Constraint::Length(22), // Board (10*2 + 2 for borders)
            Constraint::Length(16), // Next queue + stats
        ])
        .split(game_area);

    // Render hold piece
    render_hold(frame, main_layout[0], game.hold_piece, block_char);

    // Render main board
    render_board(frame, main_layout[1], game, settings);

    // Right side: next queue and stats
    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(14), // Next queue
            Constraint::Min(6),     // Stats
        ])
        .split(main_layout[2]);

    render_next_queue(frame, right_layout[0], game.preview(), block_char);
    render_stats(frame, right_layout[1], game);

    // Overlays
    match game.state {
        GameState::Countdown(n) => render_countdown(frame, area, n),
        GameState::Paused => render_overlay(frame, area, "PAUSED", "Press P to resume"),
        GameState::GameOver => {
            let subtitle = match game.mode() {
                GameMode::Ultra => "Time's up!",
                _ => "Press any key",
            };
            render_overlay(frame, area, "GAME OVER", subtitle);
        }
        GameState::Victory => {
            let time = game.mode_state.format_time();
            render_overlay(frame, area, "COMPLETE!", &format!("Time: {}", time));
        }
        GameState::Playing => {}
    }
}

/// Center a rect within another rect
fn center_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

/// Render the hold piece box
fn render_hold(frame: &mut Frame, area: Rect, hold: Option<TetrominoType>, block_char: &str) {
    let block = Block::default()
        .title(" HOLD ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(piece_type) = hold {
        render_mini_piece(frame, inner, piece_type, block_char);
    }
}

/// Render the next piece queue
fn render_next_queue(frame: &mut Frame, area: Rect, queue: &[TetrominoType], block_char: &str) {
    let block = Block::default()
        .title(" NEXT ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Show up to 4 pieces in the queue
    let num_pieces = queue.len().min(4);
    if num_pieces == 0 {
        return;
    }

    let piece_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(3); num_pieces])
        .split(inner);

    for (i, &piece_type) in queue.iter().take(num_pieces).enumerate() {
        render_mini_piece(frame, piece_areas[i], piece_type, block_char);
    }
}

/// Render a small piece preview (for hold and next queue)
fn render_mini_piece(frame: &mut Frame, area: Rect, piece_type: TetrominoType, block_char: &str) {
    if area.height < 1 || area.width < 4 {
        return;
    }

    let color = piece_type.color();
    let shape = piece_type.shape(crate::tetromino::Rotation::North);

    // Find bounding box to normalize coordinates
    let max_row = shape.iter().map(|(r, _)| *r).max().unwrap_or(0);
    let min_col = shape.iter().map(|(_, c)| *c).min().unwrap_or(0);

    // Build exactly 2 lines (standard piece height)
    // Iterate from max_row down since row increases upward but screen renders top-to-bottom
    let mut lines: Vec<Line> = Vec::new();
    for row_offset in 0..2 {
        let mut spans = Vec::new();
        for col_offset in 0..4 {
            let target_row = max_row - row_offset as i32;
            let target_col = min_col + col_offset as i32;

            if shape.contains(&(target_row, target_col)) {
                spans.push(Span::styled(block_char, Style::default().fg(color)));
            } else {
                spans.push(Span::raw(EMPTY));
            }
        }
        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Render the game board
fn render_board(frame: &mut Frame, area: Rect, game: &Game, settings: &Settings) {
    let (block_char, ghost_char) = settings.visual.block_chars();
    let show_ghost = settings.visual.show_ghost;

    let title = format!(" {} ", game.mode().name());
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build board display
    let mut lines: Vec<Line> = Vec::new();

    // Total visible rows: main board + buffer zone above
    let total_visible_rows = BOARD_HEIGHT + VISIBLE_BUFFER;

    // Render from top to bottom (buffer rows first, then main board)
    for row in (0..total_visible_rows).rev() {
        let mut spans = Vec::new();
        let is_buffer_row = row >= BOARD_HEIGHT;

        for col in 0..BOARD_WIDTH {
            // Check for current piece (visible in both main board and buffer)
            let current_block = game.current_piece.as_ref().and_then(|piece| {
                if piece
                    .block_positions()
                    .contains(&(row as i32, col as i32))
                {
                    Some((piece.piece_type.color(), false))
                } else {
                    None
                }
            });

            // Check for ghost piece (only in main board, not buffer)
            let ghost_block = if show_ghost && !is_buffer_row {
                game.current_piece.as_ref().and_then(|piece| {
                    let ghost_row = piece.ghost_row(&game.board);
                    let offsets = piece.piece_type.shape(piece.rotation);
                    for (dr, dc) in offsets {
                        if ghost_row + dr == row as i32 && piece.col + dc == col as i32 {
                            return Some((piece.piece_type.color(), true));
                        }
                    }
                    None
                })
            } else {
                None
            };

            // Determine what to render
            let (text, style) = if let Some((color, _)) = current_block {
                (block_char, Style::default().fg(color))
            } else if let Some((color, _)) = ghost_block {
                (ghost_char, Style::default().fg(color).dim())
            } else if is_buffer_row {
                // Buffer rows show empty space (no locked blocks visible)
                (EMPTY, Style::default())
            } else {
                match game.board.get(row as i32, col as i32) {
                    Some(Cell::Filled(color)) => (block_char, Style::default().fg(color)),
                    _ => (EMPTY, Style::default()),
                }
            };

            spans.push(Span::styled(text, style));
        }

        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render stats panel
fn render_stats(frame: &mut Frame, area: Rect, game: &Game) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();

    // Mode-specific display
    match game.mode() {
        GameMode::Marathon => {
            lines.push(Line::from(Span::styled("SCORE", Style::default().fg(Color::Gray))));
            lines.push(Line::from(Span::styled(
                format!("{}", game.score.points),
                Style::default().fg(Color::Yellow).bold(),
            )));
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("LEVEL", Style::default().fg(Color::Gray))));
            lines.push(Line::from(Span::styled(
                format!("{}", game.score.level),
                Style::default().fg(Color::Cyan),
            )));
        }
        GameMode::Sprint => {
            lines.push(Line::from(Span::styled("TIME", Style::default().fg(Color::Gray))));
            lines.push(Line::from(Span::styled(
                game.mode_state.format_time(),
                Style::default().fg(Color::Yellow).bold(),
            )));
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("LINES LEFT", Style::default().fg(Color::Gray))));
            let remaining = game.mode_state.lines_remaining(game.score.lines).unwrap_or(0);
            lines.push(Line::from(Span::styled(
                format!("{}", remaining),
                Style::default().fg(Color::Cyan),
            )));
        }
        GameMode::Ultra => {
            lines.push(Line::from(Span::styled("TIME LEFT", Style::default().fg(Color::Gray))));
            let remaining = game.mode_state.format_remaining().unwrap_or_default();
            lines.push(Line::from(Span::styled(
                remaining,
                Style::default().fg(Color::Red).bold(),
            )));
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("SCORE", Style::default().fg(Color::Gray))));
            lines.push(Line::from(Span::styled(
                format!("{}", game.score.points),
                Style::default().fg(Color::Yellow).bold(),
            )));
        }
        GameMode::Versus => {
            lines.push(Line::from(Span::styled("TIME", Style::default().fg(Color::Gray))));
            lines.push(Line::from(Span::styled(
                game.mode_state.format_time(),
                Style::default().fg(Color::Yellow).bold(),
            )));
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("ATTACK", Style::default().fg(Color::Gray))));
            lines.push(Line::from(Span::styled(
                format!("{}", game.score.points), // Will track garbage sent
                Style::default().fg(Color::Red).bold(),
            )));
        }
        _ => {}
    }

    // Lines cleared (all modes)
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("LINES", Style::default().fg(Color::Gray))));
    lines.push(Line::from(Span::styled(
        format!("{}", game.score.lines),
        Style::default().fg(Color::Green),
    )));

    // Show last action if any
    if let Some(action) = &game.last_action {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            action.clone(),
            Style::default().fg(Color::Magenta).bold(),
        ));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render countdown overlay (just colored text, no border)
fn render_countdown(frame: &mut Frame, area: Rect, count: u8) {
    let text = match count {
        3 => "3",
        2 => "2",
        1 => "1",
        _ => "GO!",
    };

    let color = match count {
        3 => Color::Red,
        2 => Color::Yellow,
        1 => Color::Green,
        _ => Color::Cyan,
    };

    // Center the text in the area
    let text_area = center_rect(area, 4, 1);

    let paragraph = Paragraph::new(Line::styled(text, Style::default().fg(color).bold()))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, text_area);
}

/// Render an overlay (for pause/game over)
fn render_overlay(frame: &mut Frame, area: Rect, title: &str, subtitle: &str) {
    let popup_width = 24u16;
    let popup_height = 5u16;
    let popup_area = center_rect(area, popup_width, popup_height);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let text = vec![
        Line::styled(title, Style::default().fg(Color::Yellow).bold()),
        Line::raw(""),
        Line::styled(subtitle, Style::default().fg(Color::Gray)),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

/// Render versus mode UI (two boards side by side)
pub fn render_versus(
    frame: &mut Frame,
    game: &Game,
    session: &crate::multiplayer::MultiplayerSession,
    settings: &Settings,
) {
    use crate::multiplayer::ConnectionState;

    let area = frame.area();

    // Show different screens based on connection state
    match &session.state {
        ConnectionState::WaitingForOpponent { ticket } => {
            render_waiting_screen(frame, area, "Waiting for opponent...", Some(ticket));
        }
        ConnectionState::Connecting => {
            render_waiting_screen(frame, area, "Connecting...", None);
        }
        ConnectionState::Connected => {
            render_waiting_screen(frame, area, "Connected! Entering lobby...", None);
        }
        ConnectionState::Lobby { we_ready, they_ready } => {
            render_lobby(frame, area, session, *we_ready, *they_ready);
        }
        ConnectionState::Countdown { value } => {
            // Show both boards but with countdown overlay
            render_versus_game(frame, game, session, settings);
            render_countdown(frame, area, *value);
        }
        ConnectionState::Playing => {
            render_versus_game(frame, game, session, settings);
        }
        ConnectionState::GameOver { we_won } => {
            render_versus_game(frame, game, session, settings);
            if *we_won {
                render_overlay(frame, area, "YOU WIN!", "Press any key");
            } else {
                render_overlay(frame, area, "YOU LOSE", "Press any key");
            }
        }
        ConnectionState::Disconnected => {
            render_waiting_screen(frame, area, "Disconnected", None);
        }
        _ => {
            render_game(frame, game, settings);
        }
    }
}

/// Render waiting/connecting screen
fn render_waiting_screen(frame: &mut Frame, area: Rect, message: &str, ticket_data: Option<&str>) {
    let popup_height = if ticket_data.is_some() { 14u16 } else { 5u16 };
    let popup_width = 70u16;
    let popup_area = center_rect(area, popup_width, popup_height);

    let block = Block::default()
        .title(" VERSUS MODE ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = vec![
        Line::raw(""),
        Line::styled(message, Style::default().fg(Color::Yellow).bold()),
    ];

    if let Some(ticket_data) = ticket_data {
        // Split ticket and info (format: "ticket\ninfo")
        let parts: Vec<&str> = ticket_data.splitn(2, '\n').collect();
        let ticket = parts.first().unwrap_or(&"");
        let info = parts.get(1).unwrap_or(&"");

        lines.push(Line::raw(""));
        lines.push(Line::styled("Your ticket (share with friend):", Style::default().fg(Color::Gray)));
        lines.push(Line::raw(""));

        // Show ticket, wrapping if needed
        if ticket.len() <= 64 {
            lines.push(Line::styled(*ticket, Style::default().fg(Color::Green)));
        } else {
            // Split into two lines
            let mid = ticket.len() / 2;
            lines.push(Line::styled(&ticket[..mid], Style::default().fg(Color::Green)));
            lines.push(Line::styled(&ticket[mid..], Style::default().fg(Color::Green)));
        }

        lines.push(Line::raw(""));
        // Show clipboard/file status
        let info_color = if info.contains("CLIPBOARD") { Color::Cyan } else { Color::DarkGray };
        lines.push(Line::styled(*info, Style::default().fg(info_color).bold()));
        lines.push(Line::raw(""));
        lines.push(Line::styled("Press ESC to cancel", Style::default().fg(Color::DarkGray)));
    }

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

/// Render lobby screen showing both players and ready status
fn render_lobby(
    frame: &mut Frame,
    area: Rect,
    session: &crate::multiplayer::MultiplayerSession,
    we_ready: bool,
    they_ready: bool,
) {
    let popup_width = 50u16;
    let popup_height = 12u16;
    let popup_area = center_rect(area, popup_width, popup_height);

    let block = Block::default()
        .title(" GAME LOBBY ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let our_status = if we_ready { "READY" } else { "NOT READY" };
    let our_color = if we_ready { Color::Green } else { Color::Yellow };
    let their_status = if they_ready { "READY" } else { "NOT READY" };
    let their_color = if they_ready { Color::Green } else { Color::Yellow };

    let role_name = match session.role {
        crate::multiplayer::Role::Host => "Host",
        crate::multiplayer::Role::Guest => "Guest",
    };

    let lines = vec![
        Line::raw(""),
        Line::styled(
            format!("You ({})", role_name),
            Style::default().fg(Color::White).bold(),
        ),
        Line::styled(format!("  {}", our_status), Style::default().fg(our_color).bold()),
        Line::raw(""),
        Line::styled(
            format!("Opponent: {}", session.opponent.name),
            Style::default().fg(Color::White).bold(),
        ),
        Line::styled(format!("  {}", their_status), Style::default().fg(their_color).bold()),
        Line::raw(""),
        if we_ready {
            Line::styled("Waiting for opponent...", Style::default().fg(Color::DarkGray))
        } else {
            Line::styled("Press ENTER or SPACE to ready up", Style::default().fg(Color::Cyan))
        },
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

/// Render versus game with both boards
fn render_versus_game(
    frame: &mut Frame,
    game: &Game,
    session: &crate::multiplayer::MultiplayerSession,
    settings: &Settings,
) {
    let area = frame.area();
    let (block_char, _) = settings.visual.block_chars();

    // Wide layout: our board | middle info | opponent mini board
    let versus_width = 72u16;
    let versus_height = 24u16;
    let versus_area = center_rect(area, versus_width, versus_height);

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(50), // Our full game
            Constraint::Length(22), // Opponent mini board + stats
        ])
        .split(versus_area);

    // Render our game on the left
    let our_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // Hold
            Constraint::Length(22), // Board
            Constraint::Length(16), // Next + stats
        ])
        .split(main_layout[0]);

    render_hold(frame, our_layout[0], game.hold_piece, block_char);
    render_board(frame, our_layout[1], game, settings);

    let our_right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(14), // Next
            Constraint::Min(6),     // Stats
        ])
        .split(our_layout[2]);

    render_next_queue(frame, our_right[0], game.preview(), block_char);
    render_stats(frame, our_right[1], game);

    // Render opponent on the right
    let opp_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Mini board
            Constraint::Min(6),     // Stats
        ])
        .split(main_layout[1]);

    render_opponent_board(frame, opp_layout[0], session, block_char);
    render_opponent_stats(frame, opp_layout[1], session);
}

/// Render a small opponent board preview
fn render_opponent_board(
    frame: &mut Frame,
    area: Rect,
    session: &crate::multiplayer::MultiplayerSession,
    block_char: &str,
) {
    let block = Block::default()
        .title(format!(" {} ", session.opponent.name))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(if session.opponent.game_over {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Gray)
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render mini board (scaled down - show every other row)
    let mut lines: Vec<Line> = Vec::new();
    let visible_rows = inner.height as usize;

    for screen_row in 0..visible_rows {
        let board_row = (crate::board::BOARD_HEIGHT - 1) - (screen_row * 2);
        if board_row >= crate::board::BOARD_HEIGHT {
            continue;
        }

        let mut spans = Vec::new();
        for col in 0..crate::board::BOARD_WIDTH {
            let cell = session.opponent.board[board_row][col];
            match cell {
                Cell::Empty => spans.push(Span::raw(" ")),
                Cell::Filled(color) => spans.push(Span::styled(
                    &block_char[0..block_char.chars().next().map(|c| c.len_utf8()).unwrap_or(1)],
                    Style::default().fg(color),
                )),
            }
        }
        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render opponent stats
fn render_opponent_stats(
    frame: &mut Frame,
    area: Rect,
    session: &crate::multiplayer::MultiplayerSession,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("SCORE", Style::default().fg(Color::Gray))),
        Line::from(Span::styled(
            format!("{}", session.opponent.score),
            Style::default().fg(Color::White).bold(),
        )),
        Line::raw(""),
        Line::from(Span::styled("LINES", Style::default().fg(Color::Gray))),
        Line::from(Span::styled(
            format!("{}", session.opponent.lines),
            Style::default().fg(Color::Green),
        )),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
