//! Networking foundation for Chronos Engine.
//!
//! Provides UDP-based transport, message framing, peer discovery,
//! deterministic lockstep, rollback netcode, lobby matchmaking,
//! client-side prediction / server reconciliation, and entity
//! synchronisation with interest management.
//! Suitable for real-time multiplayer and editor-to-runtime
//! communication.

#![cfg(feature = "net")]

pub mod lobby;
pub mod lockstep;
pub mod rollback;
pub mod transport;
pub mod lag_compensation;
pub mod entity_sync;

#[cfg(feature = "voice-chat")]
pub mod voice_chat;

pub use transport::{
    ConnectionId, ConnectionManager, ConnectionState, ConnectionStats, NetworkStats, Packet,
    PacketHeader, UdpError, UdpTransport, MAX_PACKET_SIZE,
};

pub use lockstep::{LockstepConfig, LockstepSync, PlayerInput};

pub use rollback::{RollbackManager, RollbackState, SnapshotHistory, WorldSnapshot};

pub use lobby::{Lobby, LobbyError, MatchInfo, NatTraversal};

pub use lag_compensation::{
    EntityState, LagCompensation, LagCompensationConfig, ReconciliationCorrection, ServerSnapshot,
};

pub use entity_sync::{
    EntitySyncConfig, EntitySyncManager, EntityUpdate, EntityUpdateKind, InterestManager,
    InterestMode, NetworkId, SyncSnapshot,
};

#[cfg(feature = "voice-chat")]
pub use voice_chat::{AudioPacket, MicMode, VoiceChat, VoiceCodec, VoiceConfig, VoiceError, VoiceStream};

// Convenience type aliases that match the naming used in the prelude.
pub type NetMessage = Packet;
pub type PeerId = ConnectionId;
pub type TransportError = UdpError;
