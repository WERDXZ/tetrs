//! Multiplayer networking using iroh for P2P connections
//!
//! Protocol:
//! 1. Host creates endpoint, generates ticket
//! 2. Guest connects with ticket
//! 3. Exchange seeds and start game together
//! 4. On piece lock: send board state + garbage
//! 5. On game over: send result

use crate::board::{Board, Cell, BOARD_HEIGHT, BOARD_WIDTH};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use iroh::{Endpoint, NodeAddr};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use tokio::io::AsyncReadExt;
use tracing::{debug, error, info};

/// Protocol identifier for our game
const GAME_ALPN: &[u8] = b"tetrs/versus/1";

/// Messages sent between players
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage {
    /// Initial handshake with player name
    Hello { name: String },
    /// Seed exchange for synchronized piece generation
    Seed { seed: u64 },
    /// Ready to start (sent after seed received)
    Ready,
    /// Countdown sync (3, 2, 1, go)
    Countdown { value: u8 },
    /// Board state update (sent on piece lock)
    BoardState {
        /// Flattened board cells as color indices
        cells: Vec<u8>,
        /// Current score
        score: u64,
        /// Lines cleared total
        lines: u32,
        /// Current level
        level: u32,
    },
    /// Garbage lines to add to opponent
    Garbage { lines: u8 },
    /// Player topped out / game over
    GameOver { final_score: u64 },
    /// Player won (opponent topped out)
    Victory,
    /// Request rematch
    RematchRequest,
    /// Accept rematch
    RematchAccept,
    /// Decline rematch / disconnect
    Disconnect,
}

/// Connection role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Host,
    Guest,
}

/// Multiplayer connection state
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Waiting for opponent to connect (host only)
    WaitingForOpponent { ticket: String },
    /// Connecting to host (guest only)
    Connecting,
    /// Connected, exchanging seeds
    Connected,
    /// In lobby, waiting for both players to ready up
    Lobby { we_ready: bool, they_ready: bool },
    /// Both players ready, countdown
    Countdown { value: u8 },
    /// Game in progress
    Playing,
    /// Game ended, showing results
    GameOver { we_won: bool },
}

/// Opponent's game state for display
#[derive(Debug, Clone)]
pub struct OpponentState {
    pub name: String,
    pub board: [[Cell; BOARD_WIDTH]; BOARD_HEIGHT],
    pub score: u64,
    pub lines: u32,
    pub level: u32,
    pub game_over: bool,
}

impl Default for OpponentState {
    fn default() -> Self {
        Self {
            name: "Opponent".to_string(),
            board: [[Cell::Empty; BOARD_WIDTH]; BOARD_HEIGHT],
            score: 0,
            lines: 0,
            level: 1,
            game_over: false,
        }
    }
}

impl OpponentState {
    /// Update from a BoardState message
    pub fn update_from_message(&mut self, cells: &[u8], score: u64, lines: u32, level: u32) {
        self.score = score;
        self.lines = lines;
        self.level = level;

        // Decode cells (0 = empty, 1-7 = piece colors)
        for row in 0..BOARD_HEIGHT {
            for col in 0..BOARD_WIDTH {
                let idx = row * BOARD_WIDTH + col;
                if idx < cells.len() {
                    self.board[row][col] = cell_from_index(cells[idx]);
                }
            }
        }
    }
}

/// Convert cell to index for network transmission
pub fn cell_to_index(cell: &Cell) -> u8 {
    match cell {
        Cell::Empty => 0,
        Cell::Filled(color) => {
            use ratatui::style::Color;
            match color {
                Color::Cyan => 1,    // I
                Color::Yellow => 2,  // O
                Color::Magenta => 3, // T
                Color::Green => 4,   // S
                Color::Red => 5,     // Z
                Color::Blue => 6,    // J
                _ => 7,              // L (orange) or other
            }
        }
    }
}

