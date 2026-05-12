use crate::events::Event;
use crate::legal::ActionDesc;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;

const MAGIC: &[u8; 4] = b"WSR1";
const FLAG_COMPRESSED: u8 = 1 << 0;
const FLAG_PAYLOAD_LEN_U64: u8 = 1 << 1;
const MAX_REPLAY_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
/// Current replay schema version.
pub const REPLAY_SCHEMA_VERSION: u32 = 3;
/// Sentinel id for unknown or unmappable actions in replays.
pub const REPLAY_ACTION_ID_UNKNOWN: u16 = u16::MAX;

/// Replay visibility mode for stored events and actions.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplayVisibilityMode {
    /// Full visibility with private information.
    Full,
    /// Public-safe visibility with sanitization.
    Public,
}

/// Per-episode replay header metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpisodeHeader {
    /// Observation encoding version.
    pub obs_version: u32,
    /// Action encoding version.
    pub action_version: u32,
    /// Replay schema version.
    pub replay_version: u32,
    /// Base seed used for the episode.
    pub seed: u64,
    #[serde(default)]
    /// Parent base seed (when episodes are derived).
    pub base_seed: u64,
    #[serde(default)]
    /// Per-episode derived seed.
    pub episode_seed: u64,
    #[serde(default)]
    /// Combined encoding spec hash.
    pub spec_hash: u64,
    /// Starting player for the episode.
    pub starting_player: u8,
    /// Deck ids used for both players.
    pub deck_ids: [u32; 2],
    /// Curriculum identifier (for experiment tracking).
    pub curriculum_id: String,
    /// Config hash for reproducibility.
    pub config_hash: u64,
    #[serde(default)]
    /// Fingerprint algorithm identifier.
    pub fingerprint_algo: String,
    #[serde(default)]
    /// Environment id within a pool.
    pub env_id: u32,
    #[serde(default)]
    /// Episode index within the environment.
    pub episode_index: u32,
}

/// Per-decision replay metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepMeta {
    /// Actor for the decision.
    pub actor: u8,
    /// Decision kind at this step.
    pub decision_kind: crate::legal::DecisionKind,
    /// Whether the applied action was illegal.
    pub illegal_action: bool,
    /// Whether an engine error occurred.
    pub engine_error: bool,
    /// Whether the applied action was a main-phase move.
    #[serde(default)]
    pub main_move_action: bool,
    /// Whether the applied action was a main-phase pass.
    #[serde(default)]
    pub main_pass_action: bool,
}

/// Replay event type alias.
pub type ReplayEvent = Event;

/// Final episode summary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayFinal {
    /// Terminal result, if any.
    pub terminal: Option<crate::state::TerminalResult>,
    /// State fingerprint at end of episode.
    pub state_hash: u64,
    /// Total decision count.
    pub decision_count: u32,
    /// Total tick count.
    pub tick_count: u32,
}

/// Replay payload body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpisodeBody {
    /// Canonical action descriptors.
    pub actions: Vec<ActionDesc>,
    #[serde(default)]
    /// Action ids aligned with `actions` where available.
    pub action_ids: Vec<u16>,
    /// Optional event list (when recording is enabled).
    pub events: Option<Vec<ReplayEvent>>,
    /// Per-decision metadata.
    pub steps: Vec<StepMeta>,
    /// Optional final-state summary.
    pub final_state: Option<ReplayFinal>,
}

/// Full replay payload (header + body).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayData {
    /// Header metadata.
    pub header: EpisodeHeader,
    /// Episode body.
    pub body: EpisodeBody,
}

/// Replay sampling and storage configuration.
#[derive(Clone, Debug)]
pub struct ReplayConfig {
    /// Whether replay recording is enabled.
    pub enabled: bool,
    /// Sampling rate in 0..=1.
    pub sample_rate: f32,
    /// Output directory for replay files.
    pub out_dir: PathBuf,
    /// Whether to compress replay payloads.
    pub compress: bool,
    /// Include trigger card id in event payloads.
    pub include_trigger_card_id: bool,
    /// Visibility mode for stored events/actions.
    pub visibility_mode: ReplayVisibilityMode,
    /// Store actions in the replay output.
    pub store_actions: bool,
    /// Cached threshold derived from sample_rate.
    pub sample_threshold: u32,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        let mut config = Self {
            enabled: false,
            sample_rate: 0.0,
            out_dir: PathBuf::from("replays"),
            compress: false,
            include_trigger_card_id: false,
            visibility_mode: ReplayVisibilityMode::Public,
            store_actions: true,
            sample_threshold: 0,
        };
        config.rebuild_cache();
        config
    }
}

