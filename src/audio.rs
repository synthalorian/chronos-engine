//! Audio system for the Chronos Engine.
//!
//! Provides sound effect playback, music streaming with crossfade,
//! volume control, spatial audio attenuation, and audio buffer caching.
//! Built on top of rodio 0.20.

use std::collections::HashMap;
use std::io::Cursor;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

// ──────────────────────────────────────────────
// Error Types
// ──────────────────────────────────────────────

/// Errors that can occur during audio operations.
#[derive(Debug)]
pub enum AudioError {
    /// No audio output device is available.
    NoDeviceAvailable(String),
    /// Failed to decode audio data.
    DecodeFailed(String),
    /// File I/O error.
    FileError(std::io::Error),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoDeviceAvailable(msg) => write!(f, "no audio device: {}", msg),
            AudioError::DecodeFailed(msg) => write!(f, "decode failed: {}", msg),
            AudioError::FileError(err) => write!(f, "file error: {}", err),
        }
    }
}

impl std::error::Error for AudioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AudioError::FileError(err) => Some(err),
            _ => None,
        }
    }
}

// ──────────────────────────────────────────────
// Volume Control
// ──────────────────────────────────────────────

/// Independent volume channels for master, music, and SFX.
///
/// Master volume scales all output. Music and SFX volumes are applied
/// per-category before the master scaling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeControl {
    /// Master volume (0.0–1.0). Scales all output.
    pub master: f32,
    /// Music channel volume (0.0–1.0).
    pub music: f32,
    /// SFX channel volume (0.0–1.0).
    pub sfx: f32,
}

impl Default for VolumeControl {
    fn default() -> Self {
        VolumeControl {
            master: 1.0,
            music: 0.8,
            sfx: 0.8,
        }
    }
}

impl VolumeControl {
    /// Creates a new volume control with the given values, clamped to 0.0–1.0.
    pub fn new(master: f32, music: f32, sfx: f32) -> Self {
        VolumeControl {
            master: master.clamp(0.0, 1.0),
            music: music.clamp(0.0, 1.0),
            sfx: sfx.clamp(0.0, 1.0),
        }
    }

    /// Returns the effective music volume: `music * master`.
    pub fn effective_music(&self) -> f32 {
        self.music * self.master
    }

    /// Returns the effective SFX volume: `sfx * master`.
    pub fn effective_sfx(&self) -> f32 {
        self.sfx * self.master
    }
}

// ──────────────────────────────────────────────
// Spatial Audio
// ──────────────────────────────────────────────

/// Position-based volume attenuation for 3D sounds.
///
/// Uses a simple inverse distance model:
/// `volume = 1.0 / (1.0 + distance * rolloff)`
pub struct SpatialAudio;

impl SpatialAudio {
    /// Computes a volume factor from listener-to-source distance.
    ///
    /// A rolloff of 0.0 means no attenuation (always full volume).
    pub fn compute_volume(listener_pos: [f32; 3], source_pos: [f32; 3], rolloff: f32) -> f32 {
        let dx = source_pos[0] - listener_pos[0];
        let dy = source_pos[1] - listener_pos[1];
        let dz = source_pos[2] - listener_pos[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        1.0 / (1.0 + distance * rolloff)
    }
}

// ──────────────────────────────────────────────
// Sound Buffer
// ──────────────────────────────────────────────

/// Caches loaded audio file bytes to avoid per-play file I/O.
///
/// Load a file once, then replay from memory on each trigger.
/// Supports .wav, .ogg, .flac, and .mp3 (anything rodio's Decoder handles).
#[derive(Debug, Clone)]
pub struct SoundBuffer {
    cache: HashMap<String, Vec<u8>>,
}

impl Default for SoundBuffer {
    fn default() -> Self {
        SoundBuffer {
            cache: HashMap::new(),
        }
    }
}

impl SoundBuffer {
    /// Creates an empty buffer cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads a file into the cache, keyed by its path.
    pub fn load(&mut self, path: &str) -> Result<(), AudioError> {
        let bytes = std::fs::read(path).map_err(AudioError::FileError)?;
        self.cache.insert(path.to_string(), bytes);
        Ok(())
    }

    /// Returns true if the given path is cached.
    pub fn contains(&self, path: &str) -> bool {
        self.cache.contains_key(path)
    }

    /// Returns the cached bytes for a path, if loaded.
    pub fn get(&self, path: &str) -> Option<&Vec<u8>> {
        self.cache.get(path)
    }