/// Convert index back to cell
pub fn cell_from_index(index: u8) -> Cell {
    use ratatui::style::Color;
    match index {
        0 => Cell::Empty,
        1 => Cell::Filled(Color::Cyan),
        2 => Cell::Filled(Color::Yellow),
        3 => Cell::Filled(Color::Magenta),
        4 => Cell::Filled(Color::Green),
        5 => Cell::Filled(Color::Red),
        6 => Cell::Filled(Color::Blue),
        _ => Cell::Filled(Color::Rgb(255, 165, 0)),
    }
}

/// Encode board state for transmission
pub fn encode_board(board: &Board) -> Vec<u8> {
    let mut cells = Vec::with_capacity(BOARD_HEIGHT * BOARD_WIDTH);
    for row in 0..BOARD_HEIGHT {
        for col in 0..BOARD_WIDTH {
            let cell = board.get(row as i32, col as i32).unwrap_or(Cell::Empty);
            cells.push(cell_to_index(&cell));
        }
    }
    cells
}

/// Channel message for communication between game loop and network task
#[derive(Debug)]
pub enum NetCommand {
    /// Send a message to the opponent
    Send(GameMessage),
    /// Disconnect
    Disconnect,
}

/// Events received from the network
#[derive(Debug)]
pub enum NetEvent {
    /// Connection established
    Connected { opponent_name: String },
    /// Received opponent's seed
    SeedReceived { seed: u64 },
    /// Opponent is ready
    OpponentReady,
    /// Countdown update
    Countdown { value: u8 },
    /// Opponent's board state updated
    BoardUpdate {
        cells: Vec<u8>,
        score: u64,
        lines: u32,
        level: u32,
    },
    /// Received garbage lines
    GarbageReceived { lines: u8 },
    /// Opponent game over
    OpponentGameOver { final_score: u64 },
    /// Connection lost
    Disconnected { reason: String },
    /// Error occurred
    Error { message: String },
}

/// Multiplayer session manager
pub struct MultiplayerSession {
    /// Our role (host or guest)
    pub role: Role,
    /// Current connection state
    pub state: ConnectionState,
    /// Opponent's state
    pub opponent: OpponentState,
    /// The seed for this game
    pub game_seed: u64,
    /// Pending garbage lines to add
    pub pending_garbage: u8,
    /// Countdown timer start (host only)
    countdown_start: Option<std::time::Instant>,
    /// Channel to send commands to network task
    cmd_tx: Option<mpsc::Sender<NetCommand>>,
    /// Channel to receive events from network task
    event_rx: Option<mpsc::Receiver<NetEvent>>,
}

impl MultiplayerSession {
    pub fn new(role: Role) -> Self {
        Self {
            role,
            state: ConnectionState::Disconnected,
            opponent: OpponentState::default(),
            game_seed: 0,
            pending_garbage: 0,
            countdown_start: None,
            cmd_tx: None,
            event_rx: None,
        }
    }

    /// Mark ourselves as ready in the lobby, returns true if countdown should start
    pub fn set_ready(&mut self) -> bool {
        if let ConnectionState::Lobby { we_ready, they_ready } = self.state {
            if we_ready {
                return false; // Already ready
            }
            self.state = ConnectionState::Lobby { we_ready: true, they_ready };
            self.send(GameMessage::Ready);

            // If both ready and we're host, start countdown
            if they_ready && self.role == Role::Host {
                self.start_countdown();
                return true;
            }
        }
        false
    }

    /// Mark opponent as ready, returns true if countdown should start
    pub fn set_opponent_ready(&mut self) -> bool {
        if let ConnectionState::Lobby { we_ready, .. } = self.state {
            self.state = ConnectionState::Lobby { we_ready, they_ready: true };

            // If both ready and we're host, start countdown
            if we_ready && self.role == Role::Host {
                self.start_countdown();
                return true;
            }
        }
        false
    }