impl ReplayConfig {
    /// Recompute cached sampling threshold after changing `sample_rate`.
    pub fn rebuild_cache(&mut self) {
        let rate = self.sample_rate.clamp(0.0, 1.0);
        self.sample_threshold = if rate <= 0.0 {
            0
        } else if rate >= 1.0 {
            u32::MAX
        } else {
            (rate * (u32::MAX as f32)).round() as u32
        };
    }
}

/// Background replay writer that serializes episodes to disk.
#[derive(Clone)]
pub struct ReplayWriter {
    sender: Sender<ReplayData>,
}

impl ReplayWriter {
    /// Spawn a background writer for the given config.
    pub fn new(config: &ReplayConfig) -> Result<Self> {
        fs::create_dir_all(&config.out_dir).context("Failed to create replay output directory")?;
        let (tx, rx) = mpsc::channel::<ReplayData>();
        let out_dir = config.out_dir.clone();
        let compress = config.compress;
        thread::spawn(move || {
            for data in rx.into_iter() {
                let header = &data.header;
                let filename = format!(
                    "episode_{:04}_{:08}_{:016x}.wsr",
                    header.env_id, header.episode_index, header.seed
                );
                let path = out_dir.join(filename);
                if let Err(err) = write_replay_file(&path, &data, compress) {
                    eprintln!("Replay write failed: {err}");
                }
            }
        });
        Ok(Self { sender: tx })
    }

    /// Enqueue replay data for async write.
    #[allow(clippy::result_large_err)]
    pub fn send(&self, data: ReplayData) -> std::result::Result<(), mpsc::SendError<ReplayData>> {
        self.sender.send(data)
    }
}

fn write_replay_file(path: &Path, data: &ReplayData, compress: bool) -> Result<()> {
    // Write to a sidecar temp file first and atomically rename into place so
    // readers never observe partially-written replay payloads.
    // `sync_all` before rename keeps crash windows bounded to either "old file"
    // or "complete new file", never torn replay bytes.
    let mut tmp_path = path.to_path_buf();
    let tmp_extension = path
        .extension()
        .map(|ext| format!("{}.tmp", ext.to_string_lossy()))
        .unwrap_or_else(|| "tmp".to_string());
    tmp_path.set_extension(tmp_extension);
    let mut file = File::create(&tmp_path)?;
    write_replay_to_writer(&mut file, data, compress)?;
    file.flush()?;
    file.sync_all()?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

fn write_replay_to_writer<W: Write>(
    writer: &mut W,
    data: &ReplayData,
    compress: bool,
) -> Result<()> {
    // Replay payload bytes are postcard-encoded from a fixed schema and then
    // wrapped in a stable framing header (magic, flags, explicit len). This
    // preserves deterministic binary output for the same `ReplayData`.
    let base = postcard::to_stdvec(data)?;
    if base.len() > MAX_REPLAY_PAYLOAD_BYTES {
        anyhow::bail!(
            "Replay payload length {} exceeds maximum {MAX_REPLAY_PAYLOAD_BYTES} bytes",
            base.len()
        );
    }
    let payload = if compress {
        #[cfg(feature = "replay-zstd")]
        {
            zstd::stream::encode_all(&base[..], 3)?
        }
        #[cfg(not(feature = "replay-zstd"))]
        {
            anyhow::bail!("Replay compression requested but replay-zstd feature is disabled");
        }
    } else {
        base
    };
    if payload.len() > MAX_REPLAY_PAYLOAD_BYTES {
        anyhow::bail!(
            "Replay encoded payload length {} exceeds maximum {MAX_REPLAY_PAYLOAD_BYTES} bytes",
            payload.len()
        );
    }
    let mut len_bytes = Vec::with_capacity(8);
    let len_flag = write_payload_len(&mut len_bytes, payload.len())?;
    let mut flags = if compress { FLAG_COMPRESSED } else { 0 };
    flags |= len_flag;
    writer.write_all(MAGIC)?;
    writer.write_all(&[flags])?;
    writer.write_all(&len_bytes)?;
    writer.write_all(&payload)?;
    Ok(())
}

fn write_payload_len<W: Write>(writer: &mut W, payload_len: usize) -> Result<u8> {
    let len = u64::try_from(payload_len).context("Replay payload length exceeds u64 range")?;
    if len > u64::from(u32::MAX) {
        writer.write_all(&len.to_le_bytes())?;
        Ok(FLAG_PAYLOAD_LEN_U64)
    } else {
        writer.write_all(&(len as u32).to_le_bytes())?;
        Ok(0)
    }
}

fn read_payload_len<R: Read>(reader: &mut R, flags: u8) -> Result<usize> {
    let len = if (flags & FLAG_PAYLOAD_LEN_U64) != 0 {
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        u64::from_le_bytes(len_bytes)
    } else {
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)?;
        u64::from(u32::from_le_bytes(len_bytes))
    };
    let len = usize::try_from(len).context("Replay payload length exceeds platform limits")?;
    if len > MAX_REPLAY_PAYLOAD_BYTES {
        anyhow::bail!(
            "Replay payload length {len} exceeds maximum {MAX_REPLAY_PAYLOAD_BYTES} bytes"
        );
    }
    Ok(len)
}

