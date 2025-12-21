use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const WSDB_MAGIC: &[u8; 4] = b"WSDB";
pub const WSDB_SCHEMA_VERSION: u32 = 1;

pub type CardId = u32;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardType {
    Character,
    Event,
    Climax,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardColor {
    Yellow,
    Green,
    Red,
    Blue,
    Colorless,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TriggerIcon {
    Soul,
    Shot,
    Bounce,
    Draw,
    Treasure,
    Gate,
    Standby,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AbilityTemplate {
    Vanilla,
    ContinuousPower { amount: i32 },
    ContinuousCannotAttack,
    ContinuousAttackCost { cost: u8 },
    AutoOnPlayDraw { count: u8 },
    AutoOnAttackDealDamage { amount: u8, cancelable: bool },
    AutoEndPhaseDraw { count: u8 },
    EventDealDamage { amount: u8, cancelable: bool },
    ActivatedPlaceholder,
    CounterBackup { power: i32 },
    CounterDamageReduce { amount: u8 },
    CounterDamageCancel,
    Unsupported { id: u32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardStatic {
    pub id: CardId,
    #[serde(default)]
    pub card_set: Option<String>,
    pub card_type: CardType,
    pub color: CardColor,
    pub level: u8,
    pub cost: u8,
    pub power: i32,
    pub soul: u8,
    pub triggers: Vec<TriggerIcon>,
    pub traits: Vec<u16>,
    pub abilities: Vec<AbilityTemplate>,
    #[serde(default)]
    pub counter_timing: bool,
    pub raw_text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardDb {
    pub cards: Vec<CardStatic>,
    #[serde(skip)]
    index: Vec<usize>,
}

impl CardDb {
    pub fn new(cards: Vec<CardStatic>) -> Result<Self> {
        let mut db = Self { cards, index: Vec::new() };
        db.build_index()?;
        Ok(db)
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = fs::read(&path).with_context(|| format!("Failed to read card db {:?}", path.as_ref()))?;
        Self::from_wsdb_bytes(&bytes)
    }

    pub fn from_wsdb_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            anyhow::bail!("Card db file too small");
        }
        if &bytes[0..4] != WSDB_MAGIC {
            anyhow::bail!("Card db magic mismatch; expected WSDB header");
        }
        let version = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| anyhow::anyhow!("Card db header missing version bytes"))?);
        if version != WSDB_SCHEMA_VERSION {
            anyhow::bail!("Unsupported card db schema version {version}, expected {WSDB_SCHEMA_VERSION}");
        }
        let payload = &bytes[8..];
        Self::from_postcard_payload(payload)
    }

    pub fn from_postcard_payload(payload: &[u8]) -> Result<Self> {
        let mut db: CardDb = postcard::from_bytes(payload).context("Failed to decode card db payload")?;
        db.build_index()?;
        Ok(db)
    }

    pub fn get(&self, id: CardId) -> Option<&CardStatic> {
        if id == 0 {
            return None;
        }
        let idx = *self.index.get(id as usize)?;
        if idx == usize::MAX {
            return None;
        }
        self.cards.get(idx)
    }

    pub fn schema_version() -> u32 {
        WSDB_SCHEMA_VERSION
    }

    pub fn to_bytes_with_header(&self) -> Result<Vec<u8>> {
        let payload = postcard::to_stdvec(self)?;
        let mut out = Vec::with_capacity(8 + payload.len());
        out.extend_from_slice(WSDB_MAGIC);
        out.extend_from_slice(&WSDB_SCHEMA_VERSION.to_le_bytes());
        out.extend_from_slice(&payload);
        Ok(out)
    }

    fn build_index(&mut self) -> Result<()> {
        let mut max_id: usize = 0;
        for card in &mut self.cards {
            if card.id == 0 {
                anyhow::bail!("CardId 0 is reserved for empty and cannot appear in the db");
            }
            if card.counter_timing && !matches!(card.card_type, CardType::Event | CardType::Character) {
                eprintln!("CardId {} has counter timing but card_type {:?} is not eligible; disabling counter timing", card.id, card.card_type);
                card.counter_timing = false;
            }
            max_id = max_id.max(card.id as usize);
        }
        let mut index = vec![usize::MAX; max_id + 1];
        for (i, card) in self.cards.iter().enumerate() {
            let id = card.id as usize;
            if index[id] != usize::MAX {
                anyhow::bail!("Duplicate CardId {id}");
            }
            index[id] = i;
        }
        self.index = index;
        Ok(())
    }
}
