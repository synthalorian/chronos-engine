//! Lag Compensation — Phase 12 Networking.
//!
//! Implements client-side prediction and server reconciliation for
//! responsive multiplayer gameplay.
//!
//! # How it works
//!
//! 1. **Client-side prediction**: The client applies local inputs
//!    immediately without waiting for the server, storing predicted
//!    entity states in a ring buffer.
//!
//! 2. **Server reconciliation**: When an authoritative server snapshot
//!    arrives, the client finds the corresponding tick in its prediction
//!    history, corrects any divergence, and re-simulates from that point.
//!
//! 3. **Error smoothing**: Positional corrections are blended over a
//!    short window (configurable `smoothing_ms`) to avoid jarring teleports.
//!
//! This module pairs with [`TickScheduler`](crate::system::TickScheduler)
//! for deterministic simulation and with [`RollbackManager`] for when
//! rollback netcode is active.

use std::collections::HashMap;

// ──────────────────────────────────────────────────────────────
// Entity State
// ──────────────────────────────────────────────────────────────

/// The state of a single entity at a specific tick.
///
/// In a production engine this would include all synchronised
/// component data (transform, velocity, animation state, health).
/// Here we store the minimal position + velocity for demonstration.
pub const ENTITY_STATE_WIRE_SIZE: usize = 8  + 4 + 4 + 4 + 4 + 8; // = 32

/// The state of a single entity at a specific tick.
///
/// In a production engine this would include all synchronised
/// component data (transform, velocity, animation state, health).
/// Here we store the minimal position + velocity for demonstration.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityState {
    pub entity_id: u64,
    pub position_x: f32,
    pub position_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    /// Generic bitmask of changed components (1 << ComponentIndex).
    pub dirty_mask: u64,
}

impl EntityState {
    pub fn new(entity_id: u64) -> Self {
        EntityState {
            entity_id,
            position_x: 0.0,
            position_y: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            dirty_mask: 0,
        }
    }

    /// Serialize to a compact byte representation.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(ENTITY_STATE_WIRE_SIZE);
        buf.extend_from_slice(&self.entity_id.to_le_bytes());
        buf.extend_from_slice(&self.position_x.to_le_bytes());
        buf.extend_from_slice(&self.position_y.to_le_bytes());
        buf.extend_from_slice(&self.velocity_x.to_le_bytes());
        buf.extend_from_slice(&self.velocity_y.to_le_bytes());
        buf.extend_from_slice(&self.dirty_mask.to_le_bytes());
        buf
    }

    /// Deserialize from bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < ENTITY_STATE_WIRE_SIZE {
            return None;
        }
        let entity_id = u64::from_le_bytes(buf[0..8].try_into().ok()?);
        let position_x = f32::from_le_bytes(buf[8..12].try_into().ok()?);
        let position_y = f32::from_le_bytes(buf[12..16].try_into().ok()?);
        let velocity_x = f32::from_le_bytes(buf[16..20].try_into().ok()?);
        let velocity_y = f32::from_le_bytes(buf[20..24].try_into().ok()?);
        let dirty_mask = u64::from_le_bytes(buf[24..32].try_into().ok()?);
        Some(EntityState {
            entity_id,
            position_x,
            position_y,
            velocity_x,
            velocity_y,
            dirty_mask,
        })
    }
}

// ──────────────────────────────────────────────────────────────
// Server Snapshot
// ──────────────────────────────────────────────────────────────

/// An authoritative world snapshot received from the server.
///
/// Contains the server's view of entity state at a particular tick.
#[derive(Debug, Clone)]
pub struct ServerSnapshot {
    /// The simulation tick this snapshot corresponds to.
    pub tick: u64,
    /// Entity states included in this snapshot.
    pub entities: Vec<EntityState>,
}

impl ServerSnapshot {
    pub fn new(tick: u64) -> Self {
        ServerSnapshot {
            tick,
            entities: Vec::new(),
        }
    }

