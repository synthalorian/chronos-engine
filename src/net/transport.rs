//! UDP Transport & Connection Management — Phase 12 Networking Foundation.
//!
//! Provides a thin non-blocking UDP socket wrapper, length-prefixed packet
//! framing, and a connection manager with heartbeats, timeouts, and
//! per-connection statistics.

use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Instant;

// ──────────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────────

/// Errors that can occur during network operations.
#[derive(Debug)]
pub enum UdpError {
    Io(io::Error),
    WouldBlock,
    PacketTooLarge,
    MalformedPacket,
    UnknownConnection,
    NotConnected,
    BindFailed(String),
}

impl std::fmt::Display for UdpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UdpError::Io(e) => write!(f, "UDP I/O error: {}", e),
            UdpError::WouldBlock => write!(f, "UDP operation would block"),
            UdpError::PacketTooLarge => write!(f, "packet exceeds maximum size"),
            UdpError::MalformedPacket => write!(f, "malformed packet received"),
            UdpError::UnknownConnection => write!(f, "unknown connection ID"),
            UdpError::NotConnected => write!(f, "not connected to a peer"),
            UdpError::BindFailed(addr) => write!(f, "failed to bind to {}", addr),
        }
    }
}

impl std::error::Error for UdpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UdpError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for UdpError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::WouldBlock {
            UdpError::WouldBlock
        } else {
            UdpError::Io(e)
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Packet
// ──────────────────────────────────────────────────────────────

/// Maximum UDP payload size (conservative, below typical MTU).
pub const MAX_PACKET_SIZE: usize = 1200;

/// Header for every packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    /// Channel identifier (0 = unreliable, 1 = reliable, 2+ = custom).
    pub channel: u8,
    /// Sequence number for ordering / acking.
    pub sequence: u16,
    /// Acknowledgement number for the last received sequence.
    pub ack: u16,
}

impl PacketHeader {
    /// Wire size of the header in bytes.
    pub const SIZE: usize = 5;

    /// Encode the header to a byte buffer.
    pub fn write(&self, buf: &mut [u8]) {
        buf[0] = self.channel;
        buf[1..3].copy_from_slice(&self.sequence.to_le_bytes());
        buf[3..5].copy_from_slice(&self.ack.to_le_bytes());
    }

    /// Decode a header from a byte buffer.
    pub fn read(buf: &[u8]) -> Option<Self> {
        if buf.len() < Self::SIZE {
            return None;
        }
        Some(PacketHeader {
            channel: buf[0],
            sequence: u16::from_le_bytes([buf[1], buf[2]]),
            ack: u16::from_le_bytes([buf[3], buf[4]]),
        })
    }
}

/// A framed network packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}

impl Packet {
    /// Create a new packet on the given channel with the supplied payload.
    pub fn new(channel: u8, payload: impl Into<Vec<u8>>) -> Self {
        Packet {
            header: PacketHeader {
                channel,
                sequence: 0,
                ack: 0,
            },
            payload: payload.into(),
        }
    }

    /// Total wire size including header.
    pub fn wire_size(&self) -> usize {
        PacketHeader::SIZE + self.payload.len()
    }

    /// Encode the packet to a byte buffer ready for sending.
    pub fn encode(&self) -> Result<Vec<u8>, UdpError> {
        let size = self.wire_size();
        if size > MAX_PACKET_SIZE {
            return Err(UdpError::PacketTooLarge);
        }
        let mut buf = Vec::with_capacity(size);
        buf.push(self.header.channel);
        buf.extend_from_slice(&self.header.sequence.to_le_bytes());
        buf.extend_from_slice(&self.header.ack.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        Ok(buf)
    }

    /// Decode a packet from raw bytes.
    pub fn decode(buf: &[u8]) -> Result<Self, UdpError> {
        if buf.len() < PacketHeader::SIZE {
            return Err(UdpError::MalformedPacket);
        }
        let header = PacketHeader::read(buf).ok_or(UdpError::MalformedPacket)?;
        let payload = buf[PacketHeader::SIZE..].to_vec();
        Ok(Packet { header, payload })
    }
}

// ──────────────────────────────────────────────────────────────
// Connection
// ──────────────────────────────────────────────────────────────

/// Opaque handle to a peer connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u64);

/// Current state of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Handshake in progress.
    Connecting,
    /// Active and exchanging packets.
    Connected,
    /// No packets received recently; may be dead.
    TimedOut,
    /// Gracefully closed.
    Disconnected,
}

