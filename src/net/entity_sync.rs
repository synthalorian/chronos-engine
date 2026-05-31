//! Networked Entity Synchronization — Phase 12 Networking.
//!
//! Provides delta-compressed entity state synchronization across
//! network peers with interest management.
//!
//! # Design
//!
//! - Each networked entity gets a [`NetworkId`] that persists across
//!   reconnections.
//! - Components are registered as "syncable" — only registered component
//!   types are included in sync snapshots.
//! - Dirty-tracking: only components modified since the last sync are
//!   transmitted (delta compression).
//! - Interest management: each client receives updates only for entities
//!   within their "area of interest".

use std::collections::{HashMap, HashSet};
use crate::net::transport::{ConnectionManager, Packet, UdpError};

// ──────────────────────────────────────────────────────────────
// Network ID
// ──────────────────────────────────────────────────────────────

/// A globally-unique identifier for a networked entity.
///
/// Assigned when an entity is spawned and persists across the
/// entity's lifetime. Used instead of the local [`Entity`](crate::entity::Entity)
/// index because entity indices are not stable across peers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkId(pub u64);

impl NetworkId {
    /// Create a random network ID (calls `rand::random`).
    pub fn new_random() -> Self {
        NetworkId(rand::random())
    }

    /// Encode to bytes.
    pub fn encode(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    /// Decode from bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 8 {
            return None;
        }
        Some(NetworkId(u64::from_le_bytes(buf[0..8].try_into().ok()?)))
    }
}

// ──────────────────────────────────────────────────────────────
// Entity Update
// ──────────────────────────────────────────────────────────────

/// The kind of entity update being sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityUpdateKind {
    /// Entity was spawned (full state included).
    Spawn = 0,
    /// Entity state was updated (delta-compressed).
    Update = 1,
    /// Entity was despawned (no state needed).
    Despawn = 2,
}

/// A single entity update for the network.
#[derive(Debug, Clone)]
pub struct EntityUpdate {
    pub network_id: NetworkId,
    pub kind: EntityUpdateKind,
    pub position_x: f32,
    pub position_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    /// Generic animation state index.
    pub animation_state: u16,
    /// Bitmask of syncable components present.
    pub component_mask: u64,
    /// Extra opaque payload (custom component data).
    pub extra_data: Vec<u8>,
}

impl EntityUpdate {
    pub fn new(network_id: NetworkId, kind: EntityUpdateKind) -> Self {
        EntityUpdate {
            network_id,
            kind,
            position_x: 0.0,
            position_y: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            animation_state: 0,
            component_mask: 0,
            extra_data: Vec::new(),
        }
    }

    /// Serialize to a compact byte representation.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(48 + self.extra_data.len());
        buf.extend_from_slice(&self.network_id.0.to_le_bytes());
        buf.push(self.kind as u8);
        buf.extend_from_slice(&self.position_x.to_le_bytes());
        buf.extend_from_slice(&self.position_y.to_le_bytes());
        buf.extend_from_slice(&self.velocity_x.to_le_bytes());
        buf.extend_from_slice(&self.velocity_y.to_le_bytes());
        buf.extend_from_slice(&self.animation_state.to_le_bytes());
        buf.extend_from_slice(&self.component_mask.to_le_bytes());
        buf.extend_from_slice(&(self.extra_data.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.extra_data);
        buf
    }

    /// Deserialize from bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 35 {
            return None;
        }
        let network_id = NetworkId(u64::from_le_bytes(buf[0..8].try_into().ok()?));
        let kind = match buf[8] {
            0 => EntityUpdateKind::Spawn,
            1 => EntityUpdateKind::Update,
            2 => EntityUpdateKind::Despawn,
            _ => return None,
        };
        let position_x = f32::from_le_bytes(buf[9..13].try_into().ok()?);
        let position_y = f32::from_le_bytes(buf[13..17].try_into().ok()?);
        let velocity_x = f32::from_le_bytes(buf[17..21].try_into().ok()?);
        let velocity_y = f32::from_le_bytes(buf[21..25].try_into().ok()?);
        let animation_state = u16::from_le_bytes(buf[25..27].try_into().ok()?);
        let component_mask = u64::from_le_bytes(buf[27..35].try_into().ok()?);
        if buf.len() < 37 {
            return None;
        }
        let extra_len = u16::from_le_bytes(buf[35..37].try_into().ok()?) as usize;
        if buf.len() < 37 + extra_len {
            return None;
        }
        let extra_data = buf[37..37 + extra_len].to_vec();
        Some(EntityUpdate {
            network_id,
            kind,
            position_x,
            position_y,
            velocity_x,
            velocity_y,
            animation_state,
            component_mask,
            extra_data,
        })
    }
}

