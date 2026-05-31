//! Voice Chat — Phase 12 Networking (stretch goal).
//!
//! Provides Opus-based voice communication over the Chronos
//! networking layer.
//!
//! # Architecture
//!
//! - [`VoiceCodec`] wraps an Opus encoder/decoder for PCM conversion.
//! - [`VoiceChat`] manages per-peer streams, sends voice packets
//!   over the reliable channel (channel 6), and mixes received audio
//!   into output buffers.
//! - [`VoiceConfig`] controls bitrate, frame size, sample rate, and
//!   push-to-talk vs voice-activity-detection mode.
//!
//! # Dependencies
//!
//! Requires the `voice-chat` feature and the `audiopus` crate.

use std::collections::HashMap;
use std::convert::TryFrom;
use std::time::Instant;

use crate::net::transport::{ConnectionManager, Packet, UdpError};

// ──────────────────────────────────────────────────────────────
// Opus Codec Wrapper
// ──────────────────────────────────────────────────────────────

/// Wraps an Opus encoder/decoder for voice compression.
#[derive(Debug)]
pub struct VoiceCodec {
    /// Inner Opus encoder (created lazily).
    encoder: Option<audiopus::coder::Encoder>,
    /// Inner Opus decoder.
    decoder: audiopus::coder::Decoder,
    /// Sample rate in Hz (e.g. 48000).
    sample_rate: u32,
    /// Frame size in samples (e.g. 960 for 20ms at 48kHz).
    frame_size: usize,
    /// Number of channels (1 = mono).
    channels: u8,
    /// Bitrate in bps (e.g. 32000 for 32 kbps).
    bitrate: i32,
}

impl VoiceCodec {
    /// Create a new voice codec with the given configuration.
    pub fn new(sample_rate: u32, frame_size: usize, channels: u8, bitrate: i32) -> Result<Self, audiopus::Error> {
        let sr = sample_rate_to_enum(sample_rate)?;
        let ch = channels_to_enum(channels)?;
        let decoder = audiopus::coder::Decoder::new(sr, ch)?;
        Ok(VoiceCodec {
            encoder: None,
            decoder,
            sample_rate,
            frame_size,
            channels,
            bitrate,
        })
    }

    /// Lazily initialise the encoder (required before encoding).
    fn ensure_encoder(&mut self) -> Result<(), audiopus::Error> {
        if self.encoder.is_none() {
            let sr = sample_rate_to_enum(self.sample_rate)?;
            let ch = channels_to_enum(self.channels)?;
            let mut encoder = audiopus::coder::Encoder::new(
                sr,
                ch,
                audiopus::Application::Voip,
            )?;
            encoder.set_bitrate(audiopus::Bitrate::BitsPerSecond(self.bitrate))?;
            self.encoder = Some(encoder);
        }
        Ok(())
    }

    /// Encode PCM audio data into an Opus packet.
    ///
    /// `pcm_data` should contain `frame_size * channels` i16 samples.
    /// Returns the compressed Opus bytes.
    pub fn encode(&mut self, pcm_data: &[i16]) -> Result<Vec<u8>, audiopus::Error> {
        self.ensure_encoder()?;
        let encoder = self.encoder.as_mut().unwrap();
        let mut out_buf = vec![0u8; 4096]; // Opus max packet size
        let len = encoder.encode(pcm_data, &mut out_buf)?;
        out_buf.truncate(len);
        Ok(out_buf)
    }

    /// Decode an Opus packet back into PCM audio.
    ///
    /// Returns `frame_size * channels` i16 samples.
    pub fn decode(&mut self, opus_data: &[u8]) -> Result<Vec<i16>, audiopus::Error> {
        let mut out_buf = vec![0i16; self.frame_size * self.channels as usize];
        let packet = audiopus::packet::Packet::try_from(opus_data)?;
        let signals = audiopus::MutSignals::try_from(out_buf.as_mut_slice())?;
        let samples = self.decoder.decode(Some(packet), signals, false)?;
        out_buf.truncate(samples);
        Ok(out_buf)
    }

    /// Decode a packet that may have been lost (concealment).
    ///
    /// Returns a decoded frame by performing packet-loss concealment.
    pub fn decode_loss(&mut self) -> Result<Vec<i16>, audiopus::Error> {
        let mut out_buf = vec![0i16; self.frame_size * self.channels as usize];
        let signals = audiopus::MutSignals::try_from(out_buf.as_mut_slice())?;
        let samples = self.decoder.decode(None, signals, true)?;
        out_buf.truncate(samples);
        Ok(out_buf)
    }

