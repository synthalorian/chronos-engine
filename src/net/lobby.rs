//! Lobby System — Phase 12.
//!
//! Provides matchmaking primitives for Chronos multiplayer games:
//! - Match metadata (name, mode, map, max players, current players)
//! - Host / join / list operations
//! - NAT traversal stubs (UPnP discovery, hole-punching)
//!
//! The lobby itself is transport-agnostic; it works with any
//! [`ConnectionManager`].

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::net::transport::{ConnectionId, ConnectionManager, Packet, UdpError};

// ──────────────────────────────────────────────────────────────
// Match Metadata
// ──────────────────────────────────────────────────────────────

/// Information about a hosted game session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchInfo {
    /// Unique match identifier (host-assigned).
    pub match_id: u64,
    /// Human-readable match name.
    pub name: String,
    /// Game mode string (e.g., "ffa", "team_deathmatch", "coop").
    pub mode: String,
    /// Map / level identifier.
    pub map: String,
    /// Maximum number of players.
    pub max_players: u32,
    /// Currently connected players.
    pub current_players: u32,
    /// Whether the match is password-protected.
    pub has_password: bool,
    /// Public address of the host.
    pub host_addr: Option<SocketAddr>,
    /// Latency tier (ping in ms, estimated).
    pub ping_ms: u32,
    /// When this entry was last refreshed.
    pub last_seen: Instant,
}

impl MatchInfo {
    pub fn new(match_id: u64, name: impl Into<String>) -> Self {
        MatchInfo {
            match_id,
            name: name.into(),
            mode: "default".into(),
            map: "unknown".into(),
            max_players: 4,
            current_players: 0,
            has_password: false,
            host_addr: None,
            ping_ms: 0,
            last_seen: Instant::now(),
        }
    }

    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = mode.into();
        self
    }

    pub fn with_map(mut self, map: impl Into<String>) -> Self {
        self.map = map.into();
        self
    }

    pub fn with_max_players(mut self, max: u32) -> Self {
        self.max_players = max;
        self
    }

    pub fn with_password(mut self, enabled: bool) -> Self {
        self.has_password = enabled;
        self
    }

    pub fn with_host_addr(mut self, addr: SocketAddr) -> Self {
        self.host_addr = Some(addr);
        self
    }

    pub fn with_ping(mut self, ping: u32) -> Self {
        self.ping_ms = ping;
        self
    }

    /// Serialize to a compact byte representation.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.match_id.to_le_bytes());
        buf.extend_from_slice(&(self.name.len() as u16).to_le_bytes());
        buf.extend_from_slice(self.name.as_bytes());
        buf.extend_from_slice(&(self.mode.len() as u16).to_le_bytes());
        buf.extend_from_slice(self.mode.as_bytes());
        buf.extend_from_slice(&(self.map.len() as u16).to_le_bytes());
        buf.extend_from_slice(self.map.as_bytes());
        buf.extend_from_slice(&self.max_players.to_le_bytes());
        buf.extend_from_slice(&self.current_players.to_le_bytes());
        buf.push(self.has_password as u8);
        buf.push(if self.host_addr.is_some() { 1 } else { 0 });
        if let Some(addr) = self.host_addr {
            match addr.ip() {
                std::net::IpAddr::V4(ip) => {
                    buf.push(4);
                    buf.extend_from_slice(&ip.octets());
                }
                std::net::IpAddr::V6(ip) => {
                    buf.push(6);
                    buf.extend_from_slice(&ip.octets());
                }
            }
            buf.extend_from_slice(&addr.port().to_le_bytes());
        }
        buf.extend_from_slice(&self.ping_ms.to_le_bytes());
        buf
    }

    /// Deserialize from bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        let mut off = 0;
        let match_id = u64::from_le_bytes(read_bytes(buf, &mut off, 8)?.try_into().ok()?);
        let name_len = u16::from_le_bytes(read_bytes(buf, &mut off, 2)?.try_into().ok()?) as usize;
        let name = String::from_utf8(read_bytes(buf, &mut off, name_len)?.to_vec()).ok()?;
        let mode_len = u16::from_le_bytes(read_bytes(buf, &mut off, 2)?.try_into().ok()?) as usize;
        let mode = String::from_utf8(read_bytes(buf, &mut off, mode_len)?.to_vec()).ok()?;
        let map_len = u16::from_le_bytes(read_bytes(buf, &mut off, 2)?.try_into().ok()?) as usize;
        let map = String::from_utf8(read_bytes(buf, &mut off, map_len)?.to_vec()).ok()?;
        let max_players = u32::from_le_bytes(read_bytes(buf, &mut off, 4)?.try_into().ok()?);
        let current_players = u32::from_le_bytes(read_bytes(buf, &mut off, 4)?.try_into().ok()?);
        let has_password = read_byte(buf, &mut off)? != 0;
        let has_addr = read_byte(buf, &mut off)? != 0;
        let host_addr = if has_addr {
            let ip_type = read_byte(buf, &mut off)?;
            let ip = if ip_type == 4 {
                let octets: [u8; 4] = read_bytes(buf, &mut off, 4)?.try_into().ok()?;
                std::net::IpAddr::V4(std::net::Ipv4Addr::from(octets))
            } else {
                let octets: [u8; 16] = read_bytes(buf, &mut off, 16)?.try_into().ok()?;
                std::net::IpAddr::V6(std::net::Ipv6Addr::from(octets))
            };
            let port = u16::from_le_bytes(read_bytes(buf, &mut off, 2)?.try_into().ok()?);
            Some(SocketAddr::new(ip, port))
        } else {
            None
        };
        let ping_ms = u32::from_le_bytes(read_bytes(buf, &mut off, 4)?.try_into().ok()?);
        Some(MatchInfo {
            match_id,
            name,
            mode,
            map,
            max_players,
            current_players,
            has_password,
            host_addr,
            ping_ms,
            last_seen: Instant::now(),
        })
    }

    /// Returns true if this match is full.
    pub fn is_full(&self) -> bool {
        self.current_players >= self.max_players
    }

    /// Returns true if the entry is stale (not refreshed recently).
    pub fn is_stale(&self, threshold: Duration) -> bool {
        self.last_seen.elapsed() > threshold
    }
}