    /// Creates a decoder from cached bytes. Clones the bytes to create a Cursor.
    pub fn decoder(&self, path: &str) -> Result<Decoder<Cursor<Vec<u8>>>, AudioError> {
        let bytes = self.cache.get(path).ok_or_else(|| {
            AudioError::FileError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("not cached: {}", path),
            ))
        })?;
        Decoder::new(Cursor::new(bytes.clone()))
            .map_err(|e| AudioError::DecodeFailed(e.to_string()))
    }

    /// Removes a cached entry, freeing memory.
    pub fn remove(&mut self, path: &str) {
        self.cache.remove(path);
    }

    /// Returns the number of cached sound files.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if no sounds are cached.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

// ──────────────────────────────────────────────
// SFX Player
// ──────────────────────────────────────────────

/// Plays one-shot sound effects.
///
/// Each call to `play` appends a new source to an internal rodio Sink.
/// Multiple sounds can overlap.
pub struct SfxPlayer {
    sink: Sink,
}

impl std::fmt::Debug for SfxPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SfxPlayer")
            .field("empty", &self.sink.empty())
            .field("paused", &self.sink.is_paused())
            .finish()
    }
}

impl SfxPlayer {
    /// Creates a new SFX player attached to the given output stream handle.
    pub fn new(handle: &OutputStreamHandle) -> Result<Self, AudioError> {
        let sink =
            Sink::try_new(handle).map_err(|e| AudioError::NoDeviceAvailable(e.to_string()))?;
        Ok(SfxPlayer { sink })
    }

    /// Plays a sound effect from raw file bytes (decoded at call time).
    pub fn play_bytes(&self, bytes: Vec<u8>, volume: f32) -> Result<(), AudioError> {
        let source =
            Decoder::new(Cursor::new(bytes)).map_err(|e| AudioError::DecodeFailed(e.to_string()))?;
        self.sink.append(source.amplify(volume.clamp(0.0, 1.0)));
        Ok(())
    }

    /// Plays a cached sound from a [`SoundBuffer`].
    pub fn play_cached(
        &self,
        buffer: &SoundBuffer,
        path: &str,
        volume: f32,
    ) -> Result<(), AudioError> {
        let source = buffer.decoder(path)?;
        self.sink.append(source.amplify(volume.clamp(0.0, 1.0)));
        Ok(())
    }

    /// Sets the sink volume directly (0.0–1.0).
    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume.clamp(0.0, 1.0));
    }

    /// Pauses all currently playing sound effects.
    pub fn pause(&self) {
        self.sink.pause();
    }

    /// Resumes paused sound effects.
    pub fn resume(&self) {
        self.sink.play();
    }

    /// Stops all sound effects and clears the queue.
    pub fn stop(&self) {
        self.sink.stop();
    }

    /// Returns true if no sounds are queued or playing.
    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }
}

// ──────────────────────────────────────────────
// Music State
// ──────────────────────────────────────────────

/// Playback state of the music player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicState {
    /// No track loaded.
    Idle,
    /// A track is playing normally.
    Playing,
    /// Crossfade from a previous track is in progress.
    Fading,
}

// ──────────────────────────────────────────────
// Music Player
// ──────────────────────────────────────────────

/// Streams long-form audio (background music) with crossfade support.
///
/// Crossfade ramps the old track's volume down while ramping the new
/// track up over a configurable duration. Call [`MusicPlayer::update`]
/// each frame to drive the fade transition.
pub struct MusicPlayer {
    sink: Sink,
    handle: OutputStreamHandle,
    state: MusicState,
    fading_sink: Option<Sink>,
    fade_timer: f32,
    fade_duration: f32,
    base_volume: f32,
}

impl std::fmt::Debug for MusicPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MusicPlayer")
            .field("state", &self.state)
            .field("fade_timer", &self.fade_timer)
            .field("fade_duration", &self.fade_duration)
            .field("base_volume", &self.base_volume)
            .finish()
    }
}

impl MusicPlayer {
    /// Creates a new music player attached to the given output stream handle.
    pub fn new(handle: &OutputStreamHandle) -> Result<Self, AudioError> {
        let sink =
            Sink::try_new(handle).map_err(|e| AudioError::NoDeviceAvailable(e.to_string()))?;
        Ok(MusicPlayer {
            sink,
            handle: handle.clone(),
            state: MusicState::Idle,
            fading_sink: None,
            fade_timer: 0.0,
            fade_duration: 0.0,
            base_volume: 0.8,
        })
    }

    /// Returns the current playback state.
    pub fn state(&self) -> MusicState {
        self.state
    }