// ──────────────────────────────────────────────────────────────
// Sync Snapshot
// ──────────────────────────────────────────────────────────────

/// A delta-compressed snapshot of entity state.
///
/// Contains only entities that changed since the last full sync.
#[derive(Debug, Clone)]
pub struct SyncSnapshot {
    /// Snapshot sequence number (increments every sync).
    pub sequence: u64,
    /// Base snapshot sequence this delta applies on top of.
    /// 0 means this is a full snapshot.
    pub base_sequence: u64,
    /// Entity updates in this snapshot.
    pub updates: Vec<EntityUpdate>,
}

impl SyncSnapshot {
    pub fn new(sequence: u64) -> Self {
        SyncSnapshot {
            sequence,
            base_sequence: 0,
            updates: Vec::new(),
        }
    }

    pub fn with_base(mut self, base: u64) -> Self {
        self.base_sequence = base;
        self
    }

    /// Serialize to bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16 + self.updates.len() * 48);
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        buf.extend_from_slice(&self.base_sequence.to_le_bytes());
        buf.extend_from_slice(&(self.updates.len() as u16).to_le_bytes());
        for update in &self.updates {
            buf.extend_from_slice(&update.encode());
        }
        buf
    }

    /// Deserialize from bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 18 {
            return None;
        }
        let sequence = u64::from_le_bytes(buf[0..8].try_into().ok()?);
        let base_sequence = u64::from_le_bytes(buf[8..16].try_into().ok()?);
        let count = u16::from_le_bytes(buf[16..18].try_into().ok()?) as usize;
        let mut off = 18;
        let mut updates = Vec::with_capacity(count);
        for _ in 0..count {
            if off >= buf.len() {
                return None;
            }
            // Determine this update's wire size by scanning for the extra_data length
            if off + 37 > buf.len() {
                return None;
            }
            let extra_len = u16::from_le_bytes(buf[off + 35..off + 37].try_into().ok()?) as usize;
            let end = off + 37 + extra_len;
            if end > buf.len() {
                return None;
            }
            let update = EntityUpdate::decode(&buf[off..end])?;
            updates.push(update);
            off = end;
        }
        Some(SyncSnapshot {
            sequence,
            base_sequence,
            updates,
        })
    }
}

// ──────────────────────────────────────────────────────────────
// Interest Manager
// ──────────────────────────────────────────────────────────────

/// Defines how entity interest is determined for each client.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterestMode {
    /// Send all entities to all clients (simple, no culling).
    Global,
    /// Send only entities within a radius of each client's
    /// viewpoint (distance-based culling).
    Radius { default_radius: f32 },
    /// Send only entities the client has explicitly subscribed to.
    Subscription,
}

impl Default for InterestMode {
    fn default() -> Self {
        InterestMode::Radius { default_radius: 500.0 }
    }
}

/// Manages which entities are relevant to each client.
///
/// This prevents clients from receiving updates about entities
/// that are too far away to matter.
#[derive(Debug)]
pub struct InterestManager {
    mode: InterestMode,
    /// Per-client lists of subscribed entity IDs (for Subscription mode).
    client_subscriptions: HashMap<u64, HashSet<u64>>,
    /// Per-client viewpoint positions (for Radius mode).
    client_positions: HashMap<u64, (f32, f32)>,
}

impl InterestManager {
    pub fn new(mode: InterestMode) -> Self {
        InterestManager {
            mode,
            client_subscriptions: HashMap::new(),
            client_positions: HashMap::new(),
        }
    }

    /// Update a client's viewpoint position (for radius-based culling).
    pub fn set_client_position(&mut self, client_id: u64, x: f32, y: f32) {
        self.client_positions.insert(client_id, (x, y));
    }

    /// Subscribe a client to an entity (for subscription mode).
    pub fn subscribe(&mut self, client_id: u64, entity_net_id: u64) {
        self.client_subscriptions
            .entry(client_id)
            .or_default()
            .insert(entity_net_id);
    }