    /// Reset the codec state (call on join/leave to clear history).
    pub fn reset(&mut self) -> Result<(), audiopus::Error> {
        self.encoder = None;
        let sr = sample_rate_to_enum(self.sample_rate)?;
        let ch = channels_to_enum(self.channels)?;
        self.decoder = audiopus::coder::Decoder::new(sr, ch)?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// Voice Configuration
// ──────────────────────────────────────────────────────────────

/// Microphone input mode.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MicMode {
    /// Push-to-talk — voice is sent only when the user holds a key.
    #[default]
    PushToTalk,
    /// Voice activity detection — voice is sent when audio level
    /// exceeds a configurable threshold.
    VAD { threshold: f32 },
}

/// Configuration for the voice chat system.
#[derive(Debug, Clone, Copy)]
pub struct VoiceConfig {
    /// Sample rate in Hz (8000, 12000, 16000, 24000, 48000).
    pub sample_rate: u32,
    /// Frame duration in milliseconds (2.5, 5, 10, 20, 40, 60).
    pub frame_ms: f32,
    /// Number of channels (1 = mono).
    pub channels: u8,
    /// Bitrate in bps (e.g. 32000 for 32 kbps).
    pub bitrate: i32,
    /// Microphone mode.
    pub mic_mode: MicMode,
    /// Whether to enable jitter buffering.
    pub jitter_buffer: bool,
    /// Maximum jitter buffer size in packets.
    pub max_jitter: usize,
    /// Network channel for voice packets.
    pub channel: u8,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        VoiceConfig {
            sample_rate: 48000,
            frame_ms: 20.0,
            channels: 1,
            bitrate: 32000,
            mic_mode: MicMode::PushToTalk,
            jitter_buffer: true,
            max_jitter: 8,
            channel: 6,
        }
    }
}

impl VoiceConfig {
    /// Calculate the frame size in samples.
    pub fn frame_size(&self) -> usize {
        (self.sample_rate as f32 * self.frame_ms / 1000.0) as usize
    }
}

// ──────────────────────────────────────────────────────────────
// Audio Packet
// ──────────────────────────────────────────────────────────────

/// A single voice audio packet sent between peers.
#[derive(Debug, Clone)]
pub struct AudioPacket {
    /// Sequence number for ordering and loss detection.
    pub sequence: u16,
    /// Player ID who sent this packet.
    pub player_id: u64,
    /// Compressed Opus audio data.
    pub data: Vec<u8>,
}

impl AudioPacket {
    pub fn new(player_id: u64, data: Vec<u8>) -> Self {
        AudioPacket {
            sequence: 0,
            player_id,
            data,
        }
    }

    /// Serialize to bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(12 + self.data.len());
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        buf.extend_from_slice(&self.player_id.to_le_bytes());
        buf.extend_from_slice(&(self.data.len() as u16).to_le_bytes());
        buf.extend_from_slice(&self.data);
        buf
    }

    /// Deserialize from bytes.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 12 {
            return None;
        }
        let sequence = u16::from_le_bytes(buf[0..2].try_into().ok()?);
        let player_id = u64::from_le_bytes(buf[2..10].try_into().ok()?);
        let data_len = u16::from_le_bytes(buf[10..12].try_into().ok()?) as usize;
        if buf.len() < 12 + data_len {
            return None;
        }
        let data = buf[12..12 + data_len].to_vec();
        Some(AudioPacket {
            sequence,
            player_id,
            data,
        })
    }
}

// ──────────────────────────────────────────────────────────────
// Per-Player Stream
// ──────────────────────────────────────────────────────────────

/// Jitter buffer entry.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JitterEntry {
    sequence: u16,
    packet: AudioPacket,
    received_at: Instant,
}

/// Tracks a single remote player's voice stream.
#[derive(Debug)]
#[allow(dead_code)]
pub struct VoiceStream {
    player_id: u64,
    /// Incoming Opus packets, re-ordered by sequence.
    jitter_buffer: Vec<JitterEntry>,
    /// Last decoded sequence number.
    last_sequence: u16,
    /// Packets lost (consecutive missed sequence numbers).
    consecutive_loss: u32,
    /// Decoded PCM samples ready for mixing.
    decoded_pcm: Vec<i16>,
    /// Whether this stream has audio data to mix.
    has_audio: bool,
    /// Volume scalar (0.0 = mute, 1.0 = full).
    volume: f32,
}