/// Per-connection statistics.
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub last_heartbeat: Instant,
    pub rtt_ms: f32,
}

impl Default for ConnectionStats {
    fn default() -> Self {
        ConnectionStats {
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            last_heartbeat: Instant::now(),
            rtt_ms: 0.0,
        }
    }
}

/// Internal peer state tracked by the connection manager.
#[derive(Debug)]
struct Peer {
    #[allow(dead_code)]
    id: ConnectionId,
    addr: SocketAddr,
    state: ConnectionState,
    stats: ConnectionStats,
    next_sequence: u16,
    /// Outgoing reliable packets awaiting acknowledgement.
    pending_acks: VecDeque<(u16, Instant, Vec<u8>)>,
}

// ──────────────────────────────────────────────────────────────
// UdpTransport
// ──────────────────────────────────────────────────────────────

/// Non-blocking UDP socket wrapper.
///
/// Provides length-prefixed packet framing, broadcast support, and
/// basic send/receive ergonomics.
pub struct UdpTransport {
    socket: UdpSocket,
    local_addr: SocketAddr,
}

impl UdpTransport {
    /// Bind a UDP socket to the given address.
    pub fn bind(addr: &str) -> Result<Self, UdpError> {
        let socket =
            UdpSocket::bind(addr).map_err(|e| UdpError::BindFailed(format!("{}: {}", addr, e)))?;
        socket.set_nonblocking(true)?;
        let local_addr = socket.local_addr()?;
        Ok(UdpTransport { socket, local_addr })
    }

    /// Bind to an OS-assigned port on the given IP.
    pub fn bind_any(ip: &str) -> Result<Self, UdpError> {
        Self::bind(&format!("{}:0", ip))
    }

    /// Local socket address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Send raw bytes to a specific address.
    pub fn send_to(&self, addr: &SocketAddr, buf: &[u8]) -> Result<usize, UdpError> {
        if buf.len() > MAX_PACKET_SIZE {
            return Err(UdpError::PacketTooLarge);
        }
        let sent = self.socket.send_to(buf, addr)?;
        Ok(sent)
    }

