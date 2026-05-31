//! Deterministic Lockstep Networking — Phase 12.
//!
//! Ensures all clients simulate the exact same game state by:
//! 1. Collecting local inputs for a configurable delay (input buffer).
//! 2. Broadcasting inputs to all peers.
//! 3. Only advancing the simulation once every peer's input for tick N
//!    has been received (or a timeout grace period has elapsed).
//!
//! This pairs naturally with [`TickScheduler`](crate::system::TickScheduler),
//! which already provides fixed-timestep determinism.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use crate::net::transport::{ConnectionManager, Packet, UdpError};

// ──────────────────────────────────────────────────────────────
// Input Representation
// ──────────────────────────────────────────────────────────────

/// A single player's input for one simulation tick.
///
/// This is intentionally compact — it will be sent over the network
/// every tick. Game-specific input (key presses, stick axes, mouse
/// position) should be encoded into the `payload` byte vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInput {
    /// The simulation tick this input is for.
    pub tick: u64,
    /// The player / connection this input came from.
    pub player_id: u64,
    /// Opaque input payload (bit-packed keys, axes, etc.).
    pub payload: Vec<u8>,
}

impl PlayerInput {
    /// Serialize to a compact wire format:
    /// `[tick:8][player_id:8][len:2][payload:len]`
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(18 + self.payload.len());
        buf.extend_from_slice(&self.tick.to_le_bytes());
        buf.extend_from_slice(&self.player_id.to_le_bytes());
        buf.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Deserialize from bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 18 {
            return None;
        }
        let tick = u64::from_le_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]);
        let player_id = u64::from_le_bytes([
            buf[8], buf[9], buf[10], buf[11], buf[12], buf[13], buf[14], buf[15],
        ]);
        let len = u16::from_le_bytes([buf[16], buf[17]]) as usize;
        if buf.len() < 18 + len {
            return None;
        }
        let payload = buf[18..18 + len].to_vec();
        Some(PlayerInput {
            tick,
            player_id,
            payload,
        })
    }
}

// ──────────────────────────────────────────────────────────────
// Lockstep Configuration
// ──────────────────────────────────────────────────────────────

/// Configuration for the lockstep synchronizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockstepConfig {
    /// Number of ticks to delay execution (input buffer size).
    /// Typical values: 2–4 for LAN, 4–8 for internet.
    pub input_delay_ticks: u32,
    /// Ticks to wait for a missing input before declaring a desync
    /// and using a default / predicted input.
    pub timeout_ticks: u32,
    /// Whether this peer is the authority (host) that can advance
    /// the simulation even if some clients are slow.
    pub is_authority: bool,
}

impl LockstepConfig {
    pub fn lan_host() -> Self {
        LockstepConfig {
            input_delay_ticks: 2,
            timeout_ticks: 4,
            is_authority: true,
        }
    }

    pub fn lan_client() -> Self {
        LockstepConfig {
            input_delay_ticks: 2,
            timeout_ticks: 4,
            is_authority: false,
        }
    }

    pub fn internet_host() -> Self {
        LockstepConfig {
            input_delay_ticks: 6,
            timeout_ticks: 10,
            is_authority: true,
        }
    }

    pub fn internet_client() -> Self {
        LockstepConfig {
            input_delay_ticks: 6,
            timeout_ticks: 10,
            is_authority: false,
        }
    }
}

impl Default for LockstepConfig {
    fn default() -> Self {
        LockstepConfig::lan_client()
    }
}

// ──────────────────────────────────────────────────────────────
// Lockstep Synchronizer
// ──────────────────────────────────────────────────────────────

/// Manages deterministic lockstep simulation across multiple peers.
///
/// # Usage
///
/// 1. Create a `LockstepSync` with your player ID and config.
/// 2. Every local frame, call `record_local_input(tick, payload)`.
/// 3. Call `broadcast_inputs(conn_mgr)` to send buffered inputs to peers.
/// 4. Call `receive_remote_inputs(conn_mgr)` to ingest incoming packets.
/// 5. Query `is_ready_to_advance(target_tick)` — if true, call `advance_tick()`
///    and then run your `TickScheduler::tick()` with the collected inputs.
#[derive(Debug)]
pub struct LockstepSync {
    pub local_player_id: u64,
    pub config: LockstepConfig,