    /// Serialize to bytes (for sending over the network).
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + 2 + self.entities.len() * 36);
        buf.extend_from_slice(&self.tick.to_le_bytes());
        buf.extend_from_slice(&(self.entities.len() as u16).to_le_bytes());
        for entity in &self.entities {
            buf.extend_from_slice(&entity.encode());
        }
        buf
    }

    /// Deserialize from bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 10 {
            return None;
        }
        let tick = u64::from_le_bytes(buf[0..8].try_into().ok()?);
        let count = u16::from_le_bytes(buf[8..10].try_into().ok()?) as usize;
        let mut off = 10;
        let mut entities = Vec::with_capacity(count);
        for _ in 0..count {
            if off + ENTITY_STATE_WIRE_SIZE > buf.len() {
                return None;
            }
            let state = EntityState::decode(&buf[off..off + ENTITY_STATE_WIRE_SIZE])?;
            entities.push(state);
            off += ENTITY_STATE_WIRE_SIZE;
        }
        Some(ServerSnapshot { tick, entities })
    }
}

// ──────────────────────────────────────────────────────────────
// Predicted State Ring Buffer
// ──────────────────────────────────────────────────────────────

/// A single entry in the client's prediction history.
#[derive(Debug, Clone)]
struct PredictedEntry {
    tick: u64,
    local_entity_states: HashMap<u64, EntityState>,
    /// Whether this state has been confirmed by a server snapshot.
    confirmed: bool,
}

/// Ring buffer storing the client's predicted states.
#[derive(Debug)]
struct PredictedHistory {
    entries: Vec<PredictedEntry>,
    capacity: usize,
}