    /// Start the countdown (host only)
    fn start_countdown(&mut self) {
        self.countdown_start = Some(std::time::Instant::now());
        self.state = ConnectionState::Countdown { value: 3 };
        self.send(GameMessage::Countdown { value: 3 });
    }

    /// Update countdown timer (host only), returns new value if changed
    pub fn update_countdown(&mut self) -> Option<u8> {
        if self.role != Role::Host {
            return None;
        }

        let ConnectionState::Countdown { value } = self.state else {
            return None;
        };

        let Some(start) = self.countdown_start else {
            return None;
        };

        let elapsed = start.elapsed().as_secs();
        let new_value = 3u8.saturating_sub(elapsed as u8);

        if new_value != value {
            if new_value == 0 {
                self.state = ConnectionState::Playing;
                self.countdown_start = None;
            } else {
                self.state = ConnectionState::Countdown { value: new_value };
            }
            self.send(GameMessage::Countdown { value: new_value });
            return Some(new_value);
        }
        None
    }

    /// Handle incoming countdown from host (guest only)
    pub fn receive_countdown(&mut self, value: u8) {
        if value == 0 {
            self.state = ConnectionState::Playing;
        } else {
            self.state = ConnectionState::Countdown { value };
        }
    }

    /// Check for incoming network events (non-blocking)
    pub fn poll_events(&mut self) -> Vec<NetEvent> {
        let mut events = Vec::new();
        if let Some(rx) = &mut self.event_rx {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }
        events
    }

    /// Send a message to the opponent
    pub fn send(&self, msg: GameMessage) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(NetCommand::Send(msg));
        }
    }

    /// Send our board state (from board and stats)
    pub fn send_board_state_raw(&self, board: &Board, score: u64, lines: u32, level: u32) {
        self.send(GameMessage::BoardState {
            cells: encode_board(board),
            score,
            lines,
            level,
        });
    }

    /// Send our board state (from game)
    pub fn send_board_state(&self, game: &crate::game::Game) {
        self.send_board_state_raw(
            &game.board,
            game.score.points,
            game.score.lines,
            game.score.level,
        );
    }

    /// Send disconnect message and cleanup
    pub fn send_disconnect(&mut self) {
        self.send(GameMessage::Disconnect);
        self.disconnect();
    }

    /// Send garbage to opponent
    pub fn send_garbage(&self, lines: u8) {
        if lines > 0 {
            self.send(GameMessage::Garbage { lines });
        }
    }

    /// Take pending garbage (resets to 0)
    pub fn take_pending_garbage(&mut self) -> u8 {
        let garbage = self.pending_garbage;
        self.pending_garbage = 0;
        garbage
    }

    /// Send game over
    pub fn send_game_over(&self, final_score: u64) {
        self.send(GameMessage::GameOver { final_score });
    }

    /// Disconnect
    pub fn disconnect(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(NetCommand::Disconnect);
        }
        self.state = ConnectionState::Disconnected;
        self.cmd_tx = None;
        self.event_rx = None;
    }

    /// Set the channels for network communication
    pub fn set_channels(
        &mut self,
        cmd_tx: mpsc::Sender<NetCommand>,
        event_rx: mpsc::Receiver<NetEvent>,
    ) {
        self.cmd_tx = Some(cmd_tx);
        self.event_rx = Some(event_rx);
    }
}

/// Calculate garbage lines to send based on lines cleared
/// Standard guideline:
/// - Single: 0 garbage
/// - Double: 1 garbage
/// - Triple: 2 garbage
/// - Tetris: 4 garbage
/// - T-spin single: 2 garbage
/// - T-spin double: 4 garbage
/// - T-spin triple: 6 garbage
/// - Back-to-back bonus: +1 garbage
/// - Combo bonus: +combo count
pub fn calculate_garbage(lines: u32, is_tspin: bool, combo: u32, back_to_back: bool) -> u8 {
    let base = if is_tspin {
        match lines {
            1 => 2,
            2 => 4,
            3 => 6,
            _ => 0,
        }
    } else {
        match lines {
            1 => 0,
            2 => 1,
            3 => 2,
            4 => 4,
            _ => 0,
        }
    };

    let b2b_bonus = if back_to_back && (lines == 4 || is_tspin) { 1 } else { 0 };
    let combo_bonus = if combo > 0 { combo.min(10) } else { 0 };

    (base + b2b_bonus + combo_bonus as u8).min(12) // Cap at 12 lines
}

