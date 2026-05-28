// Audio import module for the Chronos Engine.
//
// Decodes WAV, OGG, MP3, and FLAC files into raw interleaved f32 PCM
// sample buffers using the symphonia crate. Supports metadata extraction,
// peak normalization, mono conversion, silence trimming, and basic
// linear-interpolation resampling.

#[cfg(feature = "asset-pipeline")]
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;

// ──────────────────────────────────────────────
// File Format
// ──────────────────────────────────────────────

/// Supported audio file formats for import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioFileFormat {
    Wav,
    Ogg,
    Mp3,
    Flac,
    Unknown,
}

// ──────────────────────────────────────────────
// Error Types
// ──────────────────────────────────────────────

/// Errors that can occur during audio import operations.
#[derive(Debug)]
pub enum AudioImportError {
    /// File I/O error.
    Io(std::io::Error),
    /// Failed to decode audio data.
    DecodeFailed(String),
    /// The audio format is not supported.
    UnsupportedFormat(String),
    /// No audio track found in the file.
    NoAudioTrack,
    /// Failed to resample audio to target sample rate.
    ResampleFailed(String),
}

impl std::fmt::Display for AudioImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioImportError::Io(err) => write!(f, "audio import I/O error: {}", err),
            AudioImportError::DecodeFailed(msg) => write!(f, "audio decode failed: {}", msg),
            AudioImportError::UnsupportedFormat(msg) => {
                write!(f, "unsupported audio format: {}", msg)
            }
            AudioImportError::NoAudioTrack => write!(f, "no audio track found in file"),
            AudioImportError::ResampleFailed(msg) => {
                write!(f, "audio resample failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for AudioImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AudioImportError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AudioImportError {
    fn from(err: std::io::Error) -> Self {
        AudioImportError::Io(err)
    }
}

// ──────────────────────────────────────────────
// Audio Metadata
// ──────────────────────────────────────────────

/// Metadata extracted from an audio file header.
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    /// Track title, if present in file tags.
    pub title: Option<String>,
    /// Artist name, if present in file tags.
    pub artist: Option<String>,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of audio channels.
    pub channels: u16,
    /// Bits per sample (e.g. 16 for PCM WAV).
    pub bit_depth: Option<u16>,
}

// ──────────────────────────────────────────────
// Import Settings
// ──────────────────────────────────────────────

/// Post-processing settings applied during audio import.
#[derive(Debug, Clone)]
pub struct AudioImportSettings {
    /// Normalize peak amplitude to 1.0.
    pub normalize: bool,
    /// Target sample rate in Hz. Resamples via linear interpolation if set.
    pub target_sample_rate: Option<u32>,
    /// Downmix multi-channel audio to mono.
    pub convert_to_mono: bool,
    /// Trim leading and trailing silence from the sample buffer.
    pub trim_silence: bool,
    /// Silence threshold in dB for trimming (default: -60.0).
    pub silence_threshold_db: f32,
}

impl Default for AudioImportSettings {
    fn default() -> Self {
        Self {
            normalize: false,
            target_sample_rate: None,
            convert_to_mono: false,
            trim_silence: false,
            silence_threshold_db: -60.0,
        }
    }
}

// ──────────────────────────────────────────────
// Imported Audio
// ──────────────────────────────────────────────

/// Fully decoded audio data ready for the engine's audio system.
///
/// Samples are stored as interleaved f32 PCM regardless of source format.
/// For stereo, samples alternate L, R, L, R, ...
#[derive(Debug, Clone)]
pub struct ImportedAudio {
    /// Interleaved PCM sample data normalised to [-1.0, 1.0].
    pub samples: Vec<f32>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of audio channels (1 = mono, 2 = stereo, etc.).
    pub channels: u16,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Detected file format.
    pub format: AudioFileFormat,
}

// ──────────────────────────────────────────────
// Audio Importer
// ──────────────────────────────────────────────

/// Audio file importer powered by symphonia.
///
/// Decodes WAV, OGG, MP3, and FLAC to interleaved f32 PCM. Supports
/// optional post-processing via `AudioImportSettings`.
pub struct AudioImporter;

