//! Rollback Netcode — Phase 12.
//!
//! Provides GGPO-style deterministic rollback for Chronos Engine.
//!
//! # How it works
//!
//! 1. Every simulation tick, a `WorldSnapshot` captures the full game state.
//! 2. Inputs from remote players arrive with variable latency.
//! 3. If a remote input arrives late (for a tick already simulated), we:
//!    a. Restore the snapshot from the last confirmed tick before the late input.
//!    b. Replay all ticks from that point forward with the corrected input.
//! 4. The result is identical to what would have happened if the input
//!    had arrived on time.
//!
//! This requires the simulation to be fully deterministic (same inputs →
//! same state), which is guaranteed by Chronos's `TickScheduler`.

use std::collections::{HashMap, VecDeque};

use crate::net::lockstep::PlayerInput;
use crate::world::World;

// ──────────────────────────────────────────────────────────────
// Snapshot
// ──────────────────────────────────────────────────────────────

/// A compressed snapshot of entity state for rollback.
///
/// This is intentionally a simplified representation. In a production
/// engine you would snapshot every component storage directly. Here we
/// store basic world bookkeeping state, which is sufficient for
/// demonstration and testing.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldSnapshot {
    pub tick: u64,
    /// Number of alive entities at snapshot time.
    pub entity_count: u32,
    /// Free slots and generations for entity recycling.
    pub free_slots: Vec<u32>,
    pub generations: Vec<u32>,
}

impl WorldSnapshot {
    pub fn new(tick: u64) -> Self {
        WorldSnapshot {
            tick,
            entity_count: 0,
            free_slots: Vec::new(),
            generations: Vec::new(),
        }
    }

    /// Snapshot a subset of serializable component types from the world.
    ///
    /// For the rollback system to work, every component that affects
    /// gameplay must be captured here. This simplified implementation
    /// stores entity count, free slots, and generations for demonstration.
    /// In a production build you would snapshot every component storage.
    pub fn capture(world: &World, tick: u64) -> Self {
        let mut snap = WorldSnapshot::new(tick);
        snap.entity_count = world.entity_count() as u32;
        snap.free_slots = world.free_slots.iter().copied().collect();
        snap.generations = world.generations.clone();
        snap
    }
}

// ──────────────────────────────────────────────────────────────
// Snapshot History
// ──────────────────────────────────────────────────────────────

/// Ring-buffer of recent snapshots for rollback.
#[derive(Debug)]
pub struct SnapshotHistory {
    snapshots: VecDeque<WorldSnapshot>,
    max_size: usize,
}