impl PredictedHistory {
    fn new(capacity: usize) -> Self {
        PredictedHistory {
            entries: Vec::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, tick: u64, states: HashMap<u64, EntityState>) {
        if self.entries.len() == self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(PredictedEntry {
            tick,
            local_entity_states: states,
            confirmed: false,
        });
    }

    fn find_mut(&mut self, tick: u64) -> Option<&mut PredictedEntry> {
        self.entries.iter_mut().find(|e| e.tick == tick)
    }

    fn find(&self, tick: u64) -> Option<&PredictedEntry> {
        self.entries.iter().find(|e| e.tick == tick)
    }

    #[allow(dead_code)]
    fn oldest_tick(&self) -> Option<u64> {
        self.entries.first().map(|e| e.tick)
    }

    fn prune_before(&mut self, tick: u64) {
        self.entries.retain(|e| e.tick >= tick);
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    #[allow(dead_code)]
    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ──────────────────────────────────────────────────────────────
// Lag Compensation Configuration
// ──────────────────────────────────────────────────────────────

/// Configuration for the lag compensation system.
#[derive(Debug, Clone, Copy)]
pub struct LagCompensationConfig {
    /// Maximum number of predicted ticks to store.
    pub history_size: usize,
    /// How many ticks of extrapolation before reverting to
    /// server state (avoids runaway prediction on disconnect).
    pub max_prediction_ticks: u32,
    /// Smoothing duration in milliseconds for position corrections.
    pub smoothing_ms: u32,
    /// Whether this client is the server (no prediction needed).
    pub is_server: bool,
}

impl Default for LagCompensationConfig {
    fn default() -> Self {
        LagCompensationConfig {
            history_size: 128,
            max_prediction_ticks: 8,
            smoothing_ms: 50,
            is_server: false,
        }
    }
}

impl LagCompensationConfig {
    pub fn server() -> Self {
        LagCompensationConfig {
            is_server: true,
            ..Default::default()
        }
    }

    pub fn client() -> Self {
        LagCompensationConfig {
            is_server: false,
            ..Default::default()
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Reconciliation Error
// ──────────────────────────────────────────────────────────────

/// The result of reconciling a predicted state with a server snapshot.
#[derive(Debug, Clone)]
pub struct ReconciliationCorrection {
    pub entity_id: u64,
    pub predicted_x: f32,
    pub predicted_y: f32,
    pub server_x: f32,
    pub server_y: f32,
    pub error_distance: f32,
}

impl ReconciliationCorrection {
    /// Smoothing factor (0..1) for blending towards server state.
    pub fn smoothing_factor(&self, smoothing_ms: f32, dt_ms: f32) -> f32 {
        if smoothing_ms <= 0.0 {
            return 1.0;
        }
        (dt_ms / smoothing_ms).clamp(0.0, 1.0)
    }
}

// ──────────────────────────────────────────────────────────────
// Lag Compensation Engine
// ──────────────────────────────────────────────────────────────

/// The main lag compensation engine.
///
/// Manages client-side prediction history and reconciles with
/// server snapshots when they arrive.
///
/// # Usage (client)
///
/// 1. Each tick, call `predict_tick()` to store the current local state.
/// 2. When a [`ServerSnapshot`] arrives, call `reconcile()` — this
///    returns corrections that should be applied to local entities.
/// 3. Optionally query `smoothed_correction()` for per-frame blending.
///
/// # Usage (server)
///
/// 1. After each simulation tick, call `create_snapshot()` to build
///    an authoritative snapshot to send to clients.
/// 2. The server does not perform prediction.
#[derive(Debug)]
pub struct LagCompensation {
    pub config: LagCompensationConfig,
    /// Predicted state history (client-side).
    history: PredictedHistory,
    /// Corrections from the most recent reconciliation.
    pending_corrections: HashMap<u64, ReconciliationCorrection>,
    /// Tick of the last server snapshot we reconciled with.
    last_confirmed_tick: u64,
    /// Total number of reconciliations performed.
    pub reconciliation_count: u64,
    /// Total accumulated error (sum of distances) across corrections.
    pub total_error_distance: f32,
}

impl LagCompensation {
    pub fn new(config: LagCompensationConfig) -> Self {
        LagCompensation {
            config,
            history: PredictedHistory::new(config.history_size),
            pending_corrections: HashMap::new(),
            last_confirmed_tick: 0,
            reconciliation_count: 0,
            total_error_distance: 0.0,
        }
    }

    /// Store the current predicted state for a tick.
    ///
    /// Call this **before** applying the next tick's inputs, with the
    /// state that resulted from the current tick's simulation.
    pub fn predict_tick(&mut self, tick: u64, entity_states: HashMap<u64, EntityState>) {
        if self.config.is_server {
            return; // Servers don't predict.
        }
        self.history.push(tick, entity_states);
    }

    /// Reconcile the prediction history with an authoritative server snapshot.
    ///
    /// Returns a list of corrections that should be applied to bring
    /// local entities in line with the server's view.
    pub fn reconcile(&mut self, snapshot: &ServerSnapshot) -> Vec<ReconciliationCorrection> {
        if self.config.is_server || snapshot.tick <= self.last_confirmed_tick {
            return Vec::new();
        }

        let mut corrections = Vec::new();

        // Mark the snapshot's tick as confirmed.
        self.last_confirmed_tick = snapshot.tick;
        if let Some(entry) = self.history.find_mut(snapshot.tick) {
            entry.confirmed = true;
        }

        // Compare each entity in the snapshot with our predicted state.
        for server_entity in &snapshot.entities {
            let predicted = self
                .history
                .find(snapshot.tick)
                .and_then(|entry| entry.local_entity_states.get(&server_entity.entity_id));

            if let Some(predicted) = predicted {
                let dx = predicted.position_x - server_entity.position_x;
                let dy = predicted.position_y - server_entity.position_y;
                let error_distance = (dx * dx + dy * dy).sqrt();

                if error_distance > 0.001 {
                    let correction = ReconciliationCorrection {
                        entity_id: server_entity.entity_id,
                        predicted_x: predicted.position_x,
                        predicted_y: predicted.position_y,
                        server_x: server_entity.position_x,
                        server_y: server_entity.position_y,
                        error_distance,
                    };
                    corrections.push(correction);
                    self.total_error_distance += error_distance;
                }
            }
        }

        self.pending_corrections.clear();
        for corr in &corrections {
            self.pending_corrections.insert(corr.entity_id, corr.clone());
        }

        // Prune history before the confirmed tick.
        self.history.prune_before(snapshot.tick);

        self.reconciliation_count += 1;
        corrections
    }

    /// Get the pending correction for a specific entity (if any).
    ///
    /// Apply the correction by blending the entity's current position
    /// toward the server position using `smoothing_factor()`.
    pub fn correction_for(&self, entity_id: u64) -> Option<&ReconciliationCorrection> {
        self.pending_corrections.get(&entity_id)
    }

    /// Consume and return all pending corrections.
    ///
    /// Call this after applying corrections each frame.
    pub fn drain_corrections(&mut self) -> Vec<ReconciliationCorrection> {
        let corrections: Vec<_> = self.pending_corrections.drain().map(|(_, v)| v).collect();
        corrections
    }

    /// Create a server snapshot from the current local entity states.
    ///
    /// Call this on the server after each simulation tick to build
    /// the snapshot that will be sent to all clients.
    pub fn create_snapshot(
        &self,
        tick: u64,
        entities: Vec<EntityState>,
    ) -> ServerSnapshot {
        ServerSnapshot {
            tick,
            entities,
        }
    }

    /// The tick of the last confirmed server snapshot.
    pub fn last_confirmed_tick(&self) -> u64 {
        self.last_confirmed_tick
    }

    /// How many ticks ahead we are predicting (client only).
    pub fn prediction_delta(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.last_confirmed_tick)
    }

    /// Whether prediction is within safe bounds.
    pub fn is_prediction_safe(&self, current_tick: u64) -> bool {
        self.prediction_delta(current_tick) <= self.config.max_prediction_ticks as u64
    }

    /// Number of stored predicted states.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Clear all prediction state (e.g., on match restart).
    pub fn reset(&mut self) {
        self.history = PredictedHistory::new(self.config.history_size);
        self.pending_corrections.clear();
        self.last_confirmed_tick = 0;
        self.reconciliation_count = 0;
        self.total_error_distance = 0.0;
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: EntityState encode/decode roundtrip.
    #[test]
    fn entity_state_roundtrip() {
        let state = EntityState {
            entity_id: 42,
            position_x: 105.5,
            position_y: -32.1,
            velocity_x: 1.5,
            velocity_y: 0.0,
            dirty_mask: 0b0011,
        };
        let encoded = state.encode();
        let decoded = EntityState::decode(&encoded).unwrap();
        assert!((decoded.position_x - state.position_x).abs() < 0.001);
        assert!((decoded.velocity_y - state.velocity_y).abs() < 0.001);
        assert_eq!(decoded.dirty_mask, state.dirty_mask);
    }

    // Test 2: ServerSnapshot encode/decode roundtrip.
    #[test]
    fn server_snapshot_roundtrip() {
        let mut snap = ServerSnapshot::new(100);
        snap.entities.push(EntityState::new(1));
        snap.entities.push(EntityState::new(2));
        let encoded = snap.encode();
        let decoded = ServerSnapshot::decode(&encoded).unwrap();
        assert_eq!(decoded.tick, 100);
        assert_eq!(decoded.entities.len(), 2);
    }

    // Test 3: Server does not predict.
    #[test]
    fn server_does_not_predict() {
        let mut lc = LagCompensation::new(LagCompensationConfig::server());
        lc.predict_tick(1, HashMap::new());
        assert!(lc.history.is_empty());
    }

    // Test 4: Client stores prediction history.
    #[test]
    fn client_stores_predictions() {
        let mut lc = LagCompensation::new(LagCompensationConfig::client());
        let mut states = HashMap::new();
        states.insert(1, EntityState::new(1));
        lc.predict_tick(1, states);
        assert_eq!(lc.history_len(), 1);
    }

    // Test 5: Reconciliation produces corrections for diverged entities.
    #[test]
    fn reconciliation_detects_divergence() {
        let mut lc = LagCompensation::new(LagCompensationConfig::client());

        // Predict state where entity 1 is at (100, 100)
        let mut predicted = HashMap::new();
        predicted.insert(
            1,
            EntityState {
                entity_id: 1,
                position_x: 100.0,
                position_y: 100.0,
                velocity_x: 0.0,
                velocity_y: 0.0,
                dirty_mask: 0,
            },
        );
        lc.predict_tick(10, predicted);

        // Server says entity 1 is at (95, 105)
        let mut snap = ServerSnapshot::new(10);
        snap.entities.push(EntityState {
            entity_id: 1,
            position_x: 95.0,
            position_y: 105.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            dirty_mask: 0,
        });

        let corrections = lc.reconcile(&snap);
        assert!(!corrections.is_empty());
        assert_eq!(corrections[0].entity_id, 1);
        assert!((corrections[0].error_distance - 7.071).abs() < 0.01);
    }

    // Test 6: No correction when state matches.
    #[test]
    fn no_correction_when_matching() {
        let mut lc = LagCompensation::new(LagCompensationConfig::client());

        let mut predicted = HashMap::new();
        predicted.insert(
            1,
            EntityState {
                entity_id: 1,
                position_x: 50.0,
                position_y: 50.0,
                velocity_x: 0.0,
                velocity_y: 0.0,
                dirty_mask: 0,
            },
        );
        lc.predict_tick(5, predicted);

        let mut snap = ServerSnapshot::new(5);
        snap.entities.push(EntityState {
            entity_id: 1,
            position_x: 50.0,
            position_y: 50.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            dirty_mask: 0,
        });

        let corrections = lc.reconcile(&snap);
        assert!(corrections.is_empty());
    }

    // Test 7: Stale snapshots are ignored.
    #[test]
    fn stale_snapshot_ignored() {
        let mut lc = LagCompensation::new(LagCompensationConfig::client());
        lc.last_confirmed_tick = 20;

        let snap = ServerSnapshot::new(10);
        let corrections = lc.reconcile(&snap);
        assert!(corrections.is_empty());
    }

    // Test 8: History pruning after reconciliation.
    #[test]
    fn history_prunes_after_reconcile() {
        let mut lc = LagCompensation::new(LagCompensationConfig::client());
        lc.predict_tick(1, HashMap::new());
        lc.predict_tick(2, HashMap::new());
        lc.predict_tick(3, HashMap::new());

        let snap = ServerSnapshot::new(2);
        lc.reconcile(&snap);

        assert!(lc.history.find(1).is_none());
        assert!(lc.history.find(2).is_some());
        assert!(lc.history.find(3).is_some());
    }

    // Test 9: Prediction delta calculation.
    #[test]
    fn prediction_delta() {
        let mut lc = LagCompensation::new(LagCompensationConfig::client());
        assert_eq!(lc.prediction_delta(10), 10);
        lc.last_confirmed_tick = 7;
        assert_eq!(lc.prediction_delta(10), 3);
    }

    // Test 10: Prediction safety check.
    #[test]
    fn prediction_safety() {
        let mut lc = LagCompensation::new(LagCompensationConfig::client());
        lc.config.max_prediction_ticks = 5;
        lc.last_confirmed_tick = 10;
        assert!(lc.is_prediction_safe(12));
        assert!(!lc.is_prediction_safe(20));
    }

    // Test 11: Reset clears all state.
    #[test]
    fn reset_clears_state() {
        let mut lc = LagCompensation::new(LagCompensationConfig::client());
        lc.predict_tick(1, HashMap::new());
        lc.last_confirmed_tick = 1;
        lc.reconciliation_count = 5;
        lc.total_error_distance = 3.5;
        lc.reset();
        assert!(lc.history.is_empty());
        assert_eq!(lc.reconciliation_count, 0);
        assert_eq!(lc.total_error_distance, 0.0);
    }

    // Test 12: Smoothing factor calculation.
    #[test]
    fn smoothing_factor() {
        let corr = ReconciliationCorrection {
            entity_id: 1,
            predicted_x: 100.0,
            predicted_y: 100.0,
            server_x: 90.0,
            server_y: 90.0,
            error_distance: 14.14,
        };
        let factor = corr.smoothing_factor(50.0, 25.0);
        assert!((factor - 0.5).abs() < 0.01);
        let factor = corr.smoothing_factor(50.0, 100.0);
        assert!((factor - 1.0).abs() < 0.01);
    }
}