fn read_byte(buf: &[u8], off: &mut usize) -> Option<u8> {
    if *off >= buf.len() {
        return None;
    }
    let b = buf[*off];
    *off += 1;
    Some(b)
}

fn read_bytes<'a>(buf: &'a [u8], off: &mut usize, len: usize) -> Option<&'a [u8]> {
    if *off + len > buf.len() {
        return None;
    }
    let slice = &buf[*off..*off + len];
    *off += len;
    Some(slice)
}

// ──────────────────────────────────────────────────────────────
// Lobby
// ──────────────────────────────────────────────────────────────

/// The high-level lobby interface.
///
/// Hosts advertise their match via the connection manager; clients
/// browse the discovered matches and join one.
#[derive(Debug)]
pub struct Lobby {
    /// Local player name.
    pub local_name: String,
    /// Matches discovered from remote hosts.
    pub discovered_matches: HashMap<u64, MatchInfo>,
    /// The match we are currently hosting (if any).
    pub hosted_match: Option<MatchInfo>,
    /// Connection IDs of players who have joined our hosted match.
    pub joined_players: Vec<ConnectionId>,
    /// How long to keep stale match entries.
    pub stale_threshold: Duration,
    /// Last time we broadcasted our match advertisement.
    last_advertise: Instant,
    /// Advertise interval.
    advertise_interval: Duration,
}

impl Lobby {
    pub fn new(local_name: impl Into<String>) -> Self {
        Lobby {
            local_name: local_name.into(),
            discovered_matches: HashMap::new(),
            hosted_match: None,
            joined_players: Vec::new(),
            stale_threshold: Duration::from_secs(30),
            last_advertise: Instant::now() - Duration::from_secs(60),
            advertise_interval: Duration::from_secs(5),
        }
    }