/// Serialize a message to bytes with length prefix
fn encode_message(msg: &GameMessage) -> Vec<u8> {
    let json = serde_json::to_vec(msg).unwrap_or_default();
    let len = json.len() as u32;
    let mut data = len.to_be_bytes().to_vec();
    data.extend(json);
    data
}

/// Read a length-prefixed message from a stream
async fn read_message<R: AsyncReadExt + Unpin>(reader: &mut R) -> Option<GameMessage> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await.ok()?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > 1024 * 1024 {
        return None; // Reject messages > 1MB
    }

    let mut data = vec![0u8; len];
    reader.read_exact(&mut data).await.ok()?;
    serde_json::from_slice(&data).ok()
}

/// Start hosting a game, returns the ticket string for the guest to connect
/// This function blocks until the game is complete (does not spawn a task)
pub async fn start_hosting(
    ticket_tx: mpsc::Sender<Result<String, String>>,
    event_tx: mpsc::Sender<NetEvent>,
    cmd_rx: mpsc::Receiver<NetCommand>,
    our_name: String,
    our_seed: u64,
) {
    info!("Starting host with seed {}", our_seed);

    // Create endpoint
    let endpoint = match Endpoint::builder()
        .alpns(vec![GAME_ALPN.to_vec()])
        .bind()
        .await
    {
        Ok(ep) => ep,
        Err(e) => {
            error!("Failed to create endpoint: {}", e);
            let _ = ticket_tx.send(Err(format!("Failed to create endpoint: {}", e)));
            return;
        }
    };

    // Get our node address for the ticket
    let node_addr = match endpoint.node_addr().await {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to get node address: {}", e);
            let _ = ticket_tx.send(Err(format!("Failed to get node address: {}", e)));
            return;
        }
    };

    // Serialize NodeAddr to JSON then base64 encode for shorter ticket
    let json = match serde_json::to_string(&node_addr) {
        Ok(j) => j,
        Err(e) => {
            let _ = ticket_tx.send(Err(format!("Failed to serialize ticket: {}", e)));
            return;
        }
    };
    let ticket = URL_SAFE_NO_PAD.encode(json.as_bytes());
    info!("Generated ticket (len={})", ticket.len());

    // Send ticket back to main thread
    if ticket_tx.send(Ok(ticket)).is_err() {
        error!("Failed to send ticket back to main thread");
        return;
    }

    // Now handle the connection (blocking until done)
    info!("Host waiting for connection...");
    if let Err(e) = host_connection_loop(endpoint, event_tx.clone(), cmd_rx, our_name, our_seed).await {
        error!("Host connection loop error: {}", e);
        let _ = event_tx.send(NetEvent::Error { message: e });
    }
}

