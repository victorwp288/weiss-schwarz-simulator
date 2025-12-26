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
pub const REPLAY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpisodeHeader {
    pub obs_version: u32,
    pub action_version: u32,
    pub replay_version: u32,
    pub seed: u64,
    pub starting_player: u8,
    pub deck_ids: [u32; 2],
    pub curriculum_id: String,
    pub config_hash: u64,
    #[serde(default)]
    pub fingerprint_algo: String,
    #[serde(default)]
    pub env_id: u32,
    #[serde(default)]
    pub episode_index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepMeta {
    pub actor: u8,
    pub decision_kind: crate::legal::DecisionKind,
    pub illegal_action: bool,
    pub engine_error: bool,
}

pub type ReplayEvent = Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayFinal {
    pub terminal: Option<crate::state::TerminalResult>,
    pub state_hash: u64,
    pub decision_count: u32,
    pub tick_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpisodeBody {
    pub actions: Vec<ActionDesc>,
    pub events: Option<Vec<ReplayEvent>>,
    pub steps: Vec<StepMeta>,
    pub final_state: Option<ReplayFinal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayData {
    pub header: EpisodeHeader,
    pub body: EpisodeBody,
}

#[derive(Clone, Debug)]
pub struct ReplayConfig {
    pub enabled: bool,
    pub sample_rate: f32,
    pub out_dir: PathBuf,
    pub compress: bool,
    pub include_trigger_card_id: bool,
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
            sample_threshold: 0,
        };
        config.rebuild_cache();
        config
    }
}

impl ReplayConfig {
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

#[derive(Clone)]
pub struct ReplayWriter {
    sender: Sender<ReplayData>,
}

impl ReplayWriter {
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

    pub fn send(&self, data: ReplayData) {
        let _ = self.sender.send(data);
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
