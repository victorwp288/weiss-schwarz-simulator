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
/// Current replay schema version.
pub const REPLAY_SCHEMA_VERSION: u32 = 2;
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
    let base = postcard::to_stdvec(data)?;
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
    let mut file = File::create(path)?;
    file.write_all(MAGIC)?;
    let flags: u8 = if compress { 1 } else { 0 };
    file.write_all(&[flags])?;
    let len = payload.len() as u32;
    file.write_all(&len.to_le_bytes())?;
    file.write_all(&payload)?;
    Ok(())
}

/// Read and decode a replay file from disk.
pub fn read_replay_file(path: &Path) -> Result<ReplayData> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != MAGIC {
        anyhow::bail!("Invalid replay magic");
    }
    let mut flag = [0u8; 1];
    file.read_exact(&mut flag)?;
    let mut len_bytes = [0u8; 4];
    file.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    let mut payload = vec![0u8; len];
    file.read_exact(&mut payload)?;
    let compressed = (flag[0] & 1) == 1;
    if compressed {
        #[cfg(feature = "replay-zstd")]
        {
            payload = zstd::stream::decode_all(&payload[..])?;
        }
        #[cfg(not(feature = "replay-zstd"))]
        {
            anyhow::bail!("Replay file is compressed but replay-zstd feature is disabled");
        }
    }
    let data: ReplayData = postcard::from_bytes(&payload)?;
    Ok(data)
}