    /// Inputs we have recorded locally, keyed by tick.
    local_inputs: HashMap<u64, PlayerInput>,

    /// Inputs received from remote players, keyed by (player_id, tick).
    remote_inputs: HashMap<(u64, u64), PlayerInput>,

    /// The highest tick we have ever been ready to simulate.
    current_tick: u64,

    /// Ticks we have already advanced past.
    confirmed_tick: u64,

    /// Missing inputs by tick (used for timeout / desync detection).
    missing_log: VecDeque<(u64, Instant, Vec<u64>)>,

    /// Default input payload used when a peer times out.
    pub default_input: Vec<u8>,

    /// Desync counter (incremented every time we timeout-advance).
    pub desync_count: u64,
}

impl LockstepSync {
    pub fn new(local_player_id: u64, config: LockstepConfig) -> Self {
        LockstepSync {
            local_player_id,
            config,
            local_inputs: HashMap::new(),
            remote_inputs: HashMap::new(),
            current_tick: 0,
            confirmed_tick: 0,
            missing_log: VecDeque::new(),
            default_input: Vec::new(),
            desync_count: 0,
        }
    }

    /// Record local input for a specific simulation tick.
    ///
    /// The tick should be `current_tick + input_delay_ticks` into the
    /// future so that all peers have time to receive it.
    pub fn record_local_input(&mut self, tick: u64, payload: impl Into<Vec<u8>>) {
        let input = PlayerInput {
            tick,
            player_id: self.local_player_id,
            payload: payload.into(),
        };
        self.local_inputs.insert(tick, input);
    }

    /// Get the local input for a tick (if recorded).
    pub fn local_input(&self, tick: u64) -> Option<&PlayerInput> {
        self.local_inputs.get(&tick)
    }

    /// Get (or synthesize) the input for a given player at a given tick.
    ///
    /// If the real input hasn't arrived and we've passed the timeout,
    /// returns a synthetic input with the default payload.
    pub fn input_for(&self, player_id: u64, tick: u64) -> PlayerInput {
        if player_id == self.local_player_id {
            return self
                .local_inputs
                .get(&tick)
                .cloned()
                .unwrap_or(PlayerInput {
                    tick,
                    player_id,
                    payload: self.default_input.clone(),
                });
        }
        self.remote_inputs
            .get(&(player_id, tick))
            .cloned()
            .unwrap_or(PlayerInput {
                tick,
                player_id,
                payload: self.default_input.clone(),
            })
    }

    /// Collect all inputs for every known player at a given tick.
    pub fn collect_inputs_for_tick(&self, tick: u64, player_ids: &[u64]) -> Vec<PlayerInput> {
        player_ids
            .iter()
            .map(|&id| self.input_for(id, tick))
            .collect()
    }