impl AudioImporter {
    /// Detect the audio file format from the file extension.
    pub fn detect_format(path: &Path) -> AudioFileFormat {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
        {
            Some(ref ext) => match ext.as_str() {
                "wav" | "wave" => AudioFileFormat::Wav,
                "ogg" | "oga" => AudioFileFormat::Ogg,
                "mp3" => AudioFileFormat::Mp3,
                "flac" => AudioFileFormat::Flac,
                _ => AudioFileFormat::Unknown,
            },
            None => AudioFileFormat::Unknown,
        }
    }

    /// Import an audio file with default settings.
    pub fn import(path: &Path) -> Result<ImportedAudio, AudioImportError> {
        Self::import_with_settings(path, &AudioImportSettings::default())
    }

    /// Import an audio file with the specified post-processing settings.
    pub fn import_with_settings(
        path: &Path,
        settings: &AudioImportSettings,
    ) -> Result<ImportedAudio, AudioImportError> {
        let format = Self::detect_format(path);
        if format == AudioFileFormat::Unknown {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string();
            return Err(AudioImportError::UnsupportedFormat(ext));
        }

        let (mut samples, sample_rate, mut channels) = decode_file(path)?;

        // Apply settings in pipeline order
        if settings.convert_to_mono && channels > 1 {
            samples = convert_to_mono(&samples, channels);
            channels = 1;
        }

        if let Some(target_sr) = settings.target_sample_rate {
            if target_sr != sample_rate && target_sr > 0 && sample_rate > 0 {
                samples = resample(&samples, sample_rate, target_sr, channels);
            }
        }

        if settings.trim_silence {
            samples = trim_silence(&samples, channels, settings.silence_threshold_db);
        }

        if settings.normalize {
            normalize(&mut samples);
        }

        let final_sr = settings.target_sample_rate.unwrap_or(sample_rate);
        let total_frames = if channels > 0 {
            samples.len() / channels as usize
        } else {
            0
        };
        let duration_secs = if final_sr > 0 {
            total_frames as f64 / final_sr as f64
        } else {
            0.0
        };

        Ok(ImportedAudio {
            samples,
            sample_rate: final_sr,
            channels,
            duration_secs,
            format,
        })
    }

    /// Read metadata from an audio file without fully decoding it.
    ///
    /// Opens the file and probes the header for track info and tags.
    /// Does not decode any audio packets.
    pub fn read_metadata(path: &Path) -> Result<AudioMetadata, AudioImportError> {
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| AudioImportError::DecodeFailed(e.to_string()))?;

        let mut format_reader = probed.format;

        // Find the first audio track
        let track = format_reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(AudioImportError::NoAudioTrack)?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(0);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(0);

        let duration_secs = match (track.codec_params.n_frames, track.codec_params.sample_rate) {
            (Some(frames), Some(sr)) if sr > 0 => frames as f64 / sr as f64,
            _ => 0.0,
        };

        // Extract tag metadata if available
        let mut title = None;
        let mut artist = None;

        if let Some(rev) = format_reader.metadata().current() {
            for tag in rev.tags() {
                if let Some(key) = tag.std_key {
                    match key {
                        StandardTagKey::TrackTitle => {
                            title = extract_tag_string(tag);
                        }
                        StandardTagKey::Artist => {
                            artist = extract_tag_string(tag);
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(AudioMetadata {
            title,
            artist,
            duration_secs,
            sample_rate,
            channels,
            bit_depth: None,
        })
    }
}

// ──────────────────────────────────────────────
// Internal Helpers — Decoding
// ──────────────────────────────────────────────

/// Decode an audio file to interleaved f32 samples using symphonia.
fn decode_file(path: &Path) -> Result<(Vec<f32>, u32, u16), AudioImportError> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AudioImportError::DecodeFailed(format!("format probe failed: {}", e)))?;

    let mut format_reader = probed.format;

    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(AudioImportError::NoAudioTrack)?;

    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let num_channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioImportError::DecodeFailed(format!("decoder init failed: {}", e)))?;

    let mut samples: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format_reader.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                if samples.is_empty() {
                    return Err(AudioImportError::DecodeFailed(format!(
                        "packet read error: {}",
                        e
                    )));
                }
                break;
            }
        };

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => continue,
        };

        if sample_buf.is_none()
            || sample_buf.as_ref().map_or(true, |sb| sb.capacity() < decoded.capacity())
        {
            let spec = decoded.spec();
            sample_buf = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec.clone()));
        }

        if let Some(ref mut sb) = sample_buf {
            sb.copy_interleaved_ref(decoded);
            samples.extend_from_slice(sb.samples());
        }
    }

    if samples.is_empty() {
        return Err(AudioImportError::DecodeFailed(
            "no audio samples decoded from file".to_string(),
        ));
    }

    Ok((samples, sample_rate, num_channels as u16))
}