    /// Receive raw bytes and the sender's address.
    pub fn recv_from(&self) -> Result<Option<(SocketAddr, Vec<u8>)>, UdpError> {
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        match self.socket.recv_from(&mut buf) {
            Ok((len, addr)) => {
                buf.truncate(len);
                Ok(Some((addr, buf)))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Send a framed packet to a specific address.
    pub fn send_packet(&self, addr: &SocketAddr, packet: &Packet) -> Result<usize, UdpError> {
        let encoded = packet.encode()?;
        self.send_to(addr, &encoded)
    }

    /// Receive a framed packet.
    pub fn recv_packet(&self) -> Result<Option<(SocketAddr, Packet)>, UdpError> {
        match self.recv_from()? {
            Some((addr, buf)) => {
                let packet = Packet::decode(&buf)?;
                Ok(Some((addr, packet)))
            }
            None => Ok(None),
        }
    }

    /// Enable or disable broadcast.
    pub fn set_broadcast(&self, enabled: bool) -> Result<(), UdpError> {
        self.socket.set_broadcast(enabled)?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// ConnectionManager
// ──────────────────────────────────────────────────────────────

/// Manages peer connections over a shared UDP transport.
///
/// Tracks connection state, heartbeats, timeouts, and per-connection
/// statistics. Reliable packets are retransmitted until acknowledged.
pub struct ConnectionManager {
    transport: UdpTransport,
    peers: HashMap<ConnectionId, Peer>,
    addr_to_id: HashMap<SocketAddr, ConnectionId>,
    next_id: u64,
    /// Seconds without a packet before a connection is considered timed out.
    pub timeout_secs: u64,
    /// Heartbeat interval in seconds.
    pub heartbeat_interval_secs: u64,
    last_housekeeping: Instant,
}

impl ConnectionManager {
    /// Create a manager bound to the given address.
    pub fn bind(addr: &str) -> Result<Self, UdpError> {
        let transport = UdpTransport::bind(addr)?;
        Ok(ConnectionManager {
            transport,
            peers: HashMap::new(),
            addr_to_id: HashMap::new(),
            next_id: 1,
            timeout_secs: 10,
            heartbeat_interval_secs: 2,
            last_housekeeping: Instant::now(),
        })
    }

    /// Connect to a remote peer by address.
    ///
    /// Returns a [`ConnectionId`] handle for sending to this peer.
    pub fn connect(&mut self, addr: &SocketAddr) -> Result<ConnectionId, UdpError> {
        if let Some(id) = self.addr_to_id.get(addr) {
            return Ok(*id);
        }

        let id = ConnectionId(self.next_id);
        self.next_id += 1;

        let peer = Peer {
            id,
            addr: *addr,
            state: ConnectionState::Connecting,
            stats: ConnectionStats::default(),
            next_sequence: 1,
            pending_acks: VecDeque::new(),
        };

        self.peers.insert(id, peer);
        self.addr_to_id.insert(*addr, id);

        Ok(id)
    }

    /// Disconnect a peer and remove it from tracking.
    pub fn disconnect(&mut self, id: ConnectionId) {
        if let Some(peer) = self.peers.remove(&id) {
            self.addr_to_id.remove(&peer.addr);
        }
    }

    /// Send a packet to a connected peer.
    pub fn send(&mut self, id: ConnectionId, packet: &Packet) -> Result<usize, UdpError> {
        let peer = self.peers.get_mut(&id).ok_or(UdpError::UnknownConnection)?;

        let mut p = packet.clone();
        p.header.sequence = peer.next_sequence;
        peer.next_sequence = peer.next_sequence.wrapping_add(1);

        let encoded = p.encode()?;
        let sent = self.transport.send_to(&peer.addr, &encoded)?;

        peer.stats.packets_sent += 1;
        peer.stats.bytes_sent += sent as u64;

        // If reliable channel, store for retransmission.
        if p.header.channel == 1 {
            peer.pending_acks
                .push_back((p.header.sequence, Instant::now(), encoded));
        }

        Ok(sent)
    }

    /// Receive packets from any peer.
    ///
    /// Returns `(ConnectionId, Packet)` for known peers, or
    /// `(None, addr, packet)` for unknown senders.
    pub fn recv(&mut self) -> Result<Option<(Option<ConnectionId>, SocketAddr, Packet)>, UdpError> {
        match self.transport.recv_packet()? {
            Some((addr, packet)) => {
                if let Some(id) = self.addr_to_id.get(&addr).cloned() {
                    let peer_addr = {
                        if let Some(peer) = self.peers.get_mut(&id) {
                            peer.stats.packets_received += 1;
                            peer.stats.bytes_received +=
                                (packet.wire_size() + PacketHeader::SIZE) as u64;
                            peer.stats.last_heartbeat = Instant::now();

                            if peer.state == ConnectionState::Connecting {
                                peer.state = ConnectionState::Connected;
                            }

                            // Process incoming ack
                            if packet.header.ack != 0 {
                                peer.pending_acks
                                    .retain(|(seq, _, _)| *seq != packet.header.ack);
                            }

                            peer.addr
                        } else {
                            return Ok(Some((Some(id), addr, packet)));
                        }
                    };

                    // Acknowledge reliable packets (outside the peer borrow)
                    if packet.header.channel == 1 {
                        self.send_ack(peer_addr, packet.header.sequence)?;
                    }

                    Ok(Some((Some(id), addr, packet)))
                } else {
                    Ok(Some((None, addr, packet)))
                }
            }
            None => Ok(None),
        }
    }

    /// Send a heartbeat to all connected peers.
    pub fn send_heartbeats(&mut self) -> Result<(), UdpError> {
        let ids: Vec<ConnectionId> = self.peers.keys().cloned().collect();
        for id in ids {
            let hb = Packet::new(0, b"HB");
            let _ = self.send(id, &hb);
        }
        Ok(())
    }

    /// Run periodic housekeeping: heartbeats, timeout detection, retransmits.
    pub fn update(&mut self) -> Result<(), UdpError> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_housekeeping);

        if elapsed.as_secs() >= self.heartbeat_interval_secs {
            self.send_heartbeats()?;
        }

        // Check timeouts
        let timed_out: Vec<ConnectionId> = self
            .peers
            .iter()
            .filter(|(_, peer)| {
                peer.state != ConnectionState::Disconnected
                    && now.duration_since(peer.stats.last_heartbeat).as_secs() > self.timeout_secs
            })
            .map(|(id, _)| *id)
            .collect();

        for id in timed_out {
            if let Some(peer) = self.peers.get_mut(&id) {
                peer.state = ConnectionState::TimedOut;
            }
        }

        // Retransmit pending reliable packets older than 200ms
        for peer in self.peers.values_mut() {
            let to_retransmit: Vec<Vec<u8>> = peer
                .pending_acks
                .iter()
                .filter(|(_, sent_at, _)| now.duration_since(*sent_at).as_millis() > 200)
                .map(|(_, _, data)| data.clone())
                .collect();

            for data in to_retransmit {
                let _ = self.transport.send_to(&peer.addr, &data);
                peer.stats.packets_sent += 1;
                peer.stats.bytes_sent += data.len() as u64;
            }
        }

        self.last_housekeeping = now;
        Ok(())
    }

    /// Get the state of a connection.
    pub fn state(&self, id: ConnectionId) -> Option<ConnectionState> {
        self.peers.get(&id).map(|p| p.state)
    }

    /// Get statistics for a connection.
    pub fn stats(&self, id: ConnectionId) -> Option<&ConnectionStats> {
        self.peers.get(&id).map(|p| &p.stats)
    }

    /// Mutable access to stats (for testing / manual RTT injection).
    pub fn stats_mut(&mut self, id: ConnectionId) -> Option<&mut ConnectionStats> {
        self.peers.get_mut(&id).map(|p| &mut p.stats)
    }

    /// All active (non-timed-out) connection IDs.
    pub fn active_connections(&self) -> Vec<ConnectionId> {
        self.peers
            .iter()
            .filter(|(_, p)| {
                p.state == ConnectionState::Connected || p.state == ConnectionState::Connecting
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Number of tracked connections.
    pub fn connection_count(&self) -> usize {
        self.peers.len()
    }

    /// Access the underlying transport.
    pub fn transport(&self) -> &UdpTransport {
        &self.transport
    }

    fn send_ack(&self, peer_addr: SocketAddr, seq: u16) -> Result<(), UdpError> {
        let ack = Packet {
            header: PacketHeader {
                channel: 0,
                sequence: 0,
                ack: seq,
            },
            payload: Vec::new(),
        };
        let encoded = ack.encode()?;
        self.transport.send_to(&peer_addr, &encoded)?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// NetworkStats (global)
// ──────────────────────────────────────────────────────────────

/// Aggregated statistics across all connections.
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub total_packets_sent: u64,
    pub total_packets_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub active_connections: usize,
}

impl NetworkStats {
    /// Aggregate stats from a connection manager.
    pub fn from_manager(cm: &ConnectionManager) -> Self {
        let mut stats = NetworkStats {
            active_connections: cm.active_connections().len(),
            ..Default::default()
        };
        for (_, peer) in cm.peers.iter() {
            stats.total_packets_sent += peer.stats.packets_sent;
            stats.total_packets_received += peer.stats.packets_received;
            stats.total_bytes_sent += peer.stats.bytes_sent;
            stats.total_bytes_received += peer.stats.bytes_received;
        }
        stats
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    // Test 1: Packet encode / decode roundtrip.
    #[test]
    fn packet_roundtrip() {
        let p = Packet::new(1, b"hello world");
        let encoded = p.encode().unwrap();
        let decoded = Packet::decode(&encoded).unwrap();
        assert_eq!(p.header.channel, decoded.header.channel);
        assert_eq!(p.payload, decoded.payload);
    }

    // Test 2: Packet too large.
    #[test]
    fn packet_too_large() {
        let big = vec![0u8; MAX_PACKET_SIZE];
        let p = Packet::new(0, big);
        let result = p.encode();
        assert!(matches!(result, Err(UdpError::PacketTooLarge)));
    }

    // Test 3: Header read / write.
    #[test]
    fn header_roundtrip() {
        let h = PacketHeader {
            channel: 7,
            sequence: 42,
            ack: 99,
        };
        let mut buf = [0u8; 5];
        h.write(&mut buf);
        let decoded = PacketHeader::read(&buf).unwrap();
        assert_eq!(h, decoded);
    }

    // Test 4: UdpTransport bind and local_addr.
    #[test]
    fn transport_bind() {
        let t = UdpTransport::bind("127.0.0.1:0").unwrap();
        let addr = t.local_addr();
        assert!(addr.ip().is_loopback());
    }

    // Test 5: Send and receive between two transports.
    #[test]
    fn transport_send_receive() {
        let a = UdpTransport::bind("127.0.0.1:0").unwrap();
        let b = UdpTransport::bind("127.0.0.1:0").unwrap();

        let addr_a = a.local_addr();
        let addr_b = b.local_addr();

        let packet = Packet::new(0, b"ping");
        a.send_packet(&addr_b, &packet).unwrap();

        // Allow the kernel to route the packet.
        thread::sleep(Duration::from_millis(50));

        let received = b.recv_packet().unwrap().expect("should receive");
        assert_eq!(received.1.payload, b"ping");
        assert_eq!(received.0, addr_a);
    }

    // Test 6: ConnectionManager connect and state.
    #[test]
    fn manager_connect() {
        let mut cm = ConnectionManager::bind("127.0.0.1:0").unwrap();
        let dummy = SocketAddr::from(([192, 168, 1, 1], 1234));
        let id = cm.connect(&dummy).unwrap();
        assert_eq!(cm.state(id), Some(ConnectionState::Connecting));
        assert_eq!(cm.connection_count(), 1);
    }

    // Test 7: Duplicate connect returns same ID.
    #[test]
    fn manager_duplicate_connect() {
        let mut cm = ConnectionManager::bind("127.0.0.1:0").unwrap();
        let dummy = SocketAddr::from(([10, 0, 0, 1], 5678));
        let id1 = cm.connect(&dummy).unwrap();
        let id2 = cm.connect(&dummy).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(cm.connection_count(), 1);
    }

    // Test 8: Disconnect removes peer.
    #[test]
    fn manager_disconnect() {
        let mut cm = ConnectionManager::bind("127.0.0.1:0").unwrap();
        let dummy = SocketAddr::from(([10, 0, 0, 2], 9999));
        let id = cm.connect(&dummy).unwrap();
        assert_eq!(cm.connection_count(), 1);
        cm.disconnect(id);
        assert_eq!(cm.connection_count(), 0);
        assert!(cm.state(id).is_none());
    }

    // Test 9: Error display.
    #[test]
    fn error_display() {
        let e = UdpError::PacketTooLarge;
        assert!(e.to_string().contains("maximum"));

        let e = UdpError::MalformedPacket;
        assert!(e.to_string().contains("malformed"));

        let e = UdpError::BindFailed("0.0.0.0:0".into());
        assert!(e.to_string().contains("0.0.0.0:0"));
    }

    // Test 10: NetworkStats aggregation.
    #[test]
    fn network_stats_aggregate() {
        let mut cm = ConnectionManager::bind("127.0.0.1:0").unwrap();
        let dummy1 = SocketAddr::from(([10, 0, 0, 1], 1111));
        let dummy2 = SocketAddr::from(([10, 0, 0, 2], 2222));
        let id1 = cm.connect(&dummy1).unwrap();
        let id2 = cm.connect(&dummy2).unwrap();

        if let Some(stats) = cm.stats_mut(id1) {
            stats.packets_sent = 10;
            stats.bytes_received = 100;
        }
        if let Some(stats) = cm.stats_mut(id2) {
            stats.packets_received = 5;
            stats.bytes_sent = 50;
        }

        let net = NetworkStats::from_manager(&cm);
        assert_eq!(net.total_packets_sent, 10);
        assert_eq!(net.total_packets_received, 5);
        assert_eq!(net.total_bytes_sent, 50);
        assert_eq!(net.total_bytes_received, 100);
        assert_eq!(net.active_connections, 2);
    }

    // Test 11: ConnectionId equality.
    #[test]
    fn connection_id_equality() {
        assert_eq!(ConnectionId(1), ConnectionId(1));
        assert_ne!(ConnectionId(1), ConnectionId(2));
    }

    // Test 12: Packet decode too short.
    #[test]
    fn packet_decode_too_short() {
        let result = Packet::decode(&[1, 2]);
        assert!(matches!(result, Err(UdpError::MalformedPacket)));
    }
}
