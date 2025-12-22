use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TargetTemplate {
    OppFrontRow,
    SelfStage,
    SelfWaitingRoom,
    SelfHand,
    SelfDeckTop,
    SelfClock,
    SelfLevel,
    SelfStock,
    SelfMemory,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EffectTemplate {
    Draw { count: u8 },
    DealDamage { amount: u8, cancelable: bool },
    AddPower { amount: i32, duration_turn: bool },
    MoveToHand,
    ChangeController,
    CounterBackup { power: i32 },
    CounterDamageReduce { amount: u8 },
    CounterDamageCancel,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AbilityDef {
    pub kind: AbilityKind,
    pub timing: Option<AbilityTiming>,
    pub effects: Vec<EffectTemplate>,
    pub targets: Vec<TargetTemplate>,
}

impl AbilityDef {
    pub fn validate(&self) -> Result<()> {
        if self.effects.is_empty() {
            anyhow::bail!("AbilityDef must contain at least one effect");
        }
        if self.effects.len() > u8::MAX as usize {
            anyhow::bail!("AbilityDef has too many effects");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AbilityTiming {
    MainPhase,
    ClimaxPhase,
    AttackDeclaration,
    TriggerResolution,
    Counter,
    DamageResolution,
    Encore,
    EndPhase,
    OnPlay,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AbilityTemplate {
    Vanilla,
    ContinuousPower {
        amount: i32,
    },
    ContinuousCannotAttack,
    ContinuousAttackCost {
        cost: u8,
    },
    AutoOnPlayDraw {
        count: u8,
    },
    AutoOnAttackDealDamage {
        amount: u8,
        cancelable: bool,
    },
    AutoEndPhaseDraw {
        count: u8,
    },
    EventDealDamage {
        amount: u8,
        cancelable: bool,
    },
    ActivatedPlaceholder,
    ActivatedTargetedPower {
        amount: i32,
        count: u8,
        target: TargetTemplate,
    },
    ActivatedTargetedMoveToHand {
        count: u8,
        target: TargetTemplate,
    },
    ActivatedChangeController {
        count: u8,
        target: TargetTemplate,
    },
    CounterBackup {
        power: i32,
    },
    CounterDamageReduce {
        amount: u8,
    },
    CounterDamageCancel,
    AbilityDef(AbilityDef),
    Unsupported {
        id: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityKind {
    Continuous,
    Activated,
    Auto,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilitySpec {
    pub kind: AbilityKind,
    pub template: AbilityTemplate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbilityTemplateTag {
    Vanilla,
    ContinuousPower,
    ContinuousCannotAttack,
    ContinuousAttackCost,
    AutoOnPlayDraw,
    AutoOnAttackDealDamage,
    AutoEndPhaseDraw,
    EventDealDamage,
    ActivatedPlaceholder,
    ActivatedTargetedPower,
    ActivatedTargetedMoveToHand,
    ActivatedChangeController,
    CounterBackup,
    CounterDamageReduce,
    CounterDamageCancel,
    AbilityDef,
    Unsupported,
}

impl AbilityTemplate {
    pub fn tag(&self) -> AbilityTemplateTag {
        match self {
            AbilityTemplate::Vanilla => AbilityTemplateTag::Vanilla,
            AbilityTemplate::ContinuousPower { .. } => AbilityTemplateTag::ContinuousPower,
            AbilityTemplate::ContinuousCannotAttack => AbilityTemplateTag::ContinuousCannotAttack,
            AbilityTemplate::ContinuousAttackCost { .. } => {
                AbilityTemplateTag::ContinuousAttackCost
            }
            AbilityTemplate::AutoOnPlayDraw { .. } => AbilityTemplateTag::AutoOnPlayDraw,
            AbilityTemplate::AutoOnAttackDealDamage { .. } => {
                AbilityTemplateTag::AutoOnAttackDealDamage
            }
            AbilityTemplate::AutoEndPhaseDraw { .. } => AbilityTemplateTag::AutoEndPhaseDraw,
            AbilityTemplate::EventDealDamage { .. } => AbilityTemplateTag::EventDealDamage,
            AbilityTemplate::ActivatedPlaceholder => AbilityTemplateTag::ActivatedPlaceholder,
            AbilityTemplate::ActivatedTargetedPower { .. } => {
                AbilityTemplateTag::ActivatedTargetedPower
            }
            AbilityTemplate::ActivatedTargetedMoveToHand { .. } => {
                AbilityTemplateTag::ActivatedTargetedMoveToHand
            }
            AbilityTemplate::ActivatedChangeController { .. } => {
                AbilityTemplateTag::ActivatedChangeController
            }
            AbilityTemplate::CounterBackup { .. } => AbilityTemplateTag::CounterBackup,
            AbilityTemplate::CounterDamageReduce { .. } => AbilityTemplateTag::CounterDamageReduce,
            AbilityTemplate::CounterDamageCancel => AbilityTemplateTag::CounterDamageCancel,
            AbilityTemplate::AbilityDef(_) => AbilityTemplateTag::AbilityDef,
            AbilityTemplate::Unsupported { .. } => AbilityTemplateTag::Unsupported,
        }
    }
}

impl AbilitySpec {
    pub fn from_template(template: &AbilityTemplate) -> Self {
        let kind = match template {
            AbilityTemplate::ContinuousPower { .. }
            | AbilityTemplate::ContinuousCannotAttack
            | AbilityTemplate::ContinuousAttackCost { .. } => AbilityKind::Continuous,
            AbilityTemplate::ActivatedPlaceholder
            | AbilityTemplate::ActivatedTargetedPower { .. }
            | AbilityTemplate::ActivatedTargetedMoveToHand { .. }
            | AbilityTemplate::ActivatedChangeController { .. } => AbilityKind::Activated,
            AbilityTemplate::AbilityDef(def) => def.kind,
            _ => AbilityKind::Auto,
        };
        Self {
            kind,
            template: template.clone(),
        }
    }
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
    pub ability_defs: Vec<AbilityDef>,
    #[serde(default)]
    pub counter_timing: bool,
    pub raw_text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardDb {
    pub cards: Vec<CardStatic>,
    #[serde(skip)]
    index: Vec<usize>,
    #[serde(skip)]
    ability_specs: Vec<Vec<AbilitySpec>>,
    #[serde(skip)]
    compiled_ability_effects: Vec<Vec<Vec<crate::effects::EffectSpec>>>,
}

impl CardDb {
    pub fn new(cards: Vec<CardStatic>) -> Result<Self> {
        let mut db = Self {
            cards,
            index: Vec::new(),
            ability_specs: Vec::new(),
            compiled_ability_effects: Vec::new(),
        };
        db.build_index()?;
        Ok(db)
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = fs::read(&path)
            .with_context(|| format!("Failed to read card db {:?}", path.as_ref()))?;
        Self::from_wsdb_bytes(&bytes)
    }

    pub fn from_wsdb_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            anyhow::bail!("Card db file too small");
        }
        if &bytes[0..4] != WSDB_MAGIC {
            anyhow::bail!("Card db magic mismatch; expected WSDB header");
        }
        let version = u32::from_le_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Card db header missing version bytes"))?,
        );
        if version != WSDB_SCHEMA_VERSION {
            anyhow::bail!(
                "Unsupported card db schema version {version}, expected {WSDB_SCHEMA_VERSION}"
            );
        }
        let payload = &bytes[8..];
        Self::from_postcard_payload(payload)
    }

    pub fn from_postcard_payload(payload: &[u8]) -> Result<Self> {
        let mut db: CardDb =
            postcard::from_bytes(payload).context("Failed to decode card db payload")?;
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
            if card.counter_timing
                && !matches!(card.card_type, CardType::Event | CardType::Character)
            {
                eprintln!("CardId {} has counter timing but card_type {:?} is not eligible; disabling counter timing", card.id, card.card_type);
                card.counter_timing = false;
            }
            for def in &card.ability_defs {
                def.validate()
                    .with_context(|| format!("CardId {} AbilityDef invalid", card.id))?;
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
        self.build_ability_specs();
        self.build_compiled_abilities()?;
        Ok(())
    }

    fn build_ability_specs(&mut self) {
        self.ability_specs = self
            .cards
            .iter()
            .map(|card| {
                let mut specs: Vec<AbilitySpec> = card
                    .abilities
                    .iter()
                    .map(AbilitySpec::from_template)
                    .collect();
                for def in &card.ability_defs {
                    specs.push(AbilitySpec::from_template(&AbilityTemplate::AbilityDef(
                        def.clone(),
                    )));
                }
                specs
            })
            .collect();
    }

    fn build_compiled_abilities(&mut self) -> Result<()> {
        let mut compiled: Vec<Vec<Vec<crate::effects::EffectSpec>>> =
            Vec::with_capacity(self.cards.len());
        for card in &self.cards {
            let specs = self.iter_card_abilities_in_canonical_order(card.id);
            let mut per_ability: Vec<Vec<crate::effects::EffectSpec>> =
                Vec::with_capacity(specs.len());
            for (ability_index, spec) in specs.iter().enumerate() {
                let idx = ability_index as u8;
                let effects = match &spec.template {
                    AbilityTemplate::AbilityDef(def) => compile_effects_from_def(card.id, idx, def),
                    AbilityTemplate::Vanilla | AbilityTemplate::Unsupported { .. } => Vec::new(),
                    _ => compile_effects_from_template(card.id, idx, &spec.template),
                };
                per_ability.push(effects);
            }
            compiled.push(per_ability);
        }
        self.compiled_ability_effects = compiled;
        Ok(())
    }

    pub fn iter_card_abilities_in_canonical_order(&self, card_id: CardId) -> &[AbilitySpec] {
        let idx = match self.index.get(card_id as usize) {
            Some(idx) => *idx,
            None => return &[],
        };
        if idx == usize::MAX {
            return &[];
        }
        self.ability_specs
            .get(idx)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn compiled_effects_for_ability(
        &self,
        card_id: CardId,
        ability_index: usize,
    ) -> &[crate::effects::EffectSpec] {
        let idx = match self.index.get(card_id as usize) {
            Some(idx) => *idx,
            None => return &[],
        };
        if idx == usize::MAX {
            return &[];
        }
        self.compiled_ability_effects
            .get(idx)
            .and_then(|per_ability| per_ability.get(ability_index))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn compiled_effects_flat(&self, card_id: CardId) -> Vec<crate::effects::EffectSpec> {
        let idx = match self.index.get(card_id as usize) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };
        if idx == usize::MAX {
            return Vec::new();
        }
        let Some(per_ability) = self.compiled_ability_effects.get(idx) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for effects in per_ability {
            out.extend(effects.iter().cloned());
        }
        out
    }
}

fn target_spec_from_template(template: TargetTemplate, count: u8) -> crate::state::TargetSpec {
    let zone = match template {
        TargetTemplate::OppFrontRow | TargetTemplate::SelfStage => crate::state::TargetZone::Stage,
        TargetTemplate::SelfWaitingRoom => crate::state::TargetZone::WaitingRoom,
        TargetTemplate::SelfHand => crate::state::TargetZone::Hand,
        TargetTemplate::SelfDeckTop => crate::state::TargetZone::DeckTop,
        TargetTemplate::SelfClock => crate::state::TargetZone::Clock,
        TargetTemplate::SelfLevel => crate::state::TargetZone::Level,
        TargetTemplate::SelfStock => crate::state::TargetZone::Stock,
        TargetTemplate::SelfMemory => crate::state::TargetZone::Memory,
    };
    let card_type = match zone {
        crate::state::TargetZone::Stage => Some(CardType::Character),
        _ => None,
    };
    crate::state::TargetSpec {
        zone,
        side: match template {
            TargetTemplate::OppFrontRow => crate::state::TargetSide::Opponent,
            _ => crate::state::TargetSide::SelfSide,
        },
        slot_filter: match template {
            TargetTemplate::OppFrontRow => crate::state::TargetSlotFilter::FrontRow,
            _ => crate::state::TargetSlotFilter::Any,
        },
        card_type,
        count,
    }
}

fn compile_effects_from_template(
    card_id: CardId,
    ability_index: u8,
    template: &AbilityTemplate,
) -> Vec<crate::effects::EffectSpec> {
    let mut out = Vec::new();
    match template {
        AbilityTemplate::ContinuousPower { amount } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Continuous,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: *amount,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                },
                target: Some(target_spec_from_template(TargetTemplate::SelfStage, 1)),
            });
        }
        AbilityTemplate::ContinuousCannotAttack => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Continuous,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::CannotAttack,
                    magnitude: 1,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                },
                target: Some(target_spec_from_template(TargetTemplate::SelfStage, 1)),
            });
        }
        AbilityTemplate::ContinuousAttackCost { cost } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Continuous,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::AttackCost,
                    magnitude: *cost as i32,
                    duration: crate::state::ModifierDuration::WhileOnStage,
                },
                target: Some(target_spec_from_template(TargetTemplate::SelfStage, 1)),
            });
        }
        AbilityTemplate::AutoOnPlayDraw { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Draw { count: *count },
                target: None,
            });
        }
        AbilityTemplate::AutoOnAttackDealDamage { amount, cancelable } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Damage {
                    amount: *amount as i32,
                    cancelable: *cancelable,
                    damage_type: crate::state::DamageType::Effect,
                },
                target: None,
            });
        }
        AbilityTemplate::AutoEndPhaseDraw { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Draw { count: *count },
                target: None,
            });
        }
        AbilityTemplate::EventDealDamage { amount, cancelable } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::EventPlay,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Damage {
                    amount: *amount as i32,
                    cancelable: *cancelable,
                    damage_type: crate::state::DamageType::Effect,
                },
                target: None,
            });
        }
        AbilityTemplate::ActivatedPlaceholder => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: 1000,
                    duration: crate::state::ModifierDuration::UntilEndOfTurn,
                },
                target: Some(target_spec_from_template(TargetTemplate::SelfStage, 1)),
            });
        }
        AbilityTemplate::ActivatedTargetedPower {
            amount,
            count,
            target,
        } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: *amount,
                    duration: crate::state::ModifierDuration::UntilEndOfTurn,
                },
                target: Some(target_spec_from_template(*target, *count)),
            });
        }
        AbilityTemplate::ActivatedTargetedMoveToHand { count, target } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(target_spec_from_template(*target, *count)),
            });
        }
        AbilityTemplate::ActivatedChangeController { count, target } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::ChangeController {
                    new_controller: crate::state::TargetSide::SelfSide,
                },
                target: Some(target_spec_from_template(*target, *count)),
            });
        }
        AbilityTemplate::CounterBackup { power } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Counter,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::CounterBackup { power: *power },
                target: None,
            });
        }
        AbilityTemplate::CounterDamageReduce { amount } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Counter,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::CounterDamageReduce { amount: *amount },
                target: None,
            });
        }
        AbilityTemplate::CounterDamageCancel => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Counter,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::CounterDamageCancel,
                target: None,
            });
        }
        AbilityTemplate::AbilityDef(_)
        | AbilityTemplate::Vanilla
        | AbilityTemplate::Unsupported { .. } => {}
    }
    out
}

