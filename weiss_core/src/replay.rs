use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;
use anyhow::{Context, Result};
use crate::legal::ActionDesc;
use crate::db::{CardId, TriggerIcon};
use crate::events::{ChoiceOptionSummary, ChoiceSkipReason, ModifierRemoveReason, RevealAudience, RevealReason, TriggerCancelReason, Zone};
use crate::state::{ChoiceOptionRef, ChoiceReason, DamageModifierKind, DamageType, ModifierDuration, ModifierKind, TriggerEffect};

const MAGIC: &[u8; 4] = b"WSR1";
pub const REPLAY_SCHEMA_VERSION: u32 = 5;

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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepMeta {
    pub actor: u8,
    pub decision_kind: crate::legal::DecisionKind,
    pub illegal_action: bool,
    pub engine_error: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReplayEvent {
    Draw { player: u8, card: CardId },
    Damage { player: u8, card: CardId },
    DamageCancel { player: u8 },
    DamageIntent { event_id: u32, source_player: u8, source_slot: Option<u8>, target: u8, amount: i32, damage_type: DamageType, cancelable: bool },
    DamageModifierApplied { event_id: u32, modifier: DamageModifierKind, before_amount: i32, after_amount: i32, before_cancelable: bool, after_cancelable: bool, before_canceled: bool, after_canceled: bool },
    DamageModified { event_id: u32, target: u8, original: i32, modified: i32, canceled: bool, damage_type: DamageType },
    DamageCommitted { event_id: u32, target: u8, card: CardId, damage_type: DamageType },
    ReversalCommitted { player: u8, slot: u8, cause_damage_event: Option<u32> },
    Reveal { player: u8, card: CardId, reason: RevealReason, audience: RevealAudience },
    TriggerQueued { trigger_id: u32, group_id: u32, player: u8, source: CardId, effect: TriggerEffect },
    TriggerResolved { trigger_id: u32, player: u8, effect: TriggerEffect },
    TriggerCanceled { trigger_id: u32, player: u8, reason: TriggerCancelReason },
    ChoicePresented { choice_id: u32, player: u8, reason: ChoiceReason, options: Vec<ChoiceOptionSummary>, total_candidates: u16 },
    ChoiceMade { choice_id: u32, player: u8, option: ChoiceOptionRef },
    ChoiceAutopicked { choice_id: u32, player: u8, option: ChoiceOptionRef },
    ChoiceSkipped { choice_id: u32, player: u8, reason: ChoiceReason, skip_reason: ChoiceSkipReason },
    ZoneMove { player: u8, card: CardId, from: Zone, to: Zone, from_slot: Option<u8>, to_slot: Option<u8> },
    ModifierAdded { id: u32, source: CardId, target_player: u8, target_slot: u8, target_card: CardId, kind: ModifierKind, magnitude: i32, duration: ModifierDuration },
    ModifierRemoved { id: u32, reason: ModifierRemoveReason },
    Play { player: u8, card: CardId, slot: u8 },
    PlayEvent { player: u8, card: CardId },
    PlayClimax { player: u8, card: CardId },
    Trigger { player: u8, icon: TriggerIcon, card: Option<CardId> },
    Attack { player: u8, slot: u8 },
    AttackType { player: u8, attacker_slot: u8, attack_type: crate::state::AttackType },
    Counter { player: u8, card: CardId, power: i32 },
    Clock { player: u8, card: Option<CardId> },
    Refresh { player: u8 },
    RefreshPenalty { player: u8, card: CardId },
    LevelUpChoice { player: u8, card: CardId },
    Encore { player: u8, slot: u8, kept: bool },
    Stand { player: u8 },
    EndTurn { player: u8 },
    Terminal { winner: Option<u8> },
}

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
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sample_rate: 0.0,
            out_dir: PathBuf::from("replays"),
            compress: false,
            include_trigger_card_id: false,
        }
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
            for (counter, data) in (0_u64..).zip(rx.into_iter()) {
                let filename = format!("episode_{:08}.wsr", counter);
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