    /// Starts playing a music track immediately (no fade).
    ///
    /// If something is already playing, it is stopped first.
    pub fn play(&mut self, path: &str, volume: f32) -> Result<(), AudioError> {
        self.stop();
        let bytes = std::fs::read(path).map_err(AudioError::FileError)?;
        let source =
            Decoder::new(Cursor::new(bytes)).map_err(|e| AudioError::DecodeFailed(e.to_string()))?;
        self.base_volume = volume.clamp(0.0, 1.0);
        self.sink.set_volume(self.base_volume);
        self.sink.append(source);
        self.state = MusicState::Playing;
        Ok(())
    }

    /// Starts playing a cached track immediately.
    pub fn play_cached(
        &mut self,
        buffer: &SoundBuffer,
        path: &str,
        volume: f32,
    ) -> Result<(), AudioError> {
        self.stop();
        let source = buffer.decoder(path)?;
        self.base_volume = volume.clamp(0.0, 1.0);
        self.sink.set_volume(self.base_volume);
        self.sink.append(source);
        self.state = MusicState::Playing;
        Ok(())
    }

    /// Crossfades to a new track over `fade_duration_secs` seconds.
    ///
    /// The old track fades out while the new track fades in.
    /// Call [`MusicPlayer::update`] each frame to drive the transition.
    pub fn crossfade_to(
        &mut self,
        path: &str,
        fade_duration_secs: f32,
        target_volume: f32,
    ) -> Result<(), AudioError> {
        let new_sink = Sink::try_new(&self.handle)
            .map_err(|e| AudioError::NoDeviceAvailable(e.to_string()))?;

        let bytes = std::fs::read(path).map_err(AudioError::FileError)?;
        let source =
            Decoder::new(Cursor::new(bytes)).map_err(|e| AudioError::DecodeFailed(e.to_string()))?;

        let vol = target_volume.clamp(0.0, 1.0);
        new_sink.set_volume(0.0);
        new_sink.append(source);

        // Move current sink into the fading role, promote new sink to primary.
        let old_sink = std::mem::replace(&mut self.sink, new_sink);
        self.fading_sink = Some(old_sink);

        self.base_volume = vol;
        self.fade_duration = fade_duration_secs.max(0.001);
        self.fade_timer = 0.0;
        self.state = MusicState::Fading;
        Ok(())
    }

    /// Drives the crossfade. Call once per frame with the frame delta time in seconds.
    pub fn update(&mut self, dt: f32) {
        if self.state != MusicState::Fading {
            return;
        }

        self.fade_timer += dt;
        let t = (self.fade_timer / self.fade_duration).min(1.0);

        // Fade in new track
        self.sink.set_volume(self.base_volume * t);

        // Fade out old track
        if let Some(ref old_sink) = self.fading_sink {
            old_sink.set_volume(self.base_volume * (1.0 - t));
        }

        if t >= 1.0 {
            if let Some(old_sink) = self.fading_sink.take() {
                old_sink.stop();
            }
            self.state = MusicState::Playing;
        }
    }

    /// Sets the volume on the active sink (0.0–1.0).
    ///
    /// During a crossfade, this updates the base volume that the fade
    /// ramp interpolates toward.
    pub fn set_volume(&mut self, volume: f32) {
        self.base_volume = volume.clamp(0.0, 1.0);
        if self.state != MusicState::Fading {
            self.sink.set_volume(self.base_volume);
        }
    }

    /// Pauses the current track.
    pub fn pause(&self) {
        self.sink.pause();
    }

    /// Resumes the current track.
    pub fn resume(&self) {
        self.sink.play();
    }

    /// Stops all music, including any crossfading tracks.
    pub fn stop(&mut self) {
        self.sink.stop();
        if let Some(old_sink) = self.fading_sink.take() {
            old_sink.stop();
        }
        self.state = MusicState::Idle;
        self.fade_timer = 0.0;
    }

    /// Returns true if the current track has finished playing.
    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }
}

// ──────────────────────────────────────────────
// Audio Engine
// ──────────────────────────────────────────────

/// Top-level audio engine. Owns the rodio output stream.
///
/// Create one at startup. The `_stream` field must stay alive for the
/// entire lifetime of the application — dropping it stops all audio.
pub struct AudioEngine {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    volume: VolumeControl,
    buffer: SoundBuffer,
}

impl std::fmt::Debug for AudioEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioEngine")
            .field("volume", &self.volume)
            .field("buffer_count", &self.buffer.len())
            .finish()
    }
}