    /// Check if every required player's input for `tick` has been received.
    ///
    /// If the authority flag is set and the tick is within the timeout
    /// window, the host may return `true` anyway to prevent one slow
    /// client from stalling everyone.
    pub fn has_all_inputs(&self, tick: u64, player_ids: &[u64]) -> bool {
        for &id in player_ids {
            if id == self.local_player_id {
                if !self.local_inputs.contains_key(&tick) {
                    return false;
                }
            } else if !self.remote_inputs.contains_key(&(id, tick)) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if the simulation is allowed to advance to `target_tick`.
    ///
    /// This is the core gatekeeper: the game loop should only call
    /// `TickScheduler::tick()` when this returns true.
    pub fn is_ready_to_advance(&self, target_tick: u64, player_ids: &[u64]) -> bool {
        // We must be past the input delay window.
        if target_tick < self.config.input_delay_ticks as u64 {
            return false;
        }

        // Must have all inputs (or timeout has expired).
        if self.has_all_inputs(target_tick, player_ids) {
            return true;
        }

        // Authority may force-advance after timeout.
        if self.config.is_authority {
            let elapsed = target_tick.saturating_sub(self.confirmed_tick);
            elapsed >= self.config.timeout_ticks as u64
        } else {
            false
        }
    }

    /// Advance the confirmed tick counter and prune old inputs.
    pub fn advance_tick(&mut self) {
        self.current_tick += 1;
        self.confirmed_tick = self.current_tick;

        // Prune inputs older than confirmed_tick + input_delay + timeout
        let prune_before = self
            .confirmed_tick
            .saturating_sub((self.config.input_delay_ticks + self.config.timeout_ticks) as u64);

        self.local_inputs.retain(|&tick, _| tick >= prune_before);
        self.remote_inputs
            .retain(|&(_, tick), _| tick >= prune_before);
        self.missing_log
            .retain(|(tick, _, _)| *tick >= prune_before);
    }

    /// Mark that we had to advance without full inputs (desync event).
    pub fn record_desync(&mut self, tick: u64, missing_players: Vec<u64>) {
        self.desync_count += 1;
        self.missing_log
            .push_back((tick, Instant::now(), missing_players));
    }

    // ──────────────────────────────────────────────────────────
    // Network integration
    // ──────────────────────────────────────────────────────────

    /// Send all local inputs that haven't been sent yet to every peer.
    pub fn broadcast_inputs(&mut self, conn_mgr: &mut ConnectionManager) -> Result<(), UdpError> {
        // Serialize all local inputs into one packet per tick
        for input in self.local_inputs.values() {
            let payload = input.encode();
            let packet = Packet::new(3, payload); // channel 3 = lockstep inputs
            for id in conn_mgr.active_connections() {
                let _ = conn_mgr.send(id, &packet);
            }
        }
        Ok(())
    }

    /// Poll the connection manager for incoming input packets.
    pub fn receive_remote_inputs(
        &mut self,
        conn_mgr: &mut ConnectionManager,
    ) -> Result<(), UdpError> {
        while let Some((maybe_id, _addr, packet)) = conn_mgr.recv()? {
            if packet.header.channel == 3 {
                if let Some(input) = PlayerInput::decode(&packet.payload) {
                    // Map connection ID to player ID (simplified)
                    let player_id = maybe_id.map(|c| c.0).unwrap_or(input.player_id);
                    self.remote_inputs.insert((player_id, input.tick), input);
                }
            }
            // Ignore non-lockstep packets here
        }
        Ok(())
    }

    // ──────────────────────────────────────────────────────────
    // Diagnostics
    // ──────────────────────────────────────────────────────────

    /// Number of ticks worth of inputs currently buffered.
    pub fn buffered_tick_count(&self) -> usize {
        self.local_inputs.len()
    }

    /// Number of remote inputs currently stored.
    pub fn remote_input_count(&self) -> usize {
        self.remote_inputs.len()
    }

    /// Current confirmed simulation tick.
    pub fn confirmed_tick(&self) -> u64 {
        self.confirmed_tick
    }

    /// Whether any inputs are missing right now.
    pub fn has_missing_inputs(&self, player_ids: &[u64]) -> bool {
        !self.has_all_inputs(self.current_tick + 1, player_ids)
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: PlayerInput encode/decode roundtrip.
    #[test]
    fn input_encode_roundtrip() {
        let input = PlayerInput {
            tick: 42,
            player_id: 7,
            payload: vec![1, 2, 3, 4, 5],
        };
        let encoded = input.encode();
        let decoded = PlayerInput::decode(&encoded).unwrap();
        assert_eq!(input, decoded);
    }

    // Test 2: PlayerInput decode too short.
    #[test]
    fn input_decode_too_short() {
        assert!(PlayerInput::decode(&[0; 5]).is_none());
    }

    // Test 3: Record and retrieve local input.
    #[test]
    fn local_input_record() {
        let mut sync = LockstepSync::new(1, LockstepConfig::default());
        sync.record_local_input(5, vec![0xAA]);
        assert_eq!(sync.local_input(5).unwrap().payload, vec![0xAA]);
        assert!(sync.local_input(6).is_none());
    }

    // Test 4: has_all_inputs with local only.
    #[test]
    fn has_all_inputs_local() {
        let mut sync = LockstepSync::new(1, LockstepConfig::default());
        sync.record_local_input(10, vec![0]);
        assert!(sync.has_all_inputs(10, &[1]));
        assert!(!sync.has_all_inputs(10, &[1, 2]));
    }

    // Test 5: is_ready_to_advance respects input delay.
    #[test]
    fn ready_respects_delay() {
        let sync = LockstepSync::new(1, LockstepConfig::lan_client());
        // tick 0 is before input_delay_ticks (2)
        assert!(!sync.is_ready_to_advance(0, &[1]));
        assert!(!sync.is_ready_to_advance(1, &[1]));
    }

    // Test 6: Authority can force-advance after timeout.
    #[test]
    fn authority_force_advance() {
        let mut sync = LockstepSync::new(1, LockstepConfig::lan_host());
        sync.confirmed_tick = 0;
        sync.current_tick = 0;
        // No inputs recorded, but host is authority and timeout=4
        // target_tick=10, confirmed=0 -> elapsed=10 >= timeout=4
        assert!(sync.is_ready_to_advance(10, &[1, 2]));
    }

    // Test 7: Non-authority cannot force-advance.
    #[test]
    fn client_cannot_force_advance() {
        let sync = LockstepSync::new(2, LockstepConfig::lan_client());
        // tick 10 is past delay, but we don't have inputs and we're not authority
        assert!(!sync.is_ready_to_advance(10, &[1, 2]));
    }

    // Test 8: Input pruning on advance.
    #[test]
    fn advance_prunes_old_inputs() {
        let mut sync = LockstepSync::new(1, LockstepConfig::lan_client());
        sync.record_local_input(1, vec![0]);
        sync.record_local_input(2, vec![0]);
        sync.current_tick = 100;
        sync.confirmed_tick = 100;
        sync.advance_tick();
        // Prune threshold = 100 - (2 + 4) = 94, so ticks 1 and 2 should be gone
        assert!(sync.local_inputs.is_empty());
    }

    // Test 9: collect_inputs_for_tick.
    #[test]
    fn collect_inputs() {
        let mut sync = LockstepSync::new(1, LockstepConfig::default());
        sync.record_local_input(5, vec![0x11]);
        sync.remote_inputs.insert(
            (2, 5),
            PlayerInput {
                tick: 5,
                player_id: 2,
                payload: vec![0x22],
            },
        );
        let inputs = sync.collect_inputs_for_tick(5, &[1, 2]);
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].payload, vec![0x11]);
        assert_eq!(inputs[1].payload, vec![0x22]);
    }

    // Test 10: Default input fallback.
    #[test]
    fn default_input_fallback() {
        let sync = LockstepSync::new(1, LockstepConfig::default());
        let input = sync.input_for(1, 99); // player_id=1, tick=99
        assert_eq!(input.tick, 99);
        assert_eq!(input.payload, Vec::<u8>::new());
    }

    // Test 11: record_desync increments counter.
    #[test]
    fn desync_counter() {
        let mut sync = LockstepSync::new(1, LockstepConfig::default());
        sync.record_desync(5, vec![2]);
        sync.record_desync(6, vec![2, 3]);
        assert_eq!(sync.desync_count, 2);
    }

    // Test 12: Config presets.
    #[test]
    fn config_presets() {
        let lan = LockstepConfig::lan_host();
        assert_eq!(lan.input_delay_ticks, 2);
        assert!(lan.is_authority);

        let internet = LockstepConfig::internet_client();
        assert_eq!(internet.input_delay_ticks, 6);
        assert!(!internet.is_authority);
    }
}