    // ── Hosting ──

    /// Start hosting a new match.
    pub fn host_match(&mut self, info: MatchInfo) {
        self.hosted_match = Some(info);
        self.joined_players.clear();
    }

    /// Stop hosting.
    pub fn stop_hosting(&mut self) {
        self.hosted_match = None;
        self.joined_players.clear();
    }

    /// Accept a connection into the hosted match.
    pub fn accept_player(&mut self, conn_id: ConnectionId) -> Result<(), LobbyError> {
        if let Some(ref mut match_info) = self.hosted_match {
            if match_info.is_full() {
                return Err(LobbyError::MatchFull);
            }
            if !self.joined_players.contains(&conn_id) {
                self.joined_players.push(conn_id);
                match_info.current_players = self.joined_players.len() as u32;
            }
            Ok(())
        } else {
            Err(LobbyError::NotHosting)
        }
    }

    /// Remove a player from the hosted match.
    pub fn remove_player(&mut self, conn_id: ConnectionId) {
        self.joined_players.retain(|&id| id != conn_id);
        if let Some(ref mut match_info) = self.hosted_match {
            match_info.current_players = self.joined_players.len() as u32;
        }
    }

    // ── Discovery ──

    /// Refresh discovered matches, removing stale entries.
    pub fn refresh_matches(&mut self) {
        let threshold = self.stale_threshold;
        self.discovered_matches
            .retain(|_, m| !m.is_stale(threshold));
    }

    /// List all non-full, non-stale discovered matches.
    pub fn list_matches(&self) -> Vec<&MatchInfo> {
        self.discovered_matches
            .values()
            .filter(|m| !m.is_full() && !m.is_stale(self.stale_threshold))
            .collect()
    }

    /// Get a specific match by ID.
    pub fn get_match(&self, match_id: u64) -> Option<&MatchInfo> {
        self.discovered_matches.get(&match_id)
    }

    // ── Network ──

    /// Broadcast our hosted match to all active connections.
    pub fn advertise(&mut self, conn_mgr: &mut ConnectionManager) -> Result<(), UdpError> {
        if let Some(ref match_info) = self.hosted_match {
            let payload = match_info.encode();
            let packet = Packet::new(4, payload); // channel 4 = lobby advertisement
            for id in conn_mgr.active_connections() {
                let _ = conn_mgr.send(id, &packet);
            }
        }
        self.last_advertise = Instant::now();
        Ok(())
    }

    /// Poll for incoming match advertisements and update the match list.
    pub fn poll_discover(&mut self, conn_mgr: &mut ConnectionManager) -> Result<(), UdpError> {
        while let Some((_maybe_id, addr, packet)) = conn_mgr.recv()? {
            if packet.header.channel == 4 {
                if let Some(mut info) = MatchInfo::decode(&packet.payload) {
                    // Fill in the address we actually received from
                    if info.host_addr.is_none() {
                        info.host_addr = Some(addr);
                    }
                    info.last_seen = Instant::now();
                    self.discovered_matches.insert(info.match_id, info);
                }
            }
        }
        Ok(())
    }

    /// Periodic update: advertise and refresh.
    pub fn update(&mut self, conn_mgr: &mut ConnectionManager) -> Result<(), UdpError> {
        if self.last_advertise.elapsed() >= self.advertise_interval {
            self.advertise(conn_mgr)?;
        }
        self.poll_discover(conn_mgr)?;
        self.refresh_matches();
        Ok(())
    }

    // ── Utility ──

    /// Number of discovered matches.
    pub fn discovered_count(&self) -> usize {
        self.discovered_matches.len()
    }

    /// Are we currently hosting?
    pub fn is_hosting(&self) -> bool {
        self.hosted_match.is_some()
    }
}

// ──────────────────────────────────────────────────────────────
// Lobby Errors
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LobbyError {
    NotHosting,
    MatchFull,
    AlreadyInMatch,
    MatchNotFound,
    InvalidPassword,
}