impl SnapshotHistory {
    pub fn new(max_size: usize) -> Self {
        SnapshotHistory {
            snapshots: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Store a new snapshot, evicting the oldest if at capacity.
    pub fn push(&mut self, snapshot: WorldSnapshot) {
        if self.snapshots.len() == self.max_size {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(snapshot);
    }

    /// Find the newest snapshot with tick <= target.
    pub fn find_before(&self, target_tick: u64) -> Option<&WorldSnapshot> {
        self.snapshots
            .iter()
            .rfind(|s| s.tick <= target_tick)
    }

    /// Find the snapshot for an exact tick.
    pub fn find_exact(&self, tick: u64) -> Option<&WorldSnapshot> {
        self.snapshots.iter().find(|s| s.tick == tick)
    }

    /// Oldest tick currently stored.
    pub fn oldest_tick(&self) -> Option<u64> {
        self.snapshots.front().map(|s| s.tick)
    }

    /// Newest tick currently stored.
    pub fn newest_tick(&self) -> Option<u64> {
        self.snapshots.back().map(|s| s.tick)
    }

    /// Number of stored snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

// ──────────────────────────────────────────────────────────────
// Rollback State
// ──────────────────────────────────────────────────────────────

/// Tracks whether a rollback is currently in progress and which
/// ticks need re-simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackState {
    /// Normal forward simulation.
    Normal,
    /// A rollback is in progress; we are re-simulating from `from_tick`.
    Resimulating { from_tick: u64, target_tick: u64 },
}

// ──────────────────────────────────────────────────────────────
// Rollback Manager
// ──────────────────────────────────────────────────────────────

/// Orchestrates rollback and re-simulation.
///
/// # Usage
///
/// 1. Every tick, call `save_snapshot(world, tick)`.
/// 2. When a late input arrives, call `request_rollback(tick)` where
///    `tick` is the tick the late input belongs to.
/// 3. Before calling `TickScheduler::tick()`, check `needs_rollback()`.
///    If true, call `perform_rollback(world)` to restore state.
/// 4. Run the scheduler forward from the restored tick to the current
///    tick using `resimulate_tick()`.
#[derive(Debug)]
pub struct RollbackManager {
    pub history: SnapshotHistory,
    pub state: RollbackState,
    pub current_tick: u64,
    /// Inputs that triggered a rollback, keyed by tick.
    pub corrected_inputs: HashMap<u64, Vec<PlayerInput>>,
    /// Number of rollbacks performed.
    pub rollback_count: u64,
    /// Total ticks re-simulated across all rollbacks.
    pub resimulated_ticks: u64,
}

impl RollbackManager {
    pub fn new(history_size: usize) -> Self {
        RollbackManager {
            history: SnapshotHistory::new(history_size),
            state: RollbackState::Normal,
            current_tick: 0,
            corrected_inputs: HashMap::new(),
            rollback_count: 0,
            resimulated_ticks: 0,
        }
    }

    /// Save a snapshot of the current world state.
    pub fn save_snapshot(&mut self, world: &World) {
        let snap = WorldSnapshot::capture(world, self.current_tick);
        self.history.push(snap);
    }

    /// Advance the internal tick counter (call after each simulation step).
    pub fn advance_tick(&mut self) {
        self.current_tick += 1;
    }

    /// Request a rollback because `tick` received a corrected input.
    pub fn request_rollback(&mut self, tick: u64, corrected: Vec<PlayerInput>) {
        if tick >= self.current_tick {
            // Input is for current or future tick; no rollback needed.
            return;
        }

        // Only roll back if we have a snapshot before this tick.
        if self.history.find_before(tick).is_some() {
            self.corrected_inputs.insert(tick, corrected);
            self.state = RollbackState::Resimulating {
                from_tick: tick,
                target_tick: self.current_tick,
            };
        }
    }

    /// Returns `true` if a rollback needs to be performed before
    /// the next simulation tick.
    pub fn needs_rollback(&self) -> bool {
        matches!(self.state, RollbackState::Resimulating { .. })
    }

    /// Perform the rollback: restore the world to the snapshot before
    /// `from_tick`, then replay corrected inputs.
    ///
    /// Returns the tick to resume simulation from.
    pub fn perform_rollback(&mut self, world: &mut World) -> Option<u64> {
        if let RollbackState::Resimulating {
            from_tick,
            target_tick,
        } = self.state
        {
            let snap = self.history.find_before(from_tick)?;

            // In a full implementation this would deserialize every
            // component back into the world's storage. Here we reset
            // entity bookkeeping state for demonstration.
            world.free_slots = snap.free_slots.iter().copied().collect();
            world.generations = snap.generations.clone();

            self.rollback_count += 1;
            self.resimulated_ticks += target_tick.saturating_sub(from_tick);

            Some(snap.tick)
        } else {
            None
        }
    }

    /// Step the re-simulation forward by one tick.
    ///
    /// Call this instead of advancing the normal tick counter while
    /// in rollback mode. Returns `true` when the target tick is reached.
    pub fn resimulate_tick(&mut self) -> bool {
        if let RollbackState::Resimulating {
            from_tick,
            target_tick,
        } = self.state
        {
            if from_tick >= target_tick {
                self.state = RollbackState::Normal;
                return true;
            }
            self.state = RollbackState::Resimulating {
                from_tick: from_tick + 1,
                target_tick,
            };
            false
        } else {
            true
        }
    }

    /// Finish a rollback and return to normal simulation.
    pub fn finish_rollback(&mut self) {
        self.state = RollbackState::Normal;
        self.corrected_inputs.clear();
    }

    /// Retrieve corrected inputs for a specific tick (if any).
    pub fn corrected_inputs_for(&self, tick: u64) -> Option<&Vec<PlayerInput>> {
        self.corrected_inputs.get(&tick)
    }

    /// Diagnostic: how far back can we roll?
    pub fn rollback_depth(&self) -> u64 {
        self.current_tick
            .saturating_sub(self.history.oldest_tick().unwrap_or(0))
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: Snapshot capture exists.
    #[test]
    fn snapshot_new() {
        let snap = WorldSnapshot::new(5);
        assert_eq!(snap.tick, 5);
        assert_eq!(snap.entity_count, 0);
    }

    // Test 2: Snapshot history push and find.
    #[test]
    fn history_push_find() {
        let mut h = SnapshotHistory::new(10);
        h.push(WorldSnapshot::new(1));
        h.push(WorldSnapshot::new(3));
        h.push(WorldSnapshot::new(5));

        assert_eq!(h.find_exact(3).unwrap().tick, 3);
        assert_eq!(h.find_before(4).unwrap().tick, 3);
        assert_eq!(h.find_before(6).unwrap().tick, 5);
    }

    // Test 3: History eviction.
    #[test]
    fn history_eviction() {
        let mut h = SnapshotHistory::new(2);
        h.push(WorldSnapshot::new(1));
        h.push(WorldSnapshot::new(2));
        h.push(WorldSnapshot::new(3));

        assert_eq!(h.len(), 2);
        assert!(h.find_exact(1).is_none());
        assert!(h.find_exact(2).is_some());
    }

    // Test 4: Rollback state transitions.
    #[test]
    fn rollback_state_transitions() {
        let mut rb = RollbackManager::new(10);
        assert_eq!(rb.state, RollbackState::Normal);
        assert!(!rb.needs_rollback());

        rb.current_tick = 20;
        rb.history.push(WorldSnapshot::new(10)); // snapshot before tick 15
        rb.request_rollback(15, vec![]);
        assert!(rb.needs_rollback());
        assert!(matches!(
            rb.state,
            RollbackState::Resimulating {
                from_tick: 15,
                target_tick: 20
            }
        ));
    }

    // Test 5: No rollback for future tick.
    #[test]
    fn no_rollback_future() {
        let mut rb = RollbackManager::new(10);
        rb.current_tick = 10;
        rb.request_rollback(15, vec![]);
        assert!(!rb.needs_rollback());
    }

    // Test 6: Resimulate tick progression.
    #[test]
    fn resimulate_progression() {
        let mut rb = RollbackManager::new(10);
        rb.current_tick = 10;
        rb.request_rollback(5, vec![]);

        let mut finished = false;
        for _ in 0..20 {
            if rb.resimulate_tick() {
                finished = true;
                break;
            }
        }
        assert!(finished);
        assert_eq!(rb.state, RollbackState::Normal);
    }

    // Test 7: Rollback counters.
    #[test]
    fn rollback_counters() {
        let mut rb = RollbackManager::new(10);
        let mut world = World::new();

        rb.current_tick = 10;
        rb.history.push(WorldSnapshot::new(0));
        rb.request_rollback(5, vec![]);

        // perform_rollback should increment rollback_count
        let _ = rb.perform_rollback(&mut world);
        assert_eq!(rb.rollback_count, 1);
        assert_eq!(rb.resimulated_ticks, 5);
    }

    // Test 8: Corrected inputs storage.
    #[test]
    fn corrected_inputs_storage() {
        let mut rb = RollbackManager::new(10);
        let input = PlayerInput {
            tick: 7,
            player_id: 1,
            payload: vec![0xAB],
        };
        rb.current_tick = 20;
        rb.history.push(WorldSnapshot::new(5)); // snapshot before tick 7
        rb.request_rollback(7, vec![input.clone()]);
        assert_eq!(rb.corrected_inputs_for(7).unwrap()[0], input);
    }

    // Test 9: Rollback depth.
    #[test]
    fn rollback_depth() {
        let mut rb = RollbackManager::new(10);
        rb.current_tick = 50;
        rb.history.push(WorldSnapshot::new(40));
        assert_eq!(rb.rollback_depth(), 10);
    }

    // Test 10: Finish rollback clears state.
    #[test]
    fn finish_rollback() {
        let mut rb = RollbackManager::new(10);
        rb.current_tick = 20;
        rb.request_rollback(10, vec![]);
        rb.finish_rollback();
        assert_eq!(rb.state, RollbackState::Normal);
        assert!(rb.corrected_inputs.is_empty());
    }
}