/// Host connection loop - waits for guest and handles communication
async fn host_connection_loop(
    endpoint: Endpoint,
    event_tx: mpsc::Sender<NetEvent>,
    cmd_rx: mpsc::Receiver<NetCommand>,
    our_name: String,
    our_seed: u64,
) -> Result<(), String> {
    info!("Host endpoint node_id={}, waiting for connections...", endpoint.node_id());

    // Wait for incoming connection
    info!("Host calling accept()...");
    let incoming = endpoint.accept().await
        .ok_or("No incoming connection")?;

    info!("Host received incoming connection, accepting...");
    let conn = incoming.await
        .map_err(|e| format!("Connection failed: {}", e))?;

    info!("Host connection established");

    // Open bidirectional stream
    info!("Host opening bidirectional stream...");
    let (mut send, mut recv) = conn.open_bi().await
        .map_err(|e| format!("Failed to open stream: {}", e))?;

    info!("Host stream opened successfully");

    // Send hello
    info!("Host sending hello as '{}'", our_name);
    let hello = encode_message(&GameMessage::Hello { name: our_name });
    send.write_all(&hello).await
        .map_err(|e| format!("Failed to send hello: {}", e))?;

    // Receive opponent's hello
    debug!("Host waiting for guest hello...");
    if let Some(GameMessage::Hello { name }) = read_message(&mut recv).await {
        info!("Host received hello from '{}'", name);
        let _ = event_tx.send(NetEvent::Connected { opponent_name: name });
    }

    // Send our seed
    info!("Host sending seed {}", our_seed);
    let seed_msg = encode_message(&GameMessage::Seed { seed: our_seed });
    send.write_all(&seed_msg).await
        .map_err(|e| format!("Failed to send seed: {}", e))?;

    // Main communication loop
    info!("Host entering message loop");
    message_loop(send, recv, event_tx, cmd_rx).await
}

/// Join a hosted game using a ticket
pub async fn join_game(
    ticket: &str,
    event_tx: mpsc::Sender<NetEvent>,
    cmd_rx: mpsc::Receiver<NetCommand>,
    our_name: String,
) -> Result<(), String> {
    info!("Guest joining game, ticket len={}", ticket.len());
    debug!("Guest ticket: {}", ticket.trim());

    // Decode base64 ticket then parse as JSON
    let json_bytes = URL_SAFE_NO_PAD.decode(ticket.trim())
        .map_err(|e| {
            error!("Invalid ticket encoding: {}", e);
            format!("Invalid ticket encoding: {}", e)
        })?;
    let json = String::from_utf8(json_bytes)
        .map_err(|e| format!("Invalid ticket data: {}", e))?;
    debug!("Guest decoded ticket JSON: {}", json);

    let node_addr: NodeAddr = serde_json::from_str(&json)
        .map_err(|e| format!("Invalid ticket: {}", e))?;

    let direct_addrs: Vec<_> = node_addr.direct_addresses().collect();
    info!("Guest parsed ticket, node_id={}, relay_url={:?}, direct_addrs={:?}",
        node_addr.node_id,
        node_addr.relay_url(),
        direct_addrs);

    // Create endpoint
    info!("Guest creating endpoint...");
    let endpoint = Endpoint::builder()
        .bind()
        .await
        .map_err(|e| format!("Failed to create endpoint: {}", e))?;

    info!("Guest endpoint created, our node_id={}", endpoint.node_id());

    // Connect to host
    info!("Guest connecting to host node_id={}...", node_addr.node_id);
    let conn = endpoint.connect(node_addr.clone(), GAME_ALPN).await
        .map_err(|e| {
            error!("Failed to connect to {}: {}", node_addr.node_id, e);
            format!("Failed to connect: {}", e)
        })?;

    info!("Guest connected! Waiting for host to open stream...");
    // Accept bidirectional stream from host
    let (mut send, mut recv) = conn.accept_bi().await
        .map_err(|e| format!("Failed to accept stream: {}", e))?;

    // Receive host's hello
    debug!("Guest waiting for host hello...");
    if let Some(GameMessage::Hello { name }) = read_message(&mut recv).await {
        info!("Guest received hello from '{}'", name);
        let _ = event_tx.send(NetEvent::Connected { opponent_name: name });
    } else {
        error!("Failed to receive hello from host");
        return Err("Failed to receive hello".to_string());
    }

    // Send our hello
    info!("Guest sending hello as '{}'", our_name);
    let hello = encode_message(&GameMessage::Hello { name: our_name });
    send.write_all(&hello).await
        .map_err(|e| format!("Failed to send hello: {}", e))?;

    // Receive seed from host
    debug!("Guest waiting for seed...");
    if let Some(GameMessage::Seed { seed }) = read_message(&mut recv).await {
        info!("Guest received seed {}", seed);
        let _ = event_tx.send(NetEvent::SeedReceived { seed });
    }

    // Main communication loop (lobby ready is handled by game logic)
    info!("Guest entering message loop");
    message_loop(send, recv, event_tx, cmd_rx).await
}