/// Try to extract a human-readable string from a symphonia metadata tag.
fn extract_tag_string(tag: &symphonia::core::meta::Tag) -> Option<String> {
    use symphonia::core::meta::Value as SymValue;

    match &tag.value {
        SymValue::String(s) => Some(s.clone()),
        SymValue::UnsignedInt(n) => Some(n.to_string()),
        SymValue::SignedInt(n) => Some(n.to_string()),
        SymValue::Float(f) => Some(f.to_string()),
        SymValue::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

// ──────────────────────────────────────────────
// Internal Helpers — Post-processing
// ──────────────────────────────────────────────

/// Normalize samples so the peak absolute value is 1.0.
fn normalize(samples: &mut [f32]) {
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    if peak > 0.0 {
        let scale = 1.0 / peak;
        for s in samples.iter_mut() {
            *s *= scale;
        }
    }
}

/// Downmix multi-channel audio to mono by averaging all channels per frame.
fn convert_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let ch = channels as usize;
    if ch == 0 || ch == 1 {
        return samples.to_vec();
    }
    let num_frames = samples.len() / ch;
    let mut mono = Vec::with_capacity(num_frames);
    for frame in 0..num_frames {
        let sum: f32 = (0..ch).map(|c| samples[frame * ch + c]).sum();
        mono.push(sum / ch as f32);
    }
    mono
}

/// Trim leading and trailing frames below the silence threshold.
fn trim_silence(samples: &[f32], channels: u16, threshold_db: f32) -> Vec<f32> {
    let ch = channels as usize;
    if ch == 0 || samples.is_empty() {
        return Vec::new();
    }

    let threshold_linear = 10.0f32.powf(threshold_db / 20.0);

    let first_loud = samples
        .chunks_exact(ch)
        .position(|frame| frame.iter().any(|s| s.abs() > threshold_linear));

    let first_loud = match first_loud {
        Some(i) => i,
        None => return Vec::new(), // Entire buffer is silence
    };

    let last_loud = samples
        .chunks_exact(ch)
        .rposition(|frame| frame.iter().any(|s| s.abs() > threshold_linear));

    let last_loud = match last_loud {
        Some(i) => i,
        None => return Vec::new(),
    };

    let start = first_loud * ch;
    let end = (last_loud + 1) * ch;
    samples[start..end].to_vec()
}

/// Resample audio via linear interpolation.
fn resample(samples: &[f32], source_rate: u32, target_rate: u32, channels: u16) -> Vec<f32> {
    if source_rate == target_rate || target_rate == 0 || source_rate == 0 {
        return samples.to_vec();
    }

    let ch = channels as usize;
    if ch == 0 {
        return Vec::new();
    }
    let src_frames = samples.len() / ch;
    if src_frames == 0 {
        return Vec::new();
    }

    let ratio = target_rate as f64 / source_rate as f64;
    let dst_frames = ((src_frames as f64) * ratio).round() as usize;
    if dst_frames == 0 {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(dst_frames * ch);

    for i in 0..dst_frames {
        let src_pos = i as f64 / ratio;
        let frame0 = (src_pos.floor() as usize).min(src_frames - 1);
        let frame1 = (frame0 + 1).min(src_frames - 1);
        let frac = src_pos - frame0 as f64;

        for c in 0..ch {
            let s0 = samples[frame0 * ch + c];
            let s1 = samples[frame1 * ch + c];
            output.push(s0 + (s1 - s0) * frac as f32);
        }
    }

    output
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const TEST_DIR: &str = "/tmp/chronos_import_tests/audio";

    fn ensure_test_dir() -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(TEST_DIR);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Write a minimal 16-bit PCM WAV file from i16 sample data.
    fn write_wav(
        path: &Path,
        samples: &[i16],
        channels: u16,
        sample_rate: u32,
    ) -> std::io::Result<()> {
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * channels as u32 * 2;
        let block_align = channels * 2;
        let data_size = (samples.len() * 2) as u32;
        let file_size = 36 + data_size;

        let mut f = std::fs::File::create(path)?;

        // RIFF header
        f.write_all(b"RIFF")?;
        f.write_all(&file_size.to_le_bytes())?;
        f.write_all(b"WAVE")?;

        // fmt chunk
        f.write_all(b"fmt ")?;
        f.write_all(&16u32.to_le_bytes())?;
        f.write_all(&1u16.to_le_bytes())?; // PCM format
        f.write_all(&channels.to_le_bytes())?;
        f.write_all(&sample_rate.to_le_bytes())?;
        f.write_all(&byte_rate.to_le_bytes())?;
        f.write_all(&block_align.to_le_bytes())?;
        f.write_all(&bits_per_sample.to_le_bytes())?;

        // data chunk
        f.write_all(b"data")?;
        f.write_all(&data_size.to_le_bytes())?;

        for s in samples {
            f.write_all(&s.to_le_bytes())?;
        }

        Ok(())
    }

    /// Convert f32 samples in [-1, 1] to i16.
    fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
        samples
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect()
    }

    /// Generate a sine wave at the given frequency and duration.
    fn sine_wave(freq: f32, sample_rate: u32, duration_secs: f64) -> Vec<f32> {
        let num_samples = (sample_rate as f64 * duration_secs).round() as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect()
    }

    // ── Test 1: Import a minimal WAV file ──

    #[test]
    fn import_minimal_wav_succeeds() {
        let dir = ensure_test_dir();
        let path = dir.join("minimal.wav");

        let samples_f = sine_wave(440.0, 44100, 0.05);
        let samples_i16 = f32_to_i16(&samples_f);
        write_wav(&path, &samples_i16, 1, 44100).expect("write wav");

        let result = AudioImporter::import(&path);
        assert!(result.is_ok(), "import should succeed: {:?}", result.err());

        let audio = result.unwrap();
        assert!(!audio.samples.is_empty());
        assert_eq!(audio.format, AudioFileFormat::Wav);
    }

    // ── Test 2: Verify sample rate ──

    #[test]
    fn import_wav_sample_rate() {
        let dir = ensure_test_dir();
        let path = dir.join("sr_test.wav");

        let samples_f = sine_wave(440.0, 22050, 0.02);
        let samples_i16 = f32_to_i16(&samples_f);
        write_wav(&path, &samples_i16, 1, 22050).expect("write wav");

        let audio = AudioImporter::import(&path).expect("import");
        assert_eq!(audio.sample_rate, 22050);
    }

    // ── Test 3: Verify channel count ──

    #[test]
    fn import_stereo_wav_channel_count() {
        let dir = ensure_test_dir();
        let path = dir.join("stereo.wav");

        // Interleave L and R (same sine wave on both channels)
        let mono = sine_wave(440.0, 44100, 0.02);
        let mut stereo_f = Vec::with_capacity(mono.len() * 2);
        for &s in &mono {
            stereo_f.push(s);
            stereo_f.push(s);
        }
        let stereo_i16 = f32_to_i16(&stereo_f);
        write_wav(&path, &stereo_i16, 2, 44100).expect("write wav");

        let audio = AudioImporter::import(&path).expect("import");
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.samples.len() % 2, 0);
    }

    // ── Test 4: Normalize to peak 1.0 ──

    #[test]
    fn import_wav_normalized() {
        let dir = ensure_test_dir();
        let path = dir.join("quiet.wav");

        // Very quiet sine wave (amplitude 0.1)
        let mut quiet: Vec<f32> = sine_wave(440.0, 44100, 0.05);
        for s in quiet.iter_mut() {
            *s *= 0.1;
        }
        let samples_i16 = f32_to_i16(&quiet);
        write_wav(&path, &samples_i16, 1, 44100).expect("write wav");

        let settings = AudioImportSettings {
            normalize: true,
            ..Default::default()
        };

        let audio = AudioImporter::import_with_settings(&path, &settings).expect("import");
        let peak = audio.samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            (peak - 1.0).abs() < 0.05,
            "peak should be ~1.0 after normalization, got {}",
            peak
        );
    }

    // ── Test 5: Convert stereo to mono ──

    #[test]
    fn import_wav_convert_to_mono() {
        let dir = ensure_test_dir();
        let path = dir.join("to_mono.wav");

        // L = 1.0, R = -1.0 → mono should be 0.0
        let num_frames = 100;
        let mut stereo_f = Vec::with_capacity(num_frames * 2);
        for _ in 0..num_frames {
            stereo_f.push(1.0f32);
            stereo_f.push(-1.0f32);
        }
        let stereo_i16 = f32_to_i16(&stereo_f);
        write_wav(&path, &stereo_i16, 2, 44100).expect("write wav");

        let settings = AudioImportSettings {
            convert_to_mono: true,
            ..Default::default()
        };

        let audio = AudioImporter::import_with_settings(&path, &settings).expect("import");
        assert_eq!(audio.channels, 1);
        // Each mono sample should be (1.0 + (-1.0)) / 2 = 0.0
        // Allow quantization error from i16 round-trip
        for &s in &audio.samples {
            assert!(
                s.abs() < 0.1,
                "mono sample should be near 0.0, got {}",
                s
            );
        }
    }

    // ── Test 6: detect_format .wav ──

    #[test]
    fn detect_format_wav() {
        let p = std::path::PathBuf::from("/some/path/audio.wav");
        assert_eq!(AudioImporter::detect_format(&p), AudioFileFormat::Wav);

        let p2 = std::path::PathBuf::from("/some/path/audio.WAV");
        assert_eq!(AudioImporter::detect_format(&p2), AudioFileFormat::Wav);
    }

    // ── Test 7: detect_format .mp3 ──

    #[test]
    fn detect_format_mp3() {
        let p = std::path::PathBuf::from("/music/track.mp3");
        assert_eq!(AudioImporter::detect_format(&p), AudioFileFormat::Mp3);
    }

    // ── Test 8: detect_format .flac ──

    #[test]
    fn detect_format_flac() {
        let p = std::path::PathBuf::from("/lossless/album.flac");
        assert_eq!(AudioImporter::detect_format(&p), AudioFileFormat::Flac);
    }

    // ── Test 9: Error on nonexistent file ──

    #[test]
    fn error_on_nonexistent_file() {
        let p = std::path::PathBuf::from("/tmp/chronos_import_tests/does_not_exist.wav");
        let result = AudioImporter::import(&p);
        assert!(result.is_err());
        match result.unwrap_err() {
            AudioImportError::Io(_) => {} // expected
            other => panic!("expected Io error, got: {:?}", other),
        }
    }

    // ── Test 10: Error on invalid audio content ──

    #[test]
    fn error_on_invalid_audio_content() {
        let dir = ensure_test_dir();
        let path = dir.join("garbage.wav");
        std::fs::write(&path, b"RIFF\x00\x00\x00\x00WAVEgarbage data here\x00\x00")
            .expect("write garbage");

        let result = AudioImporter::import(&path);
        assert!(result.is_err(), "should fail on garbage content");
    }

    // ── Test 11: read_metadata returns correct duration ──

    #[test]
    fn read_metadata_duration() {
        let dir = ensure_test_dir();
        let path = dir.join("meta_test.wav");

        // 0.5 seconds of audio at 44100 Hz mono
        let num_samples = 44100 / 2; // 22050 samples = 0.5s
        let samples: Vec<i16> = (0..num_samples).map(|i| {
            let t = i as f32 / 44100.0;
            ((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 32767.0) as i16
        }).collect();
        write_wav(&path, &samples, 1, 44100).expect("write wav");

        let meta = AudioImporter::read_metadata(&path).expect("read_metadata");
        assert_eq!(meta.sample_rate, 44100);
        assert_eq!(meta.channels, 1);
        // Allow small floating-point tolerance
        let expected_duration = 22050.0 / 44100.0;
        assert!(
            (meta.duration_secs - expected_duration).abs() < 0.01,
            "duration should be ~{}, got {}",
            expected_duration,
            meta.duration_secs
        );
    }

    // ── Test 12: trim_silence removes leading/trailing quiet samples ──

    #[test]
    fn trim_silence_removes_quiet_regions() {
        let dir = ensure_test_dir();
        let path = dir.join("trim_test.wav");

        // Build: silence (0.0) + loud signal + silence (0.0)
        let silence_len = 100;
        let loud_len = 50;
        let mut samples_f: Vec<f32> = vec![0.0; silence_len]; // leading silence
        for i in 0..loud_len {
            let t = i as f32 / 44100.0;
            samples_f.push((2.0 * std::f32::consts::PI * 440.0 * t).sin());
        }
        samples_f.extend_from_slice(&vec![0.0; silence_len]); // trailing silence

        let samples_i16 = f32_to_i16(&samples_f);
        write_wav(&path, &samples_i16, 1, 44100).expect("write wav");

        let settings = AudioImportSettings {
            trim_silence: true,
            silence_threshold_db: -40.0,
            ..Default::default()
        };

        let audio = AudioImporter::import_with_settings(&path, &settings).expect("import");
        // The trimmed audio should have significantly fewer samples than the original
        assert!(
            audio.samples.len() < samples_f.len(),
            "trimmed {} should be less than original {}",
            audio.samples.len(),
            samples_f.len()
        );
        // But should still have the loud section
        assert!(
            !audio.samples.is_empty(),
            "should still have audio after trimming"
        );
    }

    // ── Test 13: detect_format .ogg ──

    #[test]
    fn detect_format_ogg() {
        let p = std::path::PathBuf::from("/audio/sound.ogg");
        assert_eq!(AudioImporter::detect_format(&p), AudioFileFormat::Ogg);
    }

    // ── Test 14: detect_format unknown extension ──

    #[test]
    fn detect_format_unknown() {
        let p = std::path::PathBuf::from("/audio/file.xyz");
        assert_eq!(AudioImporter::detect_format(&p), AudioFileFormat::Unknown);

        let p2 = std::path::PathBuf::from("/audio/no_extension");
        assert_eq!(AudioImporter::detect_format(&p2), AudioFileFormat::Unknown);
    }

    // ── Test 15: resample changes sample count correctly ──

    #[test]
    fn resample_changes_length() {
        let dir = ensure_test_dir();
        let path = dir.join("resample_test.wav");

        // 0.1 seconds at 44100 Hz mono
        let samples_f = sine_wave(440.0, 44100, 0.1);
        let samples_i16 = f32_to_i16(&samples_f);
        write_wav(&path, &samples_i16, 1, 44100).expect("write wav");

        let settings = AudioImportSettings {
            target_sample_rate: Some(22050),
            ..Default::default()
        };

        let audio = AudioImporter::import_with_settings(&path, &settings).expect("import");
        assert_eq!(audio.sample_rate, 22050);
        // Should have roughly half the samples (22050 vs 44100)
        let original_len = samples_f.len();
        let expected_len = (original_len as f64 * 22050.0 / 44100.0).round() as usize;
        assert!(
            (audio.samples.len() as i64 - expected_len as i64).abs() <= 2,
            "resampled length {} should be close to {}",
            audio.samples.len(),
            expected_len
        );
    }
}
