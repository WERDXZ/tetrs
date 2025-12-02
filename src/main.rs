//! TETRS - A Rusty Tetris
//!
//! Proving Rust superiority one block at a time.

mod audio;
mod bag;
mod board;
mod game;
mod input;
mod menu;
mod mode;
mod multiplayer;
mod piece;
mod score;
mod settings;
mod srs;
mod tetromino;
mod ui;

use audio::{AudioManager, BgmTrack, Sfx};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use game::{Game, GameState};
use input::InputHandler;
use menu::{Menu, MenuAction, MenuScreen};
use mode::GameMode;
use multiplayer::{MultiplayerSession, NetEvent, Role};
use crossterm::event::MouseEvent;
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use settings::Settings;
use std::{
    io::{self, stdout},
    time::Duration,
};

/// Target frame rate
const TARGET_FPS: u64 = 60;
const FRAME_DURATION: Duration = Duration::from_micros(1_000_000 / TARGET_FPS);

/// Application state
enum AppState {
    Menu(Menu),
    Playing(Game, InputHandler),
    /// Versus mode with multiplayer session
    Versus(Game, InputHandler, MultiplayerSession),
}

/// Get the tetrs temp directory, creating it if needed
fn tetrs_temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("tetrs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn main() -> io::Result<()> {
    // Generate session ID for this instance
    let session_id: u32 = rand::random();

    // Setup tetrs temp directory for logs and tickets
    let tetrs_dir = tetrs_temp_dir();
    let log_file = format!("{:08x}.log", session_id);

    // Setup tracing to log file
    let file_appender = tracing_appender::rolling::never(&tetrs_dir, &log_file);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("tetrs=debug".parse().unwrap())
                .add_directive("iroh=info".parse().unwrap())
        )
        .with_ansi(false)
        .init();

    tracing::info!("TETRS starting up, session={:08x}, log={}", session_id, tetrs_dir.join(&log_file).display());

    // Load settings
    let mut settings = Settings::load();

    // Initialize audio (optional - game works without audio)
    let mut audio = AudioManager::new();
    if let Some(ref mut a) = audio {
        a.set_bgm_volume(settings.audio.bgm_volume as f32 / 100.0);
        a.set_sfx_volume(settings.audio.sfx_volume as f32 / 100.0);
    }

    // Create async runtime for networking
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create async runtime");

    // Setup terminal
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Run app and capture result
    let result = run_app(&mut terminal, &mut settings, &mut audio, runtime.handle());

    // Restore terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    // Save settings (including any new high scores)
    if let Err(e) = settings.save() {
        eprintln!("Warning: Could not save settings: {}", e);
    }

    // Print final message
    match &result {
        Ok(Some(game)) => {
            println!("\n🦀 Thanks for playing TETRS! 🦀");
            println!("Mode: {}", game.mode().name());
            println!("Final Score: {}", game.score.points);
            println!("Level: {} | Lines: {}", game.score.level, game.score.lines);
            if game.mode() == GameMode::Sprint {
                println!("Time: {}", game.mode_state.format_time());
            }
        }
        Ok(None) => {
            println!("\n🦀 Thanks for playing TETRS! 🦀");
        }
        Err(_) => {}
    }

    result.map(|_| ())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    settings: &mut Settings,
    audio: &mut Option<AudioManager>,
    rt: &tokio::runtime::Handle,
) -> io::Result<Option<Game>> {
    let mut state = AppState::Menu(Menu::new());
    let mut last_game: Option<Game> = None;
    let mut last_countdown: Option<u8> = None;
    let mut last_action_text: Option<String> = None;
    let mut countdown_start: Option<std::time::Instant> = None;

    loop {
        // Render
        terminal.draw(|frame| match &state {
            AppState::Menu(menu) => ui::render_menu(frame, menu),
            AppState::Playing(game, _) => ui::render_game(frame, game, settings),
            AppState::Versus(game, _, session) => {
                ui::render_versus(frame, game, session, settings);
            }
        })?;

        // Handle input
        if event::poll(FRAME_DURATION)? {
            let event = event::read()?;

            match event {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        // Also handle key releases for game input
                        if let AppState::Playing(_, input) | AppState::Versus(_, input, _) = &mut state {
                            if key.kind == KeyEventKind::Release {
                                input.key_up(key);
                            }
                        }
                        continue;
                    }

                    match &mut state {
                        AppState::Menu(menu) => {
                            // Handle key rebinding mode
                            if menu.rebinding.is_some() {
                                match key.code {
                                    KeyCode::Esc => {
                                        menu.cancel_rebind();
                                    }
                                    KeyCode::Enter => {
                                        // Finish adding keys
                                        menu.finish_rebind();
                                    }
                                    _ => {
                                        // Skip modifier keys by themselves
                                        if matches!(key.code, KeyCode::Modifier(_)) {
                                            continue;
                                        }
                                        let key_str = key_to_string(key.code);
                                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                                            // Shift+Key adds to existing bindings (keeps rebind mode)
                                            menu.add_key(key_str, settings);
                                        } else {
                                            // Regular key replaces all bindings and exits
                                            menu.set_key(key_str, settings);
                                        }
                                    }
                                }
                                continue;
                            }

                            // Check if currently on a TextInput
                            let on_text_input = menu.items.get(menu.selected)
                                .map(|item| matches!(item.item_type, crate::menu::MenuItemType::TextInput { .. }))
                                .unwrap_or(false);

                            // Handle text input typing
                            if on_text_input {
                                match key.code {
                                    KeyCode::Char(c) => {
                                        menu.text_input_char(c);
                                        continue;
                                    }
                                    KeyCode::Backspace => {
                                        menu.text_input_backspace();
                                        continue;
                                    }
                                    // Allow navigation and enter to pass through
                                    KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Esc => {}
                                    _ => continue,
                                }
                            }

                            match key.code {
                                KeyCode::Up => {
                                    menu.move_up();
                                    if let Some(audio) = audio {
                                        audio.play_sfx(Sfx::SelectMove);
                                    }
                                }
                                KeyCode::Down => {
                                    menu.move_down();
                                    if let Some(audio) = audio {
                                        audio.play_sfx(Sfx::SelectMove);
                                    }
                                }
                                KeyCode::Left => menu.adjust_left(settings),
                                KeyCode::Right => menu.adjust_right(settings),
                                KeyCode::Enter => {
                                    // Check if current item is a keybind
                                    if let Some(item) = menu.items.get(menu.selected) {
                                        if matches!(item.item_type, crate::menu::MenuItemType::KeyBind { .. }) {
                                            menu.start_rebind();
                                            continue;
                                        }
                                    }

                                    if let Some(action) = menu.select().cloned() {
                                        if let Some(audio) = audio {
                                            audio.play_sfx(Sfx::SelectConfirm);
                                        }
                                        match action {
                                            MenuAction::StartGame(mode) => {
                                                let game = Game::new(mode);
                                                let input = InputHandler::from_settings(settings);
                                                // Start background music
                                                if let Some(audio) = audio {
                                                    let track = match settings.audio.bgm_track.as_str() {
                                                        "Korobeiniki (Fast)" => BgmTrack::KorobeinikiFast,
                                                        "Kalinka" => BgmTrack::Kalinka,
                                                        "Ievan Polkka" => BgmTrack::IevanPolkka,
                                                        _ => BgmTrack::Korobeiniki,
                                                    };
                                                    audio.play_bgm(track);
                                                }
                                                state = AppState::Playing(game, input);
                                            }
                                            MenuAction::GoToScreen(screen) => {
                                                menu.go_to(screen, settings);
                                            }
                                            MenuAction::Back => {
                                                if let Some(audio) = audio {
                                                    audio.play_sfx(Sfx::SelectBack);
                                                }
                                                menu.go_back(settings);
                                            }
                                            MenuAction::Quit => {
                                                return Ok(last_game);
                                            }
                                            MenuAction::SaveSettings => {
                                                let _ = settings.save();
                                            }
                                            MenuAction::HostGame => {
                                                // Generate a random seed for this game
                                                let seed = rand::random::<u64>();
                                                let our_name = "Player".to_string();

                                                match multiplayer::spawn_host(rt, our_name, seed) {
                                                    Ok((ticket, cmd_tx, event_rx)) => {
                                                        // Write ticket to file in tetrs temp dir
                                                        let ticket_path = tetrs_temp_dir().join("ticket.txt");
                                                        let _ = std::fs::write(&ticket_path, &ticket);
                                                        let ticket_info = format!("Saved to: {}", ticket_path.display());

                                                        // Create session
                                                        let mut session = MultiplayerSession::new(Role::Host);
                                                        session.game_seed = seed;
                                                        session.set_channels(cmd_tx, event_rx);
                                                        session.state = multiplayer::ConnectionState::WaitingForOpponent {
                                                            ticket: format!("{}\n{}", ticket, ticket_info)
                                                        };

                                                        // Create game with our seed
                                                        let game = Game::with_seed(GameMode::Versus, seed);
                                                        let input = InputHandler::from_settings(settings);
                                                        state = AppState::Versus(game, input, session);
                                                    }
                                                    Err(_e) => {
                                                        // Stay in menu, could show error
                                                    }
                                                }
                                            }
                                            MenuAction::JoinGame => {
                                                // Get ticket from text input
                                                if let Some(input_ticket) = menu.get_ticket_input() {
                                                    // Check if it's a file path and read contents
                                                    let ticket = if std::path::Path::new(&input_ticket).exists() {
                                                        std::fs::read_to_string(&input_ticket)
                                                            .unwrap_or(input_ticket)
                                                    } else {
                                                        input_ticket
                                                    };

                                                    let our_name = "Player".to_string();

                                                    match multiplayer::spawn_join(rt, ticket, our_name) {
                                                        Ok((cmd_tx, event_rx)) => {
                                                            let mut session = MultiplayerSession::new(Role::Guest);
                                                            session.set_channels(cmd_tx, event_rx);
                                                            session.state = multiplayer::ConnectionState::Connecting;

                                                            // Transition to versus game
                                                            let game = Game::new(GameMode::Versus);
                                                            let input = InputHandler::from_settings(settings);
                                                            state = AppState::Versus(game, input, session);
                                                        }
                                                        Err(_e) => {
                                                            // Stay in menu, could show error
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                KeyCode::Delete | KeyCode::Backspace => {
                                    // Clear keybindings for current item
                                    if let Some(item) = menu.items.get_mut(menu.selected) {
                                        if let crate::menu::MenuItemType::KeyBind { action, keys } = &mut item.item_type {
                                            keys.clear();
                                            crate::menu::update_key_binding_pub(settings, action, keys.clone());
                                        }
                                    }
                                }
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    if menu.screen == MenuScreen::Main {
                                        return Ok(last_game);
                                    } else {
                                        if let Some(audio) = audio {
                                            audio.play_sfx(Sfx::SelectBack);
                                        }
                                        menu.go_back(settings);
                                    }
                                }
                                _ => {}
                            }
                        }
                        AppState::Playing(game, input) => {
                            // Process input
                            let actions = input.key_down(key);
                            for action in actions {
                                game.process_action(action);
                            }

                            // Check for game end
                            match game.state {
                                GameState::GameOver | GameState::Victory => {
                                    // Save high score
                                    save_high_score(game, settings);

                                    // Stop BGM
                                    if let Some(audio) = audio {
                                        audio.stop_bgm();
                                    }

                                    // Any key returns to menu
                                    last_game = Some(std::mem::replace(
                                        game,
                                        Game::new(GameMode::Marathon),
                                    ));
                                    state = AppState::Menu(Menu::new());
                                }
                                _ => {}
                            }
                        }
                        AppState::Versus(game, input, session) => {
                            // Handle lobby ready-up
                            if let multiplayer::ConnectionState::Lobby { we_ready, they_ready } = session.state {
                                if !we_ready && (key.code == KeyCode::Enter || key.code == KeyCode::Char(' ')) {
                                    session.state = multiplayer::ConnectionState::Lobby {
                                        we_ready: true,
                                        they_ready,
                                    };
                                    session.send(multiplayer::GameMessage::Ready);
                                    // If both ready now, host starts countdown
                                    if they_ready && session.role == Role::Host {
                                        countdown_start = Some(std::time::Instant::now());
                                        session.state = multiplayer::ConnectionState::Countdown { value: 3 };
                                        session.send(multiplayer::GameMessage::Countdown { value: 3 });
                                    }
                                }
                            }

                            // Only process game input if we're actually playing
                            if matches!(session.state, multiplayer::ConnectionState::Playing) {
                                let actions = input.key_down(key);
                                for action in actions {
                                    game.process_action(action);
                                }
                            }

                            // Handle escape to quit/disconnect
                            if key.code == KeyCode::Esc {
                                session.send_disconnect();
                            }
                        }
                    }

                    // Handle state transitions outside the match to avoid borrow issues
                    let should_return_to_menu = match &mut state {
                        AppState::Versus(game, _, session) => {
                            if matches!(session.state, multiplayer::ConnectionState::Disconnected) {
                                if let Some(audio) = audio {
                                    audio.stop_bgm();
                                }
                                true
                            } else if matches!(session.state, multiplayer::ConnectionState::GameOver { .. }) {
                                if let Some(audio) = audio {
                                    audio.stop_bgm();
                                }
                                last_game = Some(std::mem::replace(
                                    game,
                                    Game::new(GameMode::Marathon),
                                ));
                                true
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };

                    if should_return_to_menu {
                        state = AppState::Menu(Menu::new());
                    }
                }
                Event::Mouse(mouse) => {
                    if let AppState::Menu(menu) = &mut state {
                        // Don't handle mouse while rebinding
                        if menu.rebinding.is_some() {
                            continue;
                        }
                        let size = terminal.size()?;
                        let area = Rect::new(0, 0, size.width, size.height);
                        if let Some(action) = handle_menu_mouse(menu, mouse, area, settings) {
                            match action {
                                MenuAction::StartGame(mode) => {
                                    let game = Game::new(mode);
                                    let input = InputHandler::from_settings(settings);
                                    state = AppState::Playing(game, input);
                                }
                                MenuAction::GoToScreen(screen) => {
                                    menu.go_to(screen, settings);
                                }
                                MenuAction::Back => {
                                    menu.go_back(settings);
                                }
                                MenuAction::Quit => {
                                    return Ok(last_game);
                                }
                                MenuAction::SaveSettings => {
                                    let _ = settings.save();
                                }
                                MenuAction::HostGame | MenuAction::JoinGame => {
                                    // TODO: Networking
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Update game state
        match &mut state {
            AppState::Playing(game, input) => {
                // Process held keys for DAS/ARR
                let held_actions = input.update();
                for action in held_actions {
                    game.process_action(action);
                }

                // Check countdown for SFX
                if let GameState::Countdown(count) = game.state {
                    if last_countdown != Some(count) {
                        if let Some(audio) = audio {
                            if count == 0 {
                                audio.play_sfx(Sfx::Go);
                            } else {
                                audio.play_sfx(Sfx::Countdown);
                            }
                        }
                        last_countdown = Some(count);
                    }
                } else {
                    last_countdown = None;
                }

                // Update game logic
                game.update();

                // Play SFX for line clears
                if game.last_action != last_action_text {
                    if let Some(ref action) = game.last_action {
                        if let Some(audio) = audio {
                            let sfx = if action.contains("T-Spin Triple") {
                                Some(Sfx::TSpinTriple)
                            } else if action.contains("T-Spin Double") {
                                Some(Sfx::TSpinDouble)
                            } else if action.contains("T-Spin Single") {
                                Some(Sfx::TSpinSingle)
                            } else if action.contains("Tetris") {
                                Some(Sfx::Quad)
                            } else if action.contains("Triple") {
                                Some(Sfx::Triple)
                            } else if action.contains("Double") {
                                Some(Sfx::Double)
                            } else if action.contains("Single") {
                                Some(Sfx::Single)
                            } else {
                                None
                            };
                            if let Some(sfx) = sfx {
                                audio.play_sfx(sfx);
                            }
                        }
                    }
                    last_action_text = game.last_action.clone();
                }

                // Handle pause/resume BGM
                if game.state == GameState::Paused {
                    input.clear();
                    if let Some(audio) = audio {
                        audio.pause_bgm();
                    }
                } else if game.state == GameState::Playing {
                    if let Some(audio) = audio {
                        audio.resume_bgm();
                    }
                }
            }
            AppState::Versus(game, input, session) => {
                // Process network events and update session state
                let events = session.poll_events();
                for event in events {
                    match event {
                        NetEvent::Connected { opponent_name } => {
                            session.opponent.name = opponent_name;
                            session.state = multiplayer::ConnectionState::Connected;
                        }
                        NetEvent::SeedReceived { seed } => {
                            // Guest receives seed from host - recreate game with same seed
                            session.game_seed = seed;
                            *game = Game::with_seed(GameMode::Versus, seed);
                        }
                        NetEvent::OpponentReady => {
                            // Opponent is ready - update lobby state
                            if let multiplayer::ConnectionState::Lobby { we_ready, .. } = session.state {
                                session.state = multiplayer::ConnectionState::Lobby {
                                    we_ready,
                                    they_ready: true,
                                };
                                // If both ready, host starts countdown
                                if we_ready && session.role == Role::Host {
                                    countdown_start = Some(std::time::Instant::now());
                                    session.state = multiplayer::ConnectionState::Countdown { value: 3 };
                                    session.send(multiplayer::GameMessage::Countdown { value: 3 });
                                }
                            }
                        }
                        NetEvent::Countdown { value } => {
                            // Guest receives countdown from host
                            if value == 0 {
                                session.state = multiplayer::ConnectionState::Playing;
                                countdown_start = None;
                            } else {
                                session.state = multiplayer::ConnectionState::Countdown { value };
                            }
                        }
                        NetEvent::BoardUpdate { cells, score, lines, level } => {
                            session.opponent.update_from_message(&cells, score, lines, level);
                        }
                        NetEvent::GarbageReceived { lines } => {
                            session.pending_garbage += lines;
                        }
                        NetEvent::OpponentGameOver { final_score: _ } => {
                            // We win!
                            session.opponent.game_over = true;
                            session.state = multiplayer::ConnectionState::GameOver { we_won: true };
                        }
                        NetEvent::Disconnected { reason: _ } => {
                            session.state = multiplayer::ConnectionState::GameOver { we_won: true };
                        }
                        NetEvent::Error { message: _ } => {
                            session.state = multiplayer::ConnectionState::Disconnected;
                        }
                    }
                }

                // Handle state transitions based on connection state
                match &session.state {
                    multiplayer::ConnectionState::WaitingForOpponent { .. } => {
                        // Just waiting, do nothing
                    }
                    multiplayer::ConnectionState::Connected => {
                        // Transition to lobby - both players need to ready up
                        session.state = multiplayer::ConnectionState::Lobby {
                            we_ready: false,
                            they_ready: false,
                        };
                    }
                    multiplayer::ConnectionState::Lobby { we_ready, they_ready } => {
                        // Lobby state - waiting for both players to ready up
                        // Input handling is done in the input section below
                        // Check if both ready (guest side - host handles this in OpponentReady)
                        if *we_ready && *they_ready && session.role == Role::Guest {
                            // Guest waits for host to send countdown
                        }
                    }
                    multiplayer::ConnectionState::Countdown { value } => {
                        // Play sound on countdown change
                        if last_countdown != Some(*value) {
                            if let Some(audio) = audio {
                                if *value == 0 {
                                    audio.play_sfx(Sfx::Go);
                                } else {
                                    audio.play_sfx(Sfx::Countdown);
                                }
                            }
                            last_countdown = Some(*value);
                        }

                        // Host drives the countdown timer
                        if session.role == Role::Host {
                            if let Some(start) = countdown_start {
                                let elapsed = start.elapsed().as_secs();
                                let new_value = 3u8.saturating_sub(elapsed as u8);
                                if new_value != *value {
                                    if new_value == 0 {
                                        session.state = multiplayer::ConnectionState::Playing;
                                        session.send(multiplayer::GameMessage::Countdown { value: 0 });
                                        countdown_start = None;
                                    } else {
                                        session.state = multiplayer::ConnectionState::Countdown { value: new_value };
                                        session.send(multiplayer::GameMessage::Countdown { value: new_value });
                                    }
                                }
                            }
                        }
                    }
                    multiplayer::ConnectionState::Playing => {
                        last_countdown = None;

                        // Process held keys for DAS/ARR
                        let held_actions = input.update();
                        for action in held_actions {
                            game.process_action(action);
                        }

                        // Update game logic
                        game.update();

                        // Check if piece was locked - send board state and garbage
                        if game.piece_just_locked {
                            session.send_board_state(game);

                            // Calculate and send garbage if we cleared lines
                            if let Some(ref clear_info) = game.last_clear_info {
                                let garbage = multiplayer::calculate_garbage(
                                    clear_info.lines as u32,
                                    clear_info.is_tspin,
                                    clear_info.combo.max(0) as u32,
                                    clear_info.back_to_back,
                                );
                                if garbage > 0 {
                                    session.send_garbage(garbage);
                                }
                            }

                            game.piece_just_locked = false;
                        }

                        // Apply pending garbage (TODO: add garbage lines to board)
                        let _garbage = session.take_pending_garbage();
                        // if garbage > 0 { game.add_garbage_lines(garbage); }

                        // Check for game over
                        if game.state == GameState::GameOver {
                            session.send_game_over(game.score.points);
                            session.state = multiplayer::ConnectionState::GameOver { we_won: false };
                        }

                        // Play SFX for line clears
                        if game.last_action != last_action_text {
                            if let Some(ref action) = game.last_action {
                                if let Some(audio) = audio {
                                    let sfx = if action.contains("T-Spin Triple") {
                                        Some(Sfx::TSpinTriple)
                                    } else if action.contains("T-Spin Double") {
                                        Some(Sfx::TSpinDouble)
                                    } else if action.contains("T-Spin Single") {
                                        Some(Sfx::TSpinSingle)
                                    } else if action.contains("Tetris") {
                                        Some(Sfx::Quad)
                                    } else if action.contains("Triple") {
                                        Some(Sfx::Triple)
                                    } else if action.contains("Double") {
                                        Some(Sfx::Double)
                                    } else if action.contains("Single") {
                                        Some(Sfx::Single)
                                    } else {
                                        None
                                    };
                                    if let Some(sfx) = sfx {
                                        audio.play_sfx(sfx);
                                    }
                                }
                            }
                            last_action_text = game.last_action.clone();
                        }

                        // Start BGM if not playing
                        if let Some(audio) = audio {
                            audio.resume_bgm();
                        }
                    }
                    _ => {
                        last_countdown = None;
                    }
                }
            }
            AppState::Menu(_) => {}
        }
    }
}

/// Save high score based on game mode
fn save_high_score(game: &Game, settings: &mut Settings) {
    match game.mode() {
        GameMode::Marathon => {
            settings.add_marathon_score(game.score.points, game.score.lines, game.score.level);
        }
        GameMode::Sprint => {
            if game.state == GameState::Victory {
                let time_ms = game.mode_state.elapsed.as_millis() as u64;
                settings.add_sprint_score(time_ms, game.score.lines, game.score.level);
            }
        }
        GameMode::Ultra => {
            settings.add_ultra_score(game.score.points, game.score.lines, game.score.level);
        }
        GameMode::Versus => {
            // Versus mode doesn't save high scores (multiplayer results)
        }
        _ => {}
    }
}

/// Handle mouse events in the menu
fn handle_menu_mouse(menu: &mut Menu, mouse: MouseEvent, size: Rect, settings: &mut Settings) -> Option<MenuAction> {
    // Menu layout constants - dynamic based on screen type
    let (menu_width, menu_height) = match menu.screen {
        MenuScreen::Main | MenuScreen::ModeSelect => (44u16, 18u16),
        MenuScreen::Settings => (44u16, 16u16),
        MenuScreen::SettingsKeys => (50u16, 24u16),
        MenuScreen::SettingsVisual | MenuScreen::SettingsGameplay | MenuScreen::SettingsAudio => (50u16, 14u16),
        MenuScreen::Multiplayer => (44u16, 14u16),
        MenuScreen::HostGame | MenuScreen::JoinGame => (60u16, 14u16),
        _ => (44u16, 16u16),
    };

    let show_big_title = matches!(menu.screen, MenuScreen::Main | MenuScreen::ModeSelect);
    let title_height: u16 = if show_big_title { 6 } else { 3 };

    // Calculate menu area (centered)
    let menu_x = size.x + size.width.saturating_sub(menu_width) / 2;
    let menu_y = size.y + size.height.saturating_sub(menu_height) / 2;

    // Calculate the inner area (after title and border)
    let inner_y = menu_y + title_height + 1; // +1 for top border
    let inner_x = menu_x + 1; // +1 for left border
    let inner_width = menu_width - 2; // -2 for borders

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;

            // Check if click is within menu items area
            if x >= inner_x && x < inner_x + inner_width && y > inner_y {
                // Each menu item takes 2 rows (item + spacing), starting with 1 row of spacing
                let relative_y = y - inner_y - 1; // -1 for initial spacing
                let item_index = (relative_y / 2) as usize;

                if item_index < menu.items.len() {
                    menu.selected = item_index;

                    // Check item type and handle accordingly
                    if let Some(item) = menu.items.get(menu.selected) {
                        match &item.item_type {
                            crate::menu::MenuItemType::Button(_) => {
                                return menu.select().cloned();
                            }
                            crate::menu::MenuItemType::Toggle { .. }
                            | crate::menu::MenuItemType::Cycle { .. }
                            | crate::menu::MenuItemType::Number { .. } => {
                                // Toggle/adjust on click
                                menu.adjust_right(settings);
                                return None;
                            }
                            crate::menu::MenuItemType::KeyBind { .. } => {
                                menu.start_rebind();
                                return None;
                            }
                            crate::menu::MenuItemType::TextInput { .. } => {
                                // Focus text input
                                return None;
                            }
                            crate::menu::MenuItemType::Label { .. } => {
                                // Labels are not interactive
                                return None;
                            }
                            _ => return None,
                        }
                    }
                }
            }
            None
        }
        MouseEventKind::Moved => {
            let x = mouse.column;
            let y = mouse.row;

            // Highlight on hover
            if x >= inner_x && x < inner_x + inner_width && y > inner_y {
                let relative_y = y - inner_y - 1;
                let item_index = (relative_y / 2) as usize;

                if item_index < menu.items.len() {
                    menu.selected = item_index;
                }
            }
            None
        }
        MouseEventKind::ScrollUp => {
            menu.move_up();
            None
        }
        MouseEventKind::ScrollDown => {
            menu.move_down();
            None
        }
        _ => None,
    }
}

/// Convert a KeyCode to a string for settings storage
fn key_to_string(code: KeyCode) -> String {
    match code {
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Modifier(m) => match m {
            crossterm::event::ModifierKeyCode::LeftShift | crossterm::event::ModifierKeyCode::RightShift => "Shift".to_string(),
            crossterm::event::ModifierKeyCode::LeftControl | crossterm::event::ModifierKeyCode::RightControl => "Ctrl".to_string(),
            crossterm::event::ModifierKeyCode::LeftAlt | crossterm::event::ModifierKeyCode::RightAlt => "Alt".to_string(),
            _ => "Unknown".to_string(),
        },
        _ => "Unknown".to_string(),
    }
}