fn compile_effects_from_def(
    card_id: CardId,
    ability_index: u8,
    ability: &AbilityDef,
) -> Vec<crate::effects::EffectSpec> {
    let mut effects = Vec::with_capacity(ability.effects.len());
    for (effect_index, effect) in ability.effects.iter().enumerate() {
        let effect_id = crate::effects::EffectId::new(
            match ability.kind {
                AbilityKind::Continuous => crate::effects::EffectSourceKind::Continuous,
                AbilityKind::Activated => crate::effects::EffectSourceKind::Activated,
                AbilityKind::Auto => crate::effects::EffectSourceKind::Auto,
            },
            card_id,
            ability_index,
            effect_index as u8,
        );
        let (kind, target) = match effect {
            EffectTemplate::Draw { count } => {
                (crate::effects::EffectKind::Draw { count: *count }, None)
            }
            EffectTemplate::DealDamage { amount, cancelable } => (
                crate::effects::EffectKind::Damage {
                    amount: *amount as i32,
                    cancelable: *cancelable,
                    damage_type: crate::state::DamageType::Effect,
                },
                None,
            ),
            EffectTemplate::AddPower {
                amount,
                duration_turn,
            } => (
                crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: *amount,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                ability
                    .targets
                    .first()
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::MoveToHand => (
                crate::effects::EffectKind::MoveToHand,
                ability
                    .targets
                    .first()
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::ChangeController => (
                crate::effects::EffectKind::ChangeController {
                    new_controller: crate::state::TargetSide::SelfSide,
                },
                ability
                    .targets
                    .first()
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::CounterBackup { power } => (
                crate::effects::EffectKind::CounterBackup { power: *power },
                None,
            ),
            EffectTemplate::CounterDamageReduce { amount } => (
                crate::effects::EffectKind::CounterDamageReduce { amount: *amount },
                None,
            ),
            EffectTemplate::CounterDamageCancel => {
                (crate::effects::EffectKind::CounterDamageCancel, None)
            }
        };
        effects.push(crate::effects::EffectSpec {
            id: effect_id,
            kind,
            target,
        });
    }
    effects
}