impl AudioEngine {
    /// Initializes the audio engine by opening the default output device.
    ///
    /// Returns an error if no audio device is available.
    pub fn new() -> Result<Self, AudioError> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|e| AudioError::NoDeviceAvailable(e.to_string()))?;
        Ok(AudioEngine {
            _stream: stream,
            handle,
            volume: VolumeControl::default(),
            buffer: SoundBuffer::new(),
        })
    }

    /// Returns a reference to the volume control.
    pub fn volume(&self) -> &VolumeControl {
        &self.volume
    }

    /// Returns a mutable reference to the volume control.
    pub fn volume_mut(&mut self) -> &mut VolumeControl {
        &mut self.volume
    }

    /// Returns a reference to the sound buffer cache.
    pub fn buffer(&self) -> &SoundBuffer {
        &self.buffer
    }

    /// Returns a mutable reference to the sound buffer cache.
    pub fn buffer_mut(&mut self) -> &mut SoundBuffer {
        &mut self.buffer
    }

    /// Creates a new [`SfxPlayer`] on this engine's output stream.
    pub fn create_sfx_player(&self) -> Result<SfxPlayer, AudioError> {
        SfxPlayer::new(&self.handle)
    }

    /// Creates a new [`MusicPlayer`] on this engine's output stream.
    pub fn create_music_player(&self) -> Result<MusicPlayer, AudioError> {
        MusicPlayer::new(&self.handle)
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_control_clamps_values() {
        let vc = VolumeControl::new(1.5, -0.2, 0.5);
        assert!((vc.master - 1.0).abs() < f32::EPSILON);
        assert!((vc.music).abs() < f32::EPSILON);
        assert!((vc.sfx - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn effective_volume_applies_master_scaling() {
        let vc = VolumeControl::new(0.5, 0.8, 1.0);
        assert!((vc.effective_music() - 0.4).abs() < f32::EPSILON);
        assert!((vc.effective_sfx() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn spatial_attenuation_decreases_with_distance() {
        let listener = [0.0, 0.0, 0.0];
        let near = [1.0, 0.0, 0.0];
        let far = [10.0, 0.0, 0.0];
        let near_vol = SpatialAudio::compute_volume(listener, near, 1.0);
        let far_vol = SpatialAudio::compute_volume(listener, far, 1.0);
        assert!(near_vol > far_vol, "near sound should be louder than far");
        assert!(near_vol < 1.0, "non-zero distance should attenuate");
    }

    #[test]
    fn spatial_zero_rolloff_is_full_volume() {
        let vol = SpatialAudio::compute_volume([0.0; 3], [100.0; 3], 0.0);
        assert!((vol - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn spatial_same_position_is_full_volume() {
        let pos = [5.0, 3.0, 1.0];
        let vol = SpatialAudio::compute_volume(pos, pos, 1.0);
        assert!((vol - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn spatial_known_distance_formula() {
        // distance = 3.0, rolloff = 0.5 → 1.0 / (1.0 + 3.0 * 0.5) = 1.0 / 2.5 = 0.4
        let vol = SpatialAudio::compute_volume([0.0, 0.0, 0.0], [3.0, 0.0, 0.0], 0.5);
        assert!((vol - 0.4).abs() < 1e-5);
    }

    #[test]
    fn sound_buffer_caches_and_retrieves() {
        let mut buf = SoundBuffer::new();
        let tmp = "/tmp/chronos_test_audio_buffer.bin";
        std::fs::write(tmp, b"fake audio data").expect("write temp");

        buf.load(tmp).expect("load into cache");
        assert!(buf.contains(tmp));
        assert_eq!(buf.len(), 1);
        assert!(buf.get(tmp).is_some());

        buf.remove(tmp);
        assert!(!buf.contains(tmp));
        assert!(buf.is_empty());

        let _ = std::fs::remove_file(tmp);
    }

    #[test]
    fn sound_buffer_decoder_fails_on_missing_key() {
        let buf = SoundBuffer::new();
        let result = buf.decoder("nonexistent.wav");
        assert!(result.is_err());
    }

    #[test]
    fn music_state_transitions_are_distinct() {
        assert_ne!(MusicState::Idle, MusicState::Playing);
        assert_ne!(MusicState::Playing, MusicState::Fading);
        assert_ne!(MusicState::Idle, MusicState::Fading);

        let state = MusicState::Playing;
        let copy = state;
        assert_eq!(state, copy);
    }

    #[test]
    fn volume_control_default_sanity() {
        let vc = VolumeControl::default();
        assert!((vc.master - 1.0).abs() < f32::EPSILON);
        assert!((vc.music - 0.8).abs() < f32::EPSILON);
        assert!((vc.sfx - 0.8).abs() < f32::EPSILON);
    }
}