/// Main message loop for both host and guest
async fn message_loop(
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
    event_tx: mpsc::Sender<NetEvent>,
    cmd_rx: mpsc::Receiver<NetCommand>,
) -> Result<(), String> {
    use tokio::time::{interval, Duration};

    debug!("Message loop started");
    let mut poll_interval = interval(Duration::from_millis(16)); // ~60fps

    loop {
        tokio::select! {
            // Poll for outgoing commands (non-blocking)
            _ = poll_interval.tick() => {
                // Check for commands without blocking
                match cmd_rx.try_recv() {
                    Ok(NetCommand::Send(msg)) => {
                        let data = encode_message(&msg);
                        if send.write_all(&data).await.is_err() {
                            let _ = event_tx.send(NetEvent::Disconnected {
                                reason: "Write failed".to_string()
                            });
                            break;
                        }
                    }
                    Ok(NetCommand::Disconnect) => {
                        let _ = send.write_all(&encode_message(&GameMessage::Disconnect)).await;
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        let _ = send.write_all(&encode_message(&GameMessage::Disconnect)).await;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        // No commands, continue
                    }
                }
            }
            // Handle incoming messages
            msg = read_message(&mut recv) => {
                match msg {
                    Some(GameMessage::Hello { name }) => {
                        let _ = event_tx.send(NetEvent::Connected { opponent_name: name });
                    }
                    Some(GameMessage::Seed { seed }) => {
                        let _ = event_tx.send(NetEvent::SeedReceived { seed });
                    }
                    Some(GameMessage::Ready) => {
                        let _ = event_tx.send(NetEvent::OpponentReady);
                    }
                    Some(GameMessage::Countdown { value }) => {
                        let _ = event_tx.send(NetEvent::Countdown { value });
                    }
                    Some(GameMessage::BoardState { cells, score, lines, level }) => {
                        let _ = event_tx.send(NetEvent::BoardUpdate { cells, score, lines, level });
                    }
                    Some(GameMessage::Garbage { lines }) => {
                        let _ = event_tx.send(NetEvent::GarbageReceived { lines });
                    }
                    Some(GameMessage::GameOver { final_score }) => {
                        let _ = event_tx.send(NetEvent::OpponentGameOver { final_score });
                    }
                    Some(GameMessage::Disconnect) | None => {
                        let _ = event_tx.send(NetEvent::Disconnected {
                            reason: "Connection closed".to_string()
                        });
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

/// Spawn hosting using the provided runtime handle
pub fn spawn_host(
    handle: &tokio::runtime::Handle,
    our_name: String,
    our_seed: u64,
) -> Result<(String, mpsc::Sender<NetCommand>, mpsc::Receiver<NetEvent>), String> {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let (ticket_tx, ticket_rx) = mpsc::channel();

    handle.spawn(async move {
        start_hosting(ticket_tx, event_tx, cmd_rx, our_name, our_seed).await;
    });

    // Wait for ticket from the task
    let ticket = ticket_rx.recv().map_err(|_| "Host task died".to_string())??;
    Ok((ticket, cmd_tx, event_rx))
}

/// Spawn joining using the provided runtime handle
pub fn spawn_join(
    handle: &tokio::runtime::Handle,
    ticket: String,
    our_name: String,
) -> Result<(mpsc::Sender<NetCommand>, mpsc::Receiver<NetEvent>), String> {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    handle.spawn(async move {
        if let Err(e) = join_game(&ticket, event_tx.clone(), cmd_rx, our_name).await {
            let _ = event_tx.send(NetEvent::Error { message: e });
        }
    });

    Ok((cmd_tx, event_rx))
}