impl std::fmt::Display for LobbyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LobbyError::NotHosting => write!(f, "not currently hosting a match"),
            LobbyError::MatchFull => write!(f, "match is full"),
            LobbyError::AlreadyInMatch => write!(f, "already in a match"),
            LobbyError::MatchNotFound => write!(f, "match not found"),
            LobbyError::InvalidPassword => write!(f, "invalid password"),
        }
    }
}

impl std::error::Error for LobbyError {}

// ──────────────────────────────────────────────────────────────
// NAT Traversal Stubs
// ──────────────────────────────────────────────────────────────

/// Placeholder for NAT traversal techniques.
///
/// In a production engine this would integrate with libraries like
/// `igd-next` (UPnP) or implement STUN/TURN hole-punching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NatTraversal {
    /// UPnP port mapping (not yet implemented).
    Upnp { external_port: u16 },
    /// Hole-punching via a rendezvous server (not yet implemented).
    HolePunch { rendezvous_addr: SocketAddr },
    /// TURN relay (not yet implemented).
    Turn { relay_addr: SocketAddr },
    /// Direct connection (no NAT traversal needed).
    Direct,
}

impl NatTraversal {
    /// Attempt UPnP port mapping (stub).
    pub fn try_upnp(_local_port: u16) -> Result<Self, &'static str> {
        // In a real implementation, use igd-next to map the port.
        Err("UPnP not yet implemented")
    }

    /// Attempt hole-punching (stub).
    pub fn try_hole_punch(rendezvous: SocketAddr) -> Self {
        NatTraversal::HolePunch {
            rendezvous_addr: rendezvous,
        }
    }

    /// Returns the external address to advertise, if known.
    pub fn advertised_addr(&self, local: SocketAddr) -> SocketAddr {
        match self {
            NatTraversal::Upnp { external_port } => SocketAddr::new(local.ip(), *external_port),
            NatTraversal::HolePunch { rendezvous_addr } => *rendezvous_addr,
            NatTraversal::Turn { relay_addr } => *relay_addr,
            NatTraversal::Direct => local,
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // Test 1: MatchInfo encode/decode roundtrip.
    #[test]
    fn match_info_roundtrip() {
        let info = MatchInfo::new(42, "Battle Arena")
            .with_mode("ffa")
            .with_map("desert")
            .with_max_players(8)
            .with_password(true)
            .with_host_addr(SocketAddr::from(([192, 168, 1, 5], 7777)))
            .with_ping(35);

        let encoded = info.encode();
        let decoded = MatchInfo::decode(&encoded).unwrap();
        assert_eq!(info.match_id, decoded.match_id);
        assert_eq!(info.name, decoded.name);
        assert_eq!(info.mode, decoded.mode);
        assert_eq!(info.map, decoded.map);
        assert_eq!(info.max_players, decoded.max_players);
        assert_eq!(info.has_password, decoded.has_password);
        assert_eq!(info.host_addr, decoded.host_addr);
        assert_eq!(info.ping_ms, decoded.ping_ms);
    }

    // Test 2: MatchInfo is_full.
    #[test]
    fn match_info_full() {
        let mut info = MatchInfo::new(1, "Test").with_max_players(2);
        assert!(!info.is_full());
        info.current_players = 2;
        assert!(info.is_full());
    }

    // Test 3: MatchInfo staleness.
    #[test]
    fn match_info_stale() {
        let mut info = MatchInfo::new(1, "Test");
        info.last_seen = Instant::now() - Duration::from_secs(60);
        assert!(info.is_stale(Duration::from_secs(30)));
        assert!(!info.is_stale(Duration::from_secs(120)));
    }

    // Test 4: Lobby host and accept.
    #[test]
    fn lobby_host_accept() {
        let mut lobby = Lobby::new("Player1");
        let match_info = MatchInfo::new(1, "My Game").with_max_players(2);
        lobby.host_match(match_info);
        assert!(lobby.is_hosting());

        let conn = ConnectionId(1);
        lobby.accept_player(conn).unwrap();
        assert_eq!(lobby.joined_players.len(), 1);

        let conn2 = ConnectionId(2);
        lobby.accept_player(conn2).unwrap();
        assert_eq!(lobby.joined_players.len(), 2);

        // Match is now full
        let conn3 = ConnectionId(3);
        assert!(matches!(
            lobby.accept_player(conn3),
            Err(LobbyError::MatchFull)
        ));
    }

    // Test 5: Lobby stop hosting.
    #[test]
    fn lobby_stop_hosting() {
        let mut lobby = Lobby::new("Player1");
        lobby.host_match(MatchInfo::new(1, "Game"));
        lobby.accept_player(ConnectionId(1)).unwrap();
        lobby.stop_hosting();
        assert!(!lobby.is_hosting());
        assert!(lobby.joined_players.is_empty());
    }

    // Test 6: Discover and refresh.
    #[test]
    fn lobby_discover_refresh() {
        let mut lobby = Lobby::new("Player1");
        let mut info = MatchInfo::new(1, "Old Game");
        info.last_seen = Instant::now() - Duration::from_secs(60);
        lobby.discovered_matches.insert(1, info);

        lobby.refresh_matches();
        assert!(lobby.discovered_matches.is_empty());
    }

    // Test 7: list_matches filters full and stale.
    #[test]
    fn lobby_list_filters() {
        let mut lobby = Lobby::new("Player1");
        lobby.discovered_matches.insert(
            1,
            MatchInfo::new(1, "Open").with_max_players(4).with_ping(10),
        );
        lobby
            .discovered_matches
            .insert(2, MatchInfo::new(2, "Full").with_max_players(2));
        lobby
            .discovered_matches
            .get_mut(&2)
            .unwrap()
            .current_players = 2;

        let list = lobby.list_matches();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Open");
    }

    // Test 8: LobbyError display.
    #[test]
    fn lobby_error_display() {
        let e = LobbyError::MatchFull;
        assert!(e.to_string().contains("full"));
        let e = LobbyError::NotHosting;
        assert!(e.to_string().contains("not currently hosting"));
    }

    // Test 9: NAT traversal stub.
    #[test]
    fn nat_traversal_stub() {
        let nat = NatTraversal::Direct;
        let local = SocketAddr::from(([127, 0, 0, 1], 1234));
        assert_eq!(nat.advertised_addr(local), local);

        let nat = NatTraversal::HolePunch {
            rendezvous_addr: SocketAddr::from(([10, 0, 0, 1], 5000)),
        };
        assert_eq!(
            nat.advertised_addr(local),
            SocketAddr::from(([10, 0, 0, 1], 5000))
        );
    }

    // Test 10: UPnP stub returns error.
    #[test]
    fn upnp_stub_error() {
        assert!(NatTraversal::try_upnp(7777).is_err());
    }

    // Test 11: Accept duplicate player is idempotent.
    #[test]
    fn accept_player_idempotent() {
        let mut lobby = Lobby::new("Player1");
        lobby.host_match(MatchInfo::new(1, "Game").with_max_players(4));
        let conn = ConnectionId(1);
        lobby.accept_player(conn).unwrap();
        lobby.accept_player(conn).unwrap();
        assert_eq!(lobby.joined_players.len(), 1);
        assert_eq!(lobby.hosted_match.as_ref().unwrap().current_players, 1);
    }

    // Test 12: MatchInfo without address encode/decode.
    #[test]
    fn match_info_no_addr_roundtrip() {
        let info = MatchInfo::new(99, "Simple Match").with_mode("coop");
        let encoded = info.encode();
        let decoded = MatchInfo::decode(&encoded).unwrap();
        assert_eq!(info.match_id, decoded.match_id);
        assert_eq!(info.mode, decoded.mode);
        assert!(decoded.host_addr.is_none());
    }
}