    /// Unsubscribe a client from an entity.
    pub fn unsubscribe(&mut self, client_id: u64, entity_net_id: u64) {
        if let Some(subs) = self.client_subscriptions.get_mut(&client_id) {
            subs.remove(&entity_net_id);
        }
    }

    /// Check if an entity is relevant to a specific client.
    pub fn is_relevant(
        &self,
        client_id: u64,
        entity_net_id: u64,
        entity_x: f32,
        entity_y: f32,
    ) -> bool {
        match self.mode {
            InterestMode::Global => true,
            InterestMode::Radius { default_radius } => {
                self.client_positions
                    .get(&client_id)
                    .map(|&(cx, cy)| {
                        let dx = cx - entity_x;
                        let dy = cy - entity_y;
                        (dx * dx + dy * dy) <= default_radius * default_radius
                    })
                    .unwrap_or(true) // If no position known, include it.
            }
            InterestMode::Subscription => {
                self.client_subscriptions
                    .get(&client_id)
                    .map(|subs| subs.contains(&entity_net_id))
                    .unwrap_or(false)
            }
        }
    }

    /// Remove a disconnected client.
    pub fn remove_client(&mut self, client_id: u64) {
        self.client_subscriptions.remove(&client_id);
        self.client_positions.remove(&client_id);
    }

    /// Current interest mode.
    pub fn mode(&self) -> InterestMode {
        self.mode
    }

    /// Tracked client count.
    pub fn client_count(&self) -> usize {
        self.client_positions.len()
    }
}

// ──────────────────────────────────────────────────────────────
// Entity Sync Configuration
// ──────────────────────────────────────────────────────────────

/// Configuration for the entity sync manager.
#[derive(Debug, Clone, Copy)]
pub struct EntitySyncConfig {
    /// How often (in ticks) to send full snapshots.
    pub full_sync_interval: u32,
    /// Maximum entity updates per packet.
    pub max_updates_per_packet: u32,
    /// Interest mode for culling.
    pub interest_mode: InterestMode,
    /// Sync channel identifier.
    pub channel: u8,
}