/// Read and decode a replay file from disk.
pub fn read_replay_file(path: &Path) -> Result<ReplayData> {
    let mut file = File::open(path)?;
    read_replay_from_reader(&mut file)
}

fn read_replay_from_reader<R: Read>(reader: &mut R) -> Result<ReplayData> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        anyhow::bail!("Invalid replay magic");
    }
    let mut flag = [0u8; 1];
    reader.read_exact(&mut flag)?;
    let flags = flag[0];
    let len = read_payload_len(reader, flags)?;
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    let compressed = (flags & FLAG_COMPRESSED) != 0;
    if compressed {
        #[cfg(feature = "replay-zstd")]
        {
            let decoder = zstd::stream::read::Decoder::new(&payload[..])?;
            let mut limited = decoder.take((MAX_REPLAY_PAYLOAD_BYTES as u64) + 1);
            let mut decoded = Vec::new();
            limited.read_to_end(&mut decoded)?;
            if decoded.len() > MAX_REPLAY_PAYLOAD_BYTES {
                anyhow::bail!(
                    "Replay decompressed payload exceeds maximum {MAX_REPLAY_PAYLOAD_BYTES} bytes"
                );
            }
            payload = decoded;
        }
        #[cfg(not(feature = "replay-zstd"))]
        {
            anyhow::bail!("Replay file is compressed but replay-zstd feature is disabled");
        }
    }
    let data: ReplayData = postcard::from_bytes(&payload)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(suffix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        path.push(format!(
            "weiss_core_replay_{suffix}_{}_{}",
            std::process::id(),
            ts
        ));
        path
    }

    fn sample_replay_data() -> ReplayData {
        ReplayData {
            header: EpisodeHeader {
                obs_version: 1,
                action_version: 1,
                replay_version: REPLAY_SCHEMA_VERSION,
                seed: 7,
                base_seed: 0,
                episode_seed: 0,
                spec_hash: 0,
                starting_player: 0,
                deck_ids: [11, 22],
                curriculum_id: "test".to_string(),
                config_hash: 99,
                fingerprint_algo: String::new(),
                env_id: 3,
                episode_index: 4,
            },
            body: EpisodeBody {
                actions: vec![ActionDesc::Pass],
                action_ids: vec![17],
                events: None,
                steps: vec![StepMeta {
                    actor: 0,
                    decision_kind: crate::legal::DecisionKind::Main,
                    illegal_action: false,
                    engine_error: false,
                    main_move_action: false,
                    main_pass_action: true,
                }],
                final_state: Some(ReplayFinal {
                    terminal: None,
                    state_hash: 123,
                    decision_count: 1,
                    tick_count: 2,
                }),
            },
        }
    }

    fn assert_replay_eq(actual: &ReplayData, expected: &ReplayData) {
        let actual_bytes = postcard::to_stdvec(actual).expect("serialize actual");
        let expected_bytes = postcard::to_stdvec(expected).expect("serialize expected");
        assert_eq!(actual_bytes, expected_bytes);
    }

    #[test]
    fn replay_reader_accepts_legacy_u32_payload_len() {
        let replay = sample_replay_data();
        let payload = postcard::to_stdvec(&replay).expect("serialize replay");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.push(0);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);
        let decoded =
            read_replay_from_reader(&mut Cursor::new(bytes)).expect("decode legacy replay");
        assert_replay_eq(&decoded, &replay);
    }

    #[test]
    fn replay_reader_accepts_u64_payload_len_header() {
        let replay = sample_replay_data();
        let payload = postcard::to_stdvec(&replay).expect("serialize replay");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.push(FLAG_PAYLOAD_LEN_U64);
        bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&payload);
        let decoded = read_replay_from_reader(&mut Cursor::new(bytes)).expect("decode replay");
        assert_replay_eq(&decoded, &replay);
    }

    #[test]
    fn replay_reader_rejects_oversized_u64_payload_len_before_allocating() {
        let len = (MAX_REPLAY_PAYLOAD_BYTES as u64) + 1;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.push(FLAG_PAYLOAD_LEN_U64);
        bytes.extend_from_slice(&len.to_le_bytes());

        let err = read_replay_from_reader(&mut Cursor::new(bytes)).expect_err("oversize replay");
        let msg = err.to_string();
        assert!(
            msg.contains("exceeds maximum"),
            "unexpected oversize replay error: {msg}"
        );
    }

    #[test]
    fn replay_reader_rejects_oversized_legacy_payload_len_before_allocating() {
        let len = (MAX_REPLAY_PAYLOAD_BYTES as u32) + 1;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.push(0);
        bytes.extend_from_slice(&len.to_le_bytes());

        let err = read_replay_from_reader(&mut Cursor::new(bytes)).expect_err("oversize replay");
        let msg = err.to_string();
        assert!(
            msg.contains("exceeds maximum"),
            "unexpected oversize replay error: {msg}"
        );
    }

    #[test]
    fn replay_write_read_roundtrip_small_payload() {
        let replay = sample_replay_data();
        let mut bytes = Vec::new();
        write_replay_to_writer(&mut bytes, &replay, false).expect("write replay");
        let decoded = read_replay_from_reader(&mut Cursor::new(bytes)).expect("read replay");
        assert_replay_eq(&decoded, &replay);
    }

    #[test]
    fn replay_config_rebuild_cache_clamps_sample_rate() {
        let mut cfg = ReplayConfig {
            sample_rate: -0.25,
            ..ReplayConfig::default()
        };

        cfg.rebuild_cache();
        assert_eq!(cfg.sample_threshold, 0);

        cfg.sample_rate = 0.0;
        cfg.rebuild_cache();
        assert_eq!(cfg.sample_threshold, 0);

        cfg.sample_rate = 0.5;
        cfg.rebuild_cache();
        let expected_half = (0.5f32 * (u32::MAX as f32)).round() as u32;
        assert_eq!(cfg.sample_threshold, expected_half);

        cfg.sample_rate = 1.0;
        cfg.rebuild_cache();
        assert_eq!(cfg.sample_threshold, u32::MAX);

        cfg.sample_rate = 1.25;
        cfg.rebuild_cache();
        assert_eq!(cfg.sample_threshold, u32::MAX);
    }

    #[test]
    fn replay_writer_new_creates_output_directory_and_accepts_send() {
        let out_dir = unique_temp_path("writer_ok");
        let cfg = ReplayConfig {
            enabled: true,
            out_dir: out_dir.clone(),
            ..ReplayConfig::default()
        };
        let writer = ReplayWriter::new(&cfg).expect("writer should create output directory");
        assert!(out_dir.is_dir(), "writer did not create output directory");
        writer
            .send(sample_replay_data())
            .expect("writer channel should accept replay payload");

        drop(writer);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = std::fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn replay_writer_new_surfaces_directory_creation_errors() {
        let file_path = unique_temp_path("writer_err");
        std::fs::write(&file_path, b"not a directory").expect("write temp file");
        let cfg = ReplayConfig {
            out_dir: file_path.clone(),
            ..ReplayConfig::default()
        };

        let err = match ReplayWriter::new(&cfg) {
            Err(err) => err,
            Ok(_) => panic!("expected create_dir_all failure"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("Failed to create replay output directory"),
            "unexpected error: {msg}"
        );

        let _ = std::fs::remove_file(&file_path);
    }
}