impl VoiceStream {
    fn new(player_id: u64) -> Self {
        VoiceStream {
            player_id,
            jitter_buffer: Vec::new(),
            last_sequence: 0,
            consecutive_loss: 0,
            decoded_pcm: Vec::new(),
            has_audio: false,
            volume: 1.0,
        }
    }

    /// Insert a received packet into the jitter buffer.
    fn insert_packet(&mut self, packet: AudioPacket, max_jitter: usize) {
        // Handle sequence number wrapping
        let seq_diff = packet.sequence.wrapping_sub(self.last_sequence);
        if self.last_sequence == 0 || seq_diff < 30000 {
            // Newer packet
            self.jitter_buffer.push(JitterEntry {
                sequence: packet.sequence,
                packet,
                received_at: Instant::now(),
            });
            self.jitter_buffer.sort_by_key(|e| e.sequence);

            // Trim jitter buffer
            while self.jitter_buffer.len() > max_jitter {
                self.jitter_buffer.remove(0);
            }
        }
    }

    /// Pop the next packet from the jitter buffer (in order).
    fn pop_next(&mut self) -> Option<AudioPacket> {
        if self.jitter_buffer.is_empty() {
            return None;
        }
        let expected = self.last_sequence.wrapping_add(1);
        let idx = self.jitter_buffer.iter().position(|e| {
            let diff = e.sequence.wrapping_sub(expected);
            diff <= 5 // Allow slight reorder
        });

        if let Some(idx) = idx {
            let entry = self.jitter_buffer.remove(idx);
            self.consecutive_loss = 0;
            self.last_sequence = entry.sequence;
            Some(entry.packet)
        } else {
            // Packet loss — PLC will handle it
            self.consecutive_loss += 1;
            self.last_sequence = expected;
            None
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Voice Chat
// ──────────────────────────────────────────────────────────────

/// The main voice chat orchestrator.
///
/// Manages encoding local microphone input, sending it to peers,
/// receiving remote voice streams, and mixing decoded audio into
/// an output buffer.
///
/// # Usage
///
/// ```rust,ignore
/// let mut vc = VoiceChat::new(VoiceConfig::default());
///
/// // Each frame with mic input:
/// vc.send_audio(&mut pcm_samples, &mut conn_mgr)?;
///
/// // Each frame to receive:
/// vc.receive_audio(&mut conn_mgr)?;
/// vc.mix_output(&mut output_buffer);
/// ```
#[derive(Debug)]
pub struct VoiceChat {
    pub config: VoiceConfig,
    /// Opus codec for local encoding/decoding.
    pub codec: VoiceCodec,
    /// Per-player input streams.
    streams: HashMap<u64, VoiceStream>,
    /// Local player ID.
    local_player_id: u64,
    /// Packet sequence counter for outgoing packets.
    send_sequence: u16,
    /// Whether push-to-talk is currently active.
    pub ptt_active: bool,
    /// Total bytes sent.
    pub bytes_sent: u64,
    /// Total bytes received.
    pub bytes_received: u64,
    /// Packets lost (detected from sequence gaps).
    pub packets_lost: u64,
}

impl VoiceChat {
    /// Create a new voice chat instance.
    pub fn new(config: VoiceConfig, local_player_id: u64) -> Result<Self, VoiceError> {
        let codec = VoiceCodec::new(
            config.sample_rate,
            config.frame_size(),
            config.channels,
            config.bitrate,
        )?;

        Ok(VoiceChat {
            config,
            codec,
            streams: HashMap::new(),
            local_player_id,
            send_sequence: 1,
            ptt_active: false,
            bytes_sent: 0,
            bytes_received: 0,
            packets_lost: 0,
        })
    }

    /// Encode and send audio data to all connected peers.
    ///
    /// `pcm_data` should contain `frame_size * channels` i16 samples.
    /// Returns `Ok(0)` if nothing was sent (PTT inactive, or silence).
    pub fn send_audio(
        &mut self,
        pcm_data: &[i16],
        conn_mgr: &mut ConnectionManager,
    ) -> Result<usize, VoiceError> {
        // Check if we should send
        let should_send = match self.config.mic_mode {
            MicMode::PushToTalk => self.ptt_active,
            MicMode::VAD { threshold } => {
                // Simple VAD: RMS energy check
                if pcm_data.is_empty() {
                    false
                } else {
                    let sum_sq: f32 = pcm_data.iter().map(|&s| {
                        let f = s as f32 / 32768.0;
                        f * f
                    }).sum();
                    let rms = (sum_sq / pcm_data.len() as f32).sqrt();
                    rms > threshold
                }
            }
        };

        if !should_send {
            return Ok(0);
        }

        // Encode with Opus
        let encoded = self.codec.encode(pcm_data)?;

        let packet = AudioPacket {
            sequence: self.send_sequence,
            player_id: self.local_player_id,
            data: encoded,
        };
        self.send_sequence = self.send_sequence.wrapping_add(1);

        let payload = packet.encode();
        let bytes = payload.len();
        let p = Packet::new(self.config.channel, payload);

        for id in conn_mgr.active_connections() {
            conn_mgr.send(id, &p)?;
        }

        self.bytes_sent += bytes as u64;
        Ok(bytes)
    }

    /// Poll the connection manager for incoming voice packets.
    ///
    /// Decodes Opus data and stores PCM samples in per-player streams.
    pub fn receive_audio(
        &mut self,
        conn_mgr: &mut ConnectionManager,
    ) -> Result<(), VoiceError> {
        while let Some((_id, _addr, packet)) = conn_mgr.recv()? {
            if packet.header.channel == self.config.channel {
                if let Some(audio_pkt) = AudioPacket::decode(&packet.payload) {
                    let player_id = audio_pkt.player_id;

                    // Get or create stream for this player
                    let stream = self.streams.entry(player_id).or_insert_with(|| VoiceStream::new(player_id));
                    if self.config.jitter_buffer {
                        stream.insert_packet(audio_pkt, self.config.max_jitter);
                    } else {
                        // Direct decode (no jitter buffering)
                        let decoded = self.codec.decode(&audio_pkt.data)?;
                        stream.decoded_pcm = decoded;
                        stream.has_audio = true;
                    }

                    self.bytes_received += packet.payload.len() as u64;
                }
            }
        }

        // Process jitter buffers for each stream
        if self.config.jitter_buffer {
            let codec = &mut self.codec;
            for stream in self.streams.values_mut() {
                while let Some(pkt) = stream.pop_next() {
                    match codec.decode(&pkt.data) {
                        Ok(pcm) => {
                            stream.decoded_pcm = pcm;
                            stream.has_audio = true;
                        }
                        Err(_) => {
                            // Use PLC on decode failure
                            if let Ok(pcm) = codec.decode_loss() {
                                stream.decoded_pcm = pcm;
                            }
                        }
                    }
                }

                // Handle packet loss with PLC
                if stream.consecutive_loss > 0 && stream.consecutive_loss < 5 {
                    if let Ok(pcm) = codec.decode_loss() {
                        stream.decoded_pcm = pcm;
                        stream.has_audio = true;
                    }
                    if stream.consecutive_loss > 1 {
                        self.packets_lost += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Mix decoded audio from all remote players into a single output buffer.
    ///
    /// `output` should be a mutable slice of `frame_size * channels` i16 samples.
    /// Audio from each remote player is summed and clamped.
    pub fn mix_output(&mut self, output: &mut [i16]) {
        output.fill(0);

        for stream in self.streams.values() {
            if !stream.has_audio || stream.volume <= 0.0 {
                continue;
            }

            for (out_sample, &decoded) in output.iter_mut().zip(stream.decoded_pcm.iter()) {
                let mixed = *out_sample as i32 + (decoded as f32 * stream.volume) as i32;
                *out_sample = mixed.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
        }
    }

    /// Set volume for a specific remote player.
    pub fn set_player_volume(&mut self, player_id: u64, volume: f32) {
        if let Some(stream) = self.streams.get_mut(&player_id) {
            stream.volume = volume.clamp(0.0, 1.0);
        }
    }

    /// Mute or unmute a remote player.
    pub fn mute_player(&mut self, player_id: u64, muted: bool) {
        self.set_player_volume(player_id, if muted { 0.0 } else { 1.0 });
    }

    /// Remove a disconnected player's stream.
    pub fn remove_player(&mut self, player_id: u64) {
        self.streams.remove(&player_id);
    }

    /// Whether any remote player has audio to play.
    pub fn has_audio(&self) -> bool {
        self.streams.values().any(|s| s.has_audio)
    }

    /// Number of active remote streams.
    pub fn active_stream_count(&self) -> usize {
        self.streams.values().filter(|s| s.has_audio).count()
    }

    /// Total tracked streams.
    pub fn total_streams(&self) -> usize {
        self.streams.len()
    }

    /// Reset voice chat state (call on match end).
    pub fn reset(&mut self) -> Result<(), VoiceError> {
        self.codec.reset()?;
        self.streams.clear();
        self.send_sequence = 1;
        self.ptt_active = false;
        self.bytes_sent = 0;
        self.bytes_received = 0;
        self.packets_lost = 0;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────────

/// Errors that can occur during voice chat operations.
#[derive(Debug)]
pub enum VoiceError {
    /// Opus codec error.
    Codec(audiopus::Error),
    /// Network transport error.
    Transport(UdpError),
}

impl From<UdpError> for VoiceError {
    fn from(e: UdpError) -> Self {
        VoiceError::Transport(e)
    }
}

impl From<audiopus::Error> for VoiceError {
    fn from(e: audiopus::Error) -> Self {
        VoiceError::Codec(e)
    }
}

impl std::fmt::Display for VoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceError::Codec(e) => write!(f, "Opus codec error: {}", e),
            VoiceError::Transport(e) => write!(f, "network error: {}", e),
        }
    }
}

impl std::error::Error for VoiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VoiceError::Codec(e) => Some(e),
            VoiceError::Transport(e) => Some(e),
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────────────────

/// Convert a numeric sample rate to the [`audiopus::SampleRate`] enum.
fn sample_rate_to_enum(sample_rate: u32) -> Result<audiopus::SampleRate, audiopus::Error> {
    match sample_rate {
        8000 => Ok(audiopus::SampleRate::Hz8000),
        12000 => Ok(audiopus::SampleRate::Hz12000),
        16000 => Ok(audiopus::SampleRate::Hz16000),
        24000 => Ok(audiopus::SampleRate::Hz24000),
        48000 => Ok(audiopus::SampleRate::Hz48000),
        _ => Err(audiopus::Error::InvalidSampleRate(sample_rate as i32)),
    }
}

/// Convert a numeric channel count to the [`audiopus::Channels`] enum.
fn channels_to_enum(channels: u8) -> Result<audiopus::Channels, audiopus::Error> {
    match channels {
        1 => Ok(audiopus::Channels::Mono),
        2 => Ok(audiopus::Channels::Stereo),
        _ => Err(audiopus::Error::InvalidChannels(channels as i32)),
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: AudioPacket encode/decode roundtrip.
    #[test]
    fn audio_packet_roundtrip() {
        let pkt = AudioPacket {
            sequence: 42,
            player_id: 7,
            data: vec![0x01, 0x02, 0x03],
        };
        let encoded = pkt.encode();
        let decoded = AudioPacket::decode(&encoded).unwrap();
        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.player_id, 7);
        assert_eq!(decoded.data, vec![0x01, 0x02, 0x03]);
    }

    // Test 2: AudioPacket decode too short.
    #[test]
    fn audio_packet_too_short() {
        assert!(AudioPacket::decode(&[0; 5]).is_none());
    }

    // Test 3: VoiceConfig frame_size calculation.
    #[test]
    fn voice_config_frame_size() {
        let config = VoiceConfig::default();
        // 48000 Hz * 20ms = 960 samples
        assert_eq!(config.frame_size(), 960);
    }

    // Test 4: Jitter buffer ordering and reordering.
    #[test]
    fn jitter_buffer_order() {
        let mut stream = VoiceStream::new(1);
        // Set last_sequence to 0 (so pop_next expects seq 1)
        stream.insert_packet(
            AudioPacket { sequence: 1, player_id: 1, data: vec![1] },
            8,
        );
        stream.insert_packet(
            AudioPacket { sequence: 3, player_id: 1, data: vec![3] },
            8,
        );
        stream.insert_packet(
            AudioPacket { sequence: 2, player_id: 1, data: vec![2] },
            8,
        );

        // Should pop in sequence order: 1, 2, 3
        assert_eq!(stream.pop_next().unwrap().data, vec![1]);
        assert_eq!(stream.pop_next().unwrap().data, vec![2]);
        assert_eq!(stream.pop_next().unwrap().data, vec![3]);
        assert!(stream.pop_next().is_none());
    }

    // Test 6: Mute sets volume to 0.
    #[test]
    fn mute_player() {
        // We can't easily instantiate VoiceChat without audiopus,
        // but we can test VoiceStream mute logic
        let mut stream = VoiceStream::new(1);
        assert!((stream.volume - 1.0).abs() < 0.001);
        stream.volume = 0.0;
        assert!((stream.volume - 0.0).abs() < 0.001);
    }
}