impl Default for EntitySyncConfig {
    fn default() -> Self {
        EntitySyncConfig {
            full_sync_interval: 60,
            max_updates_per_packet: 64,
            interest_mode: InterestMode::Radius { default_radius: 500.0 },
            channel: 5,
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Entity Sync Manager
// ──────────────────────────────────────────────────────────────

/// Tracks networked entities and manages delta-compressed sync.
///
/// # Usage (server)
///
/// 1. Register networked entities with `register_entity()`.
/// 2. Each tick, call `mark_dirty()` for any changed entities.
/// 3. Call `build_snapshot()` to get the delta snapshot.
/// 4. Send it to clients via the connection manager.
///
/// # Usage (client)
///
/// 1. Receive [`SyncSnapshot`] from the server.
/// 2. Apply updates via `apply_snapshot()` to update local entity state.
///
/// # Usage (both)
///
/// - Call `interest().is_relevant(...)` to filter what to send per client.
#[derive(Debug)]
pub struct EntitySyncManager {
    pub config: EntitySyncConfig,
    /// Network IDs we track (keyed by NetworkId.0 for efficiency).
    tracked_entities: HashMap<u64, TrackedEntity>,
    /// Connection ID → last sent snapshot sequence.
    last_sent: HashMap<u64, u64>,
    /// Global sequence counter.
    sequence: u64,
    /// Interest manager.
    interest_mgr: InterestManager,
}

/// Internal tracking for a single networked entity.
#[derive(Debug, Clone)]
struct TrackedEntity {
    network_id: NetworkId,
    /// Last synced state.
    last_sync: EntityUpdate,
    /// Whether the entity has changed since last sync.
    dirty: bool,
    /// Whether entity is alive (hasn't been despawned).
    alive: bool,
}

impl EntitySyncManager {
    pub fn new(config: EntitySyncConfig) -> Self {
        let interest_mgr = InterestManager::new(config.interest_mode);
        EntitySyncManager {
            config,
            tracked_entities: HashMap::new(),
            last_sent: HashMap::new(),
            sequence: 0,
            interest_mgr,
        }
    }

    /// Register a new networked entity.
    pub fn register_entity(&mut self, network_id: NetworkId, state: EntityUpdate) {
        self.tracked_entities.insert(
            network_id.0,
            TrackedEntity {
                network_id,
                last_sync: state,
                dirty: true,
                alive: true,
            },
        );
    }

    /// Mark an entity as having changed state.
    pub fn mark_dirty(&mut self, network_id: NetworkId) {
        if let Some(entity) = self.tracked_entities.get_mut(&network_id.0) {
            entity.dirty = true;
        }
    }

    /// Update an entity's current state and mark it dirty.
    pub fn update_entity(&mut self, network_id: NetworkId, new_state: EntityUpdate) {
        if let Some(entity) = self.tracked_entities.get_mut(&network_id.0) {
            entity.last_sync = new_state;
            entity.dirty = true;
        }
    }

    /// Mark an entity as despawned.
    pub fn despawn_entity(&mut self, network_id: NetworkId) {
        if let Some(entity) = self.tracked_entities.get_mut(&network_id.0) {
            entity.alive = false;
            entity.dirty = true;
        }
    }

    /// Remove all tracking for a despawned entity.
    pub fn remove_entity(&mut self, network_id: NetworkId) {
        self.tracked_entities.remove(&network_id.0);
    }

    /// Build a delta snapshot for a specific client.
    ///
    /// Only includes entities that are:
    /// 1. Dirty (changed since last sync) OR it's time for a full sync.
    /// 2. Within the client's area of interest.
    pub fn build_snapshot(&mut self, client_id: u64) -> SyncSnapshot {
        let is_full = self.sequence % self.config.full_sync_interval as u64 == 0;
        let base_seq = self.last_sent.get(&client_id).copied().unwrap_or(0);
        let seq = self.sequence;
        self.sequence += 1;

        let mut mut_snap = SyncSnapshot::new(seq);
        if !is_full && base_seq != 0 {
            mut_snap = mut_snap.with_base(base_seq);
        }

        let mut updates = Vec::new();
        for entity in self.tracked_entities.values() {
            if entity.dirty || is_full {
                let mut update = entity.last_sync.clone();
                if !entity.alive {
                    update.kind = EntityUpdateKind::Despawn;
                } else if is_full && !entity.dirty {
                    update.kind = EntityUpdateKind::Update;
                }

                // Interest check
                if self.interest_mgr.is_relevant(
                    client_id,
                    entity.network_id.0,
                    entity.last_sync.position_x,
                    entity.last_sync.position_y,
                ) {
                    updates.push(update);
                }
            }

            if updates.len() >= self.config.max_updates_per_packet as usize {
                break;
            }
        }

        // Reset dirty flags for everything we're about to send.
        for entity in self.tracked_entities.values_mut() {
            entity.dirty = false;
        }

        mut_snap.updates = updates;
        self.last_sent.insert(client_id, seq);
        mut_snap
    }

    /// Build a global snapshot (all dirty entities, no interest culling).
    ///
    /// Useful for broadcasting to all clients when each client has
    /// its own interest-managed snapshot.
    pub fn build_global_snapshot(&mut self) -> SyncSnapshot {
        let is_full = self.sequence % self.config.full_sync_interval as u64 == 0;
        let seq = self.sequence;
        self.sequence += 1;

        let mut snap = SyncSnapshot::new(seq);

        let mut updates = Vec::new();
        for entity in self.tracked_entities.values() {
            if entity.dirty || is_full {
                let mut update = entity.last_sync.clone();
                if !entity.alive {
                    update.kind = EntityUpdateKind::Despawn;
                }
                updates.push(update);
            }

            if updates.len() >= self.config.max_updates_per_packet as usize {
                break;
            }
        }

        for entity in self.tracked_entities.values_mut() {
            entity.dirty = false;
        }

        snap.updates = updates;
        snap
    }

    /// Apply a received snapshot to local tracking.
    ///
    /// Returns the list of updates (caller can use them to update
    /// the local World).
    pub fn apply_snapshot<'a>(&mut self, snapshot: &'a SyncSnapshot) -> Vec<&'a EntityUpdate> {
        let mut applied = Vec::new();
        for update in &snapshot.updates {
            match update.kind {
                EntityUpdateKind::Spawn | EntityUpdateKind::Update => {
                    // Track or update locally
                    if !self.tracked_entities.contains_key(&update.network_id.0) {
                        let state = update.clone();
                        self.tracked_entities.insert(
                            update.network_id.0,
                            TrackedEntity {
                                network_id: update.network_id,
                                last_sync: state,
                                dirty: false,
                                alive: true,
                            },
                        );
                    } else if let Some(entity) = self.tracked_entities.get_mut(&update.network_id.0)
                    {
                        entity.last_sync = update.clone();
                        entity.alive = true;
                    }
                    applied.push(update);
                }
                EntityUpdateKind::Despawn => {
                    // Mark as not alive
                    if let Some(entity) = self.tracked_entities.get_mut(&update.network_id.0) {
                        entity.alive = false;
                    }
                    applied.push(update);
                }
            }
        }
        applied
    }

    /// Send a snapshot to a specific peer via the connection manager.
    pub fn send_snapshot(
        &self,
        conn_mgr: &mut ConnectionManager,
        client_conn_id: crate::net::transport::ConnectionId,
        snapshot: &SyncSnapshot,
    ) -> Result<(), UdpError> {
        let payload = snapshot.encode();
        let packet = Packet::new(self.config.channel, payload);
        conn_mgr.send(client_conn_id, &packet)?;
        Ok(())
    }

    /// Receive snapshots from the connection manager.
    pub fn receive_snapshots(
        &mut self,
        conn_mgr: &mut ConnectionManager,
    ) -> Result<Vec<SyncSnapshot>, UdpError> {
        let mut snapshots = Vec::new();
        while let Some((_id, _addr, packet)) = conn_mgr.recv()? {
            if packet.header.channel == self.config.channel {
                if let Some(snap) = SyncSnapshot::decode(&packet.payload) {
                    snapshots.push(snap);
                }
            }
        }
        Ok(snapshots)
    }

    /// Immutable access to the interest manager.
    pub fn interest(&self) -> &InterestManager {
        &self.interest_mgr
    }

    /// Mutable access to the interest manager.
    pub fn interest_mut(&mut self) -> &mut InterestManager {
        &mut self.interest_mgr
    }

    /// Number of tracked networked entities.
    pub fn tracked_count(&self) -> usize {
        self.tracked_entities
            .values()
            .filter(|e| e.alive)
            .count()
    }

    /// Current snapshot sequence number.
    pub fn current_sequence(&self) -> u64 {
        self.sequence
    }

    /// Get the last sent sequence for a client.
    pub fn last_sent_for(&self, client_id: u64) -> Option<u64> {
        self.last_sent.get(&client_id).copied()
    }

    /// Remove a disconnected client.
    pub fn remove_client(&mut self, client_id: u64) {
        self.last_sent.remove(&client_id);
        self.interest_mgr.remove_client(client_id);
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: NetworkId encode/decode.
    #[test]
    fn network_id_roundtrip() {
        let id = NetworkId(42);
        let encoded = id.encode();
        let decoded = NetworkId::decode(&encoded).unwrap();
        assert_eq!(id, decoded);
    }

    // Test 2: EntityUpdate encode/decode roundtrip.
    #[test]
    fn entity_update_roundtrip() {
        let update = EntityUpdate {
            network_id: NetworkId(7),
            kind: EntityUpdateKind::Spawn,
            position_x: 100.0,
            position_y: 200.0,
            velocity_x: 1.0,
            velocity_y: -0.5,
            animation_state: 3,
            component_mask: 0b1111,
            extra_data: vec![0xAB, 0xCD],
        };
        let encoded = update.encode();
        let decoded = EntityUpdate::decode(&encoded).unwrap();
        assert_eq!(decoded.network_id, update.network_id);
        assert!(decoded.kind == update.kind);
        assert!((decoded.position_x - update.position_x).abs() < 0.001);
        assert_eq!(decoded.extra_data, update.extra_data);
    }

    // Test 3: SyncSnapshot encode/decode roundtrip.
    #[test]
    fn sync_snapshot_roundtrip() {
        let mut snap = SyncSnapshot::new(1).with_base(0);
        snap.updates.push(EntityUpdate::new(NetworkId(1), EntityUpdateKind::Spawn));
        snap.updates.push(EntityUpdate::new(NetworkId(2), EntityUpdateKind::Update));
        let encoded = snap.encode();
        let decoded = SyncSnapshot::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, 1);
        assert_eq!(decoded.updates.len(), 2);
    }

    // Test 4: Register entity makes it tracked.
    #[test]
    fn register_entity() {
        let mut mgr = EntitySyncManager::new(EntitySyncConfig::default());
        let id = NetworkId::new_random();
        let update = EntityUpdate::new(id, EntityUpdateKind::Spawn);
        mgr.register_entity(id, update);
        assert_eq!(mgr.tracked_count(), 1);
    }

    // Test 5: Mark dirty includes entity in next snapshot.
    #[test]
    fn dirty_entity_in_snapshot() {
        let mut mgr = EntitySyncManager::new(EntitySyncConfig::default());
        let id = NetworkId(1);
        mgr.register_entity(id, EntityUpdate::new(id, EntityUpdateKind::Spawn));

        let snap = mgr.build_snapshot(100);
        assert!(snap.updates.iter().any(|u| u.network_id == id));
    }

    // Test 6: Non-dirty entities skipped after first sync.
    #[test]
    fn clean_entity_skipped() {
        let mut mgr = EntitySyncManager::new(EntitySyncConfig::default());
        mgr.config.full_sync_interval = 100; // Long interval
        let id = NetworkId(1);
        mgr.register_entity(id, EntityUpdate::new(id, EntityUpdateKind::Spawn));

        // First snapshot includes it (dirty on register)
        let snap1 = mgr.build_snapshot(100);
        assert!(!snap1.updates.is_empty());

        // Second snapshot should skip it (no longer dirty)
        let snap2 = mgr.build_snapshot(100);
        assert!(snap2.updates.is_empty());
    }

    // Test 7: Full sync interval sends all entities.
    #[test]
    fn full_sync_interval() {
        let mut mgr = EntitySyncManager::new(EntitySyncConfig::default());
        mgr.config.full_sync_interval = 3;
        let id = NetworkId(1);
        mgr.register_entity(id, EntityUpdate::new(id, EntityUpdateKind::Spawn));

        // Consume the dirty flag
        let _ = mgr.build_snapshot(100);
        // Advance sequence to trigger full sync
        mgr.sequence = 3;
        let snap = mgr.build_snapshot(100);
        assert!(!snap.updates.is_empty());
    }

    // Test 8: Despawn entity.
    #[test]
    fn despawn_entity() {
        let mut mgr = EntitySyncManager::new(EntitySyncConfig::default());
        let id = NetworkId(1);
        mgr.register_entity(id, EntityUpdate::new(id, EntityUpdateKind::Spawn));
        mgr.despawn_entity(id);

        let snap = mgr.build_snapshot(100);
        assert!(snap.updates.iter().any(|u| {
            u.network_id == id && u.kind == EntityUpdateKind::Despawn
        }));
    }

    // Test 9: Interest radius filtering.
    #[test]
    fn interest_radius() {
        let mut im = InterestManager::new(InterestMode::Radius { default_radius: 100.0 });
        im.set_client_position(1, 0.0, 0.0);

        // Entity within radius
        assert!(im.is_relevant(1, 100, 50.0, 50.0));
        // Entity far away
        assert!(!im.is_relevant(1, 101, 500.0, 500.0));
    }

    // Test 10: Interest subscription mode.
    #[test]
    fn interest_subscription() {
        let mut im = InterestManager::new(InterestMode::Subscription);
        im.subscribe(1, 42);
        assert!(im.is_relevant(1, 42, 0.0, 0.0));
        assert!(!im.is_relevant(1, 99, 0.0, 0.0));
    }

    // Test 11: Global mode sends everything.
    #[test]
    fn interest_global() {
        let im = InterestManager::new(InterestMode::Global);
        assert!(im.is_relevant(1, 1, 99999.0, -99999.0));
    }

    // Test 12: Apply snapshot tracks new entities.
    #[test]
    fn apply_snapshot_tracks() {
        let mut mgr = EntitySyncManager::new(EntitySyncConfig::default());
        let mut snap = SyncSnapshot::new(1);
        snap.updates.push(EntityUpdate::new(NetworkId(42), EntityUpdateKind::Spawn));

        let applied = mgr.apply_snapshot(&snap);
        assert_eq!(applied.len(), 1);
        assert_eq!(mgr.tracked_count(), 1);
    }

    // Test 13: Remove client cleans up.
    #[test]
    fn remove_client() {
        let mut mgr = EntitySyncManager::new(EntitySyncConfig::default());
        mgr.last_sent.insert(1, 10);
        mgr.interest_mut().set_client_position(1, 0.0, 0.0);
        mgr.remove_client(1);
        assert!(mgr.last_sent_for(1).is_none());
        assert_eq!(mgr.interest().client_count(), 0);
    }
}
