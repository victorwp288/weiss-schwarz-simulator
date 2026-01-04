use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::events::RevealAudience;
use crate::state::{TargetSide, TargetZone};

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
    OppBackRow,
    OppStage,
    OppStageSlot { slot: u8 },
    SelfFrontRow,
    SelfBackRow,
    SelfStage,
    SelfStageSlot { slot: u8 },
    This,
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
    Draw {
        count: u8,
    },
    DealDamage {
        amount: u8,
        cancelable: bool,
    },
    AddPower {
        amount: i32,
        duration_turn: bool,
    },
    MoveToHand,
    MoveToWaitingRoom,
    MoveToStock,
    MoveToClock,
    Heal,
    RestTarget,
    StandTarget,
    StockCharge {
        count: u8,
    },
    MillTop {
        target: TargetSide,
        count: u8,
    },
    MoveStageSlot {
        slot: u8,
    },
    SwapStageSlots,
    RandomDiscardFromHand {
        target: TargetSide,
        count: u8,
    },
    RandomMill {
        target: TargetSide,
        count: u8,
    },
    RevealZoneTop {
        target: TargetSide,
        zone: TargetZone,
        count: u8,
        audience: RevealAudience,
    },
    ChangeController,
    CounterBackup {
        power: i32,
    },
    CounterDamageReduce {
        amount: u8,
    },
    CounterDamageCancel,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilityCost {
    #[serde(default)]
    pub stock: u8,
    #[serde(default)]
    pub rest_self: bool,
    #[serde(default)]
    pub rest_other: u8,
    #[serde(default)]
    pub discard_from_hand: u8,
    #[serde(default)]
    pub clock_from_hand: u8,
    #[serde(default)]
    pub clock_from_deck_top: u8,
    #[serde(default)]
    pub reveal_from_hand: u8,
}

impl AbilityCost {
    pub fn is_empty(&self) -> bool {
        self.stock == 0
            && !self.rest_self
            && self.rest_other == 0
            && self.discard_from_hand == 0
            && self.clock_from_hand == 0
            && self.clock_from_deck_top == 0
            && self.reveal_from_hand == 0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AbilityDef {
    pub kind: AbilityKind,
    pub timing: Option<AbilityTiming>,
    pub effects: Vec<EffectTemplate>,
    pub targets: Vec<TargetTemplate>,
    #[serde(default)]
    pub cost: AbilityCost,
    #[serde(default)]
    pub target_card_type: Option<CardType>,
    #[serde(default)]
    pub target_trait: Option<u16>,
    #[serde(default)]
    pub target_level_max: Option<u8>,
    #[serde(default)]
    pub target_cost_max: Option<u8>,
    #[serde(default)]
    pub target_limit: Option<u8>,
}

impl AbilityDef {
    pub fn validate(&self) -> Result<()> {
        if self.effects.is_empty() {
            anyhow::bail!("AbilityDef must contain at least one effect");
        }
        if self.effects.len() > u8::MAX as usize {
            anyhow::bail!("AbilityDef has too many effects");
        }
        if self.kind != AbilityKind::Activated && !self.cost.is_empty() {
            anyhow::bail!("AbilityDef cost is only valid for activated abilities");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AbilityTiming {
    BeginTurn,
    BeginStandPhase,
    AfterStandPhase,
    BeginDrawPhase,
    AfterDrawPhase,
    BeginClockPhase,
    AfterClockPhase,
    BeginMainPhase,
    BeginClimaxPhase,
    AfterClimaxPhase,
    BeginAttackPhase,
    BeginAttackDeclarationStep,
    BeginEncoreStep,
    EndPhase,
    EndPhaseCleanup,
    EndOfAttack,
    AttackDeclaration,
    TriggerResolution,
    Counter,
    DamageResolution,
    Encore,
    OnPlay,
    OnReverse,
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
    AutoOnPlaySalvage {
        count: u8,
        optional: bool,
        card_type: Option<CardType>,
    },
    AutoOnPlaySearchDeckTop {
        count: u8,
        optional: bool,
        card_type: Option<CardType>,
    },
    AutoOnPlayRevealDeckTop {
        count: u8,
    },
    AutoOnPlayStockCharge {
        count: u8,
    },
    AutoOnPlayMillTop {
        count: u8,
    },
    AutoOnPlayHeal {
        count: u8,
    },
    AutoOnAttackDealDamage {
        amount: u8,
        cancelable: bool,
    },
    AutoEndPhaseDraw {
        count: u8,
    },
    AutoOnReverseDraw {
        count: u8,
    },
    AutoOnReverseSalvage {
        count: u8,
        optional: bool,
        card_type: Option<CardType>,
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
    ActivatedPaidTargetedPower {
        cost: u8,
        amount: i32,
        count: u8,
        target: TargetTemplate,
    },
    ActivatedTargetedMoveToHand {
        count: u8,
        target: TargetTemplate,
    },
    ActivatedPaidTargetedMoveToHand {
        cost: u8,
        count: u8,
        target: TargetTemplate,
    },
    ActivatedChangeController {
        count: u8,
        target: TargetTemplate,
    },
    ActivatedPaidChangeController {
        cost: u8,
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
    AutoOnPlaySalvage,
    AutoOnPlaySearchDeckTop,
    AutoOnPlayRevealDeckTop,
    AutoOnPlayStockCharge,
    AutoOnPlayMillTop,
    AutoOnPlayHeal,
    AutoOnAttackDealDamage,
    AutoEndPhaseDraw,
    AutoOnReverseDraw,
    AutoOnReverseSalvage,
    EventDealDamage,
    ActivatedPlaceholder,
    ActivatedTargetedPower,
    ActivatedPaidTargetedPower,
    ActivatedTargetedMoveToHand,
    ActivatedPaidTargetedMoveToHand,
    ActivatedChangeController,
    ActivatedPaidChangeController,
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
            AbilityTemplate::AutoOnPlaySalvage { .. } => AbilityTemplateTag::AutoOnPlaySalvage,
            AbilityTemplate::AutoOnPlaySearchDeckTop { .. } => {
                AbilityTemplateTag::AutoOnPlaySearchDeckTop
            }
            AbilityTemplate::AutoOnPlayRevealDeckTop { .. } => {
                AbilityTemplateTag::AutoOnPlayRevealDeckTop
            }
            AbilityTemplate::AutoOnPlayStockCharge { .. } => {
                AbilityTemplateTag::AutoOnPlayStockCharge
            }
            AbilityTemplate::AutoOnPlayMillTop { .. } => AbilityTemplateTag::AutoOnPlayMillTop,
            AbilityTemplate::AutoOnPlayHeal { .. } => AbilityTemplateTag::AutoOnPlayHeal,
            AbilityTemplate::AutoOnAttackDealDamage { .. } => {
                AbilityTemplateTag::AutoOnAttackDealDamage
            }
            AbilityTemplate::AutoEndPhaseDraw { .. } => AbilityTemplateTag::AutoEndPhaseDraw,
            AbilityTemplate::AutoOnReverseDraw { .. } => AbilityTemplateTag::AutoOnReverseDraw,
            AbilityTemplate::AutoOnReverseSalvage { .. } => {
                AbilityTemplateTag::AutoOnReverseSalvage
            }
            AbilityTemplate::EventDealDamage { .. } => AbilityTemplateTag::EventDealDamage,
            AbilityTemplate::ActivatedPlaceholder => AbilityTemplateTag::ActivatedPlaceholder,
            AbilityTemplate::ActivatedTargetedPower { .. } => {
                AbilityTemplateTag::ActivatedTargetedPower
            }
            AbilityTemplate::ActivatedPaidTargetedPower { .. } => {
                AbilityTemplateTag::ActivatedPaidTargetedPower
            }
            AbilityTemplate::ActivatedTargetedMoveToHand { .. } => {
                AbilityTemplateTag::ActivatedTargetedMoveToHand
            }
            AbilityTemplate::ActivatedPaidTargetedMoveToHand { .. } => {
                AbilityTemplateTag::ActivatedPaidTargetedMoveToHand
            }
            AbilityTemplate::ActivatedChangeController { .. } => {
                AbilityTemplateTag::ActivatedChangeController
            }
            AbilityTemplate::ActivatedPaidChangeController { .. } => {
                AbilityTemplateTag::ActivatedPaidChangeController
            }
            AbilityTemplate::CounterBackup { .. } => AbilityTemplateTag::CounterBackup,
            AbilityTemplate::CounterDamageReduce { .. } => AbilityTemplateTag::CounterDamageReduce,
            AbilityTemplate::CounterDamageCancel => AbilityTemplateTag::CounterDamageCancel,
            AbilityTemplate::AbilityDef(_) => AbilityTemplateTag::AbilityDef,
            AbilityTemplate::Unsupported { .. } => AbilityTemplateTag::Unsupported,
        }
    }

    pub fn activation_cost(&self) -> Option<u8> {
        match self {
            AbilityTemplate::ActivatedPaidTargetedPower { cost, .. }
            | AbilityTemplate::ActivatedPaidTargetedMoveToHand { cost, .. }
            | AbilityTemplate::ActivatedPaidChangeController { cost, .. } => Some(*cost),
            _ => None,
        }
    }

    pub fn activation_cost_spec(&self) -> AbilityCost {
        match self {
            AbilityTemplate::ActivatedPaidTargetedPower { cost, .. }
            | AbilityTemplate::ActivatedPaidTargetedMoveToHand { cost, .. }
            | AbilityTemplate::ActivatedPaidChangeController { cost, .. } => AbilityCost {
                stock: *cost,
                ..AbilityCost::default()
            },
            AbilityTemplate::AbilityDef(def) => def.cost,
            _ => AbilityCost::default(),
        }
    }
}

fn ability_kind_key(kind: AbilityKind) -> u64 {
    match kind {
        AbilityKind::Continuous => 0,
        AbilityKind::Activated => 1,
        AbilityKind::Auto => 2,
    }
}

fn ability_timing_key(timing: Option<AbilityTiming>) -> u64 {
    match timing {
        None => u64::MAX,
        Some(AbilityTiming::BeginTurn) => 0,
        Some(AbilityTiming::BeginStandPhase) => 1,
        Some(AbilityTiming::AfterStandPhase) => 2,
        Some(AbilityTiming::BeginDrawPhase) => 3,
        Some(AbilityTiming::AfterDrawPhase) => 4,
        Some(AbilityTiming::BeginClockPhase) => 5,
        Some(AbilityTiming::AfterClockPhase) => 6,
        Some(AbilityTiming::BeginMainPhase) => 7,
        Some(AbilityTiming::BeginClimaxPhase) => 8,
        Some(AbilityTiming::AfterClimaxPhase) => 9,
        Some(AbilityTiming::BeginAttackPhase) => 10,
        Some(AbilityTiming::BeginAttackDeclarationStep) => 11,
        Some(AbilityTiming::BeginEncoreStep) => 12,
        Some(AbilityTiming::EndPhase) => 13,
        Some(AbilityTiming::EndPhaseCleanup) => 14,
        Some(AbilityTiming::EndOfAttack) => 15,
        Some(AbilityTiming::AttackDeclaration) => 16,
        Some(AbilityTiming::TriggerResolution) => 17,
        Some(AbilityTiming::Counter) => 18,
        Some(AbilityTiming::DamageResolution) => 19,
        Some(AbilityTiming::Encore) => 20,
        Some(AbilityTiming::OnPlay) => 21,
        Some(AbilityTiming::OnReverse) => 22,
    }
}

fn target_template_key(target: TargetTemplate) -> u64 {
    match target {
        TargetTemplate::OppFrontRow => 0,
        TargetTemplate::OppBackRow => 1,
        TargetTemplate::OppStage => 2,
        TargetTemplate::OppStageSlot { slot } => 3_000 + slot as u64,
        TargetTemplate::SelfFrontRow => 4,
        TargetTemplate::SelfBackRow => 5,
        TargetTemplate::SelfStage => 6,
        TargetTemplate::SelfStageSlot { slot } => 7_000 + slot as u64,
        TargetTemplate::This => 15,
        TargetTemplate::SelfWaitingRoom => 8,
        TargetTemplate::SelfHand => 9,
        TargetTemplate::SelfDeckTop => 10,
        TargetTemplate::SelfClock => 11,
        TargetTemplate::SelfLevel => 12,
        TargetTemplate::SelfStock => 13,
        TargetTemplate::SelfMemory => 14,
    }
}

fn card_type_key(card_type: Option<CardType>) -> u64 {
    match card_type {
        None => 0,
        Some(CardType::Character) => 1,
        Some(CardType::Event) => 2,
        Some(CardType::Climax) => 3,
    }
}

fn target_side_key(side: TargetSide) -> u64 {
    match side {
        TargetSide::SelfSide => 0,
        TargetSide::Opponent => 1,
    }
}

fn target_zone_key(zone: TargetZone) -> u64 {
    match zone {
        TargetZone::Stage => 0,
        TargetZone::Hand => 1,
        TargetZone::DeckTop => 2,
        TargetZone::Clock => 3,
        TargetZone::Level => 4,
        TargetZone::Stock => 5,
        TargetZone::Memory => 6,
        TargetZone::WaitingRoom => 7,
        TargetZone::Climax => 8,
        TargetZone::Resolution => 9,
    }
}

fn reveal_audience_key(audience: RevealAudience) -> u64 {
    match audience {
        RevealAudience::Public => 0,
        RevealAudience::BothPlayers => 1,
        RevealAudience::OwnerOnly => 2,
        RevealAudience::ControllerOnly => 3,
        RevealAudience::ReplayOnly => 4,
    }
}

fn effect_template_key(effect: &EffectTemplate, out: &mut Vec<u64>) {
    match effect {
        EffectTemplate::Draw { count } => {
            out.push(0);
            out.push(*count as u64);
        }
        EffectTemplate::DealDamage { amount, cancelable } => {
            out.push(1);
            out.push(*amount as u64);
            out.push(u64::from(*cancelable));
        }
        EffectTemplate::AddPower {
            amount,
            duration_turn,
        } => {
            out.push(2);
            out.push(*amount as i64 as u64);
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::MoveToHand => {
            out.push(3);
        }
        EffectTemplate::MoveToWaitingRoom => {
            out.push(8);
        }
        EffectTemplate::MoveToStock => {
            out.push(9);
        }
        EffectTemplate::MoveToClock => {
            out.push(10);
        }
        EffectTemplate::Heal => {
            out.push(17);
        }
        EffectTemplate::RestTarget => {
            out.push(11);
        }
        EffectTemplate::StandTarget => {
            out.push(12);
        }
        EffectTemplate::StockCharge { count } => {
            out.push(13);
            out.push(*count as u64);
        }
        EffectTemplate::MillTop { target, count } => {
            out.push(18);
            out.push(target_side_key(*target));
            out.push(*count as u64);
        }
        EffectTemplate::MoveStageSlot { slot } => {
            out.push(19);
            out.push(*slot as u64);
        }
        EffectTemplate::SwapStageSlots => {
            out.push(20);
        }
        EffectTemplate::RandomDiscardFromHand { target, count } => {
            out.push(14);
            out.push(target_side_key(*target));
            out.push(*count as u64);
        }
        EffectTemplate::RandomMill { target, count } => {
            out.push(15);
            out.push(target_side_key(*target));
            out.push(*count as u64);
        }
        EffectTemplate::RevealZoneTop {
            target,
            zone,
            count,
            audience,
        } => {
            out.push(16);
            out.push(target_side_key(*target));
            out.push(target_zone_key(*zone));
            out.push(*count as u64);
            out.push(reveal_audience_key(*audience));
        }
        EffectTemplate::ChangeController => {
            out.push(4);
        }
        EffectTemplate::CounterBackup { power } => {
            out.push(5);
            out.push(*power as i64 as u64);
        }
        EffectTemplate::CounterDamageReduce { amount } => {
            out.push(6);
            out.push(*amount as u64);
        }
        EffectTemplate::CounterDamageCancel => {
            out.push(7);
        }
    }
}

fn ability_def_key(def: &AbilityDef) -> Vec<u64> {
    let mut key = Vec::with_capacity(16 + def.effects.len() * 5 + def.targets.len());
    key.push(ability_kind_key(def.kind));
    key.push(ability_timing_key(def.timing));
    key.push(def.cost.stock as u64);
    key.push(def.cost.rest_self as u64);
    key.push(def.cost.rest_other as u64);
    key.push(def.cost.discard_from_hand as u64);
    key.push(def.cost.clock_from_hand as u64);
    key.push(def.cost.clock_from_deck_top as u64);
    key.push(def.cost.reveal_from_hand as u64);
    key.push(def.effects.len() as u64);
    for effect in &def.effects {
        effect_template_key(effect, &mut key);
    }
    key.push(def.targets.len() as u64);
    for target in &def.targets {
        key.push(target_template_key(*target));
    }
    key.push(card_type_key(def.target_card_type));
    key.push(def.target_trait.map(|v| v as u64 + 1).unwrap_or(0));
    key.push(def.target_level_max.map(|v| v as u64 + 1).unwrap_or(0));
    key.push(def.target_cost_max.map(|v| v as u64 + 1).unwrap_or(0));
    key.push(def.target_limit.map(|v| v as u64 + 1).unwrap_or(0));
    key
}

fn ability_template_key(template: &AbilityTemplate) -> Vec<u64> {
    match template {
        AbilityTemplate::Vanilla => Vec::new(),
        AbilityTemplate::ContinuousPower { amount } => vec![*amount as i64 as u64],
        AbilityTemplate::ContinuousCannotAttack => Vec::new(),
        AbilityTemplate::ContinuousAttackCost { cost } => vec![*cost as u64],
        AbilityTemplate::AutoOnPlayDraw { count } => vec![*count as u64],
        AbilityTemplate::AutoOnPlaySalvage {
            count,
            optional,
            card_type,
        } => vec![
            *count as u64,
            u64::from(*optional),
            card_type_key(*card_type),
        ],
        AbilityTemplate::AutoOnPlaySearchDeckTop {
            count,
            optional,
            card_type,
        } => vec![
            *count as u64,
            u64::from(*optional),
            card_type_key(*card_type),
        ],
        AbilityTemplate::AutoOnPlayRevealDeckTop { count } => vec![*count as u64],
        AbilityTemplate::AutoOnPlayStockCharge { count } => vec![*count as u64],
        AbilityTemplate::AutoOnPlayMillTop { count } => vec![*count as u64],
        AbilityTemplate::AutoOnPlayHeal { count } => vec![*count as u64],
        AbilityTemplate::AutoOnAttackDealDamage { amount, cancelable } => {
            vec![*amount as u64, u64::from(*cancelable)]
        }
        AbilityTemplate::AutoEndPhaseDraw { count } => vec![*count as u64],
        AbilityTemplate::AutoOnReverseDraw { count } => vec![*count as u64],
        AbilityTemplate::AutoOnReverseSalvage {
            count,
            optional,
            card_type,
        } => vec![
            *count as u64,
            u64::from(*optional),
            card_type_key(*card_type),
        ],
        AbilityTemplate::EventDealDamage { amount, cancelable } => {
            vec![*amount as u64, u64::from(*cancelable)]
        }
        AbilityTemplate::ActivatedPlaceholder => Vec::new(),
        AbilityTemplate::ActivatedTargetedPower {
            amount,
            count,
            target,
        } => vec![
            *amount as i64 as u64,
            *count as u64,
            target_template_key(*target),
        ],
        AbilityTemplate::ActivatedPaidTargetedPower {
            cost,
            amount,
            count,
            target,
        } => vec![
            *cost as u64,
            *amount as i64 as u64,
            *count as u64,
            target_template_key(*target),
        ],
        AbilityTemplate::ActivatedTargetedMoveToHand { count, target } => {
            vec![*count as u64, target_template_key(*target)]
        }
        AbilityTemplate::ActivatedPaidTargetedMoveToHand {
            cost,
            count,
            target,
        } => vec![*cost as u64, *count as u64, target_template_key(*target)],
        AbilityTemplate::ActivatedChangeController { count, target } => {
            vec![*count as u64, target_template_key(*target)]
        }
        AbilityTemplate::ActivatedPaidChangeController {
            cost,
            count,
            target,
        } => vec![*cost as u64, *count as u64, target_template_key(*target)],
        AbilityTemplate::CounterBackup { power } => vec![*power as i64 as u64],
        AbilityTemplate::CounterDamageReduce { amount } => vec![*amount as u64],
        AbilityTemplate::CounterDamageCancel => Vec::new(),
        AbilityTemplate::AbilityDef(def) => ability_def_key(def),
        AbilityTemplate::Unsupported { id } => vec![*id as u64],
    }
}

fn ability_sort_key(spec: &AbilitySpec) -> (u8, Vec<u64>) {
    let tag = spec.template.tag() as u8;
    (tag, ability_template_key(&spec.template))
}

impl AbilitySpec {
    pub fn from_template(template: &AbilityTemplate) -> Self {
        let kind = match template {
            AbilityTemplate::ContinuousPower { .. }
            | AbilityTemplate::ContinuousCannotAttack
            | AbilityTemplate::ContinuousAttackCost { .. } => AbilityKind::Continuous,
            AbilityTemplate::ActivatedPlaceholder
            | AbilityTemplate::ActivatedTargetedPower { .. }
            | AbilityTemplate::ActivatedPaidTargetedPower { .. }
            | AbilityTemplate::ActivatedTargetedMoveToHand { .. }
            | AbilityTemplate::ActivatedPaidTargetedMoveToHand { .. }
            | AbilityTemplate::ActivatedChangeController { .. }
            | AbilityTemplate::ActivatedPaidChangeController { .. } => AbilityKind::Activated,
            AbilityTemplate::AbilityDef(def) => def.kind,
            _ => AbilityKind::Auto,
        };
        Self {
            kind,
            template: template.clone(),
        }
    }

    pub fn timing(&self) -> Option<AbilityTiming> {
        match &self.template {
            AbilityTemplate::AutoOnPlayDraw { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AutoOnPlaySalvage { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AutoOnPlaySearchDeckTop { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AutoOnPlayRevealDeckTop { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AutoOnPlayStockCharge { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AutoOnPlayMillTop { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AutoOnPlayHeal { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AutoOnAttackDealDamage { .. } => {
                Some(AbilityTiming::AttackDeclaration)
            }
            AbilityTemplate::AutoEndPhaseDraw { .. } => Some(AbilityTiming::EndPhase),
            AbilityTemplate::AutoOnReverseDraw { .. } => Some(AbilityTiming::OnReverse),
            AbilityTemplate::AutoOnReverseSalvage { .. } => Some(AbilityTiming::OnReverse),
            AbilityTemplate::CounterBackup { .. }
            | AbilityTemplate::CounterDamageReduce { .. }
            | AbilityTemplate::CounterDamageCancel => Some(AbilityTiming::Counter),
            AbilityTemplate::EventDealDamage { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AbilityDef(def) => def.timing,
            _ => None,
        }
    }

    pub fn is_event_play(&self) -> bool {
        matches!(self.template, AbilityTemplate::EventDealDamage { .. })
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
        self.build_ability_specs()?;
        self.build_compiled_abilities()?;
        Ok(())
    }

    fn build_ability_specs(&mut self) -> Result<()> {
        let mut specs_list: Vec<Vec<AbilitySpec>> = Vec::with_capacity(self.cards.len());
        for card in &self.cards {
            for template in &card.abilities {
                if matches!(
                    template,
                    AbilityTemplate::ActivatedPlaceholder | AbilityTemplate::Unsupported { .. }
                ) {
                    anyhow::bail!(
                        "CardId {} uses unsupported ability template; update card db",
                        card.id
                    );
                }
            }
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
            specs.sort_by_cached_key(ability_sort_key);
            specs_list.push(specs);
        }
        self.ability_specs = specs_list;
        Ok(())
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
        TargetTemplate::OppFrontRow
        | TargetTemplate::OppBackRow
        | TargetTemplate::OppStage
        | TargetTemplate::OppStageSlot { .. }
        | TargetTemplate::SelfFrontRow
        | TargetTemplate::SelfBackRow
        | TargetTemplate::SelfStage
        | TargetTemplate::SelfStageSlot { .. }
        | TargetTemplate::This => crate::state::TargetZone::Stage,
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
            TargetTemplate::OppFrontRow
            | TargetTemplate::OppBackRow
            | TargetTemplate::OppStage
            | TargetTemplate::OppStageSlot { .. } => crate::state::TargetSide::Opponent,
            _ => crate::state::TargetSide::SelfSide,
        },
        slot_filter: match template {
            TargetTemplate::OppFrontRow | TargetTemplate::SelfFrontRow => {
                crate::state::TargetSlotFilter::FrontRow
            }
            TargetTemplate::OppBackRow | TargetTemplate::SelfBackRow => {
                crate::state::TargetSlotFilter::BackRow
            }
            TargetTemplate::OppStageSlot { slot } | TargetTemplate::SelfStageSlot { slot } => {
                crate::state::TargetSlotFilter::SpecificSlot(slot)
            }
            _ => crate::state::TargetSlotFilter::Any,
        },
        card_type,
        card_trait: None,
        level_max: None,
        cost_max: None,
        count,
        limit: None,
        source_only: matches!(template, TargetTemplate::This),
        reveal_to_controller: false,
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
                optional: false,
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
                optional: false,
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
                optional: false,
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
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlaySalvage {
            count,
            optional,
            card_type,
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfWaitingRoom, *count);
            spec.card_type = *card_type;
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(spec),
                optional: *optional,
            });
        }
        AbilityTemplate::AutoOnPlaySearchDeckTop {
            count,
            optional,
            card_type,
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfDeckTop, 1);
            spec.card_type = *card_type;
            spec.limit = Some(*count);
            spec.reveal_to_controller = true;
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(spec),
                optional: *optional,
            });
        }
        AbilityTemplate::AutoOnPlayRevealDeckTop { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::RevealDeckTop {
                    count: *count,
                    audience: crate::events::RevealAudience::ControllerOnly,
                },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlayStockCharge { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::StockCharge { count: *count },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlayMillTop { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MillTop {
                    target: crate::state::TargetSide::SelfSide,
                    count: *count,
                },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnPlayHeal { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Heal,
                target: Some(target_spec_from_template(TargetTemplate::SelfClock, *count)),
                optional: false,
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
                optional: false,
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
                optional: false,
            });
        }
        AbilityTemplate::AutoOnReverseDraw { count } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::Draw { count: *count },
                target: None,
                optional: false,
            });
        }
        AbilityTemplate::AutoOnReverseSalvage {
            count,
            optional,
            card_type,
        } => {
            let mut spec = target_spec_from_template(TargetTemplate::SelfWaitingRoom, *count);
            spec.card_type = *card_type;
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Auto,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(spec),
                optional: *optional,
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
                optional: false,
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
                optional: false,
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
                optional: false,
            });
        }
        AbilityTemplate::ActivatedPaidTargetedPower {
            amount,
            count,
            target,
            ..
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
                optional: false,
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
                optional: false,
            });
        }
        AbilityTemplate::ActivatedPaidTargetedMoveToHand { count, target, .. } => {
            out.push(crate::effects::EffectSpec {
                id: crate::effects::EffectId::new(
                    crate::effects::EffectSourceKind::Activated,
                    card_id,
                    ability_index,
                    0,
                ),
                kind: crate::effects::EffectKind::MoveToHand,
                target: Some(target_spec_from_template(*target, *count)),
                optional: false,
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
                optional: false,
            });
        }
        AbilityTemplate::ActivatedPaidChangeController { count, target, .. } => {
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
                optional: false,
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
                optional: false,
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
                optional: false,
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
                optional: false,
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
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::MoveToHand => (
                crate::effects::EffectKind::MoveToHand,
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::MoveToWaitingRoom => (
                crate::effects::EffectKind::MoveToWaitingRoom,
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::MoveToStock => (
                crate::effects::EffectKind::MoveToStock,
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::MoveToClock => (
                crate::effects::EffectKind::MoveToClock,
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::Heal => (
                crate::effects::EffectKind::Heal,
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::RestTarget => (
                crate::effects::EffectKind::RestTarget,
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::StandTarget => (
                crate::effects::EffectKind::StandTarget,
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::StockCharge { count } => (
                crate::effects::EffectKind::StockCharge { count: *count },
                None,
            ),
            EffectTemplate::MillTop { target, count } => (
                crate::effects::EffectKind::MillTop {
                    target: *target,
                    count: *count,
                },
                None,
            ),
            EffectTemplate::MoveStageSlot { slot } => (
                crate::effects::EffectKind::MoveStageSlot { slot: *slot },
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 1)),
            ),
            EffectTemplate::SwapStageSlots => (
                crate::effects::EffectKind::SwapStageSlots,
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
                    .map(|t| target_spec_from_template(*t, 2)),
            ),
            EffectTemplate::RandomDiscardFromHand { target, count } => (
                crate::effects::EffectKind::RandomDiscardFromHand {
                    target: *target,
                    count: *count,
                },
                None,
            ),
            EffectTemplate::RandomMill { target, count } => (
                crate::effects::EffectKind::RandomMill {
                    target: *target,
                    count: *count,
                },
                None,
            ),
            EffectTemplate::RevealZoneTop {
                target,
                zone,
                count,
                audience,
            } => (
                crate::effects::EffectKind::RevealZoneTop {
                    target: *target,
                    zone: *zone,
                    count: *count,
                    audience: *audience,
                },
                None,
            ),
            EffectTemplate::ChangeController => (
                crate::effects::EffectKind::ChangeController {
                    new_controller: crate::state::TargetSide::SelfSide,
                },
                ability
                    .targets
                    .get(effect_index)
                    .or_else(|| ability.targets.first())
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
        let target = target.map(|mut spec| {
            if let Some(card_type) = ability.target_card_type {
                spec.card_type = Some(card_type);
            }
            if let Some(trait_id) = ability.target_trait {
                spec.card_trait = Some(trait_id);
            }
            if let Some(level_max) = ability.target_level_max {
                spec.level_max = Some(level_max);
            }
            if let Some(cost_max) = ability.target_cost_max {
                spec.cost_max = Some(cost_max);
            }
            if let Some(limit) = ability.target_limit {
                if spec.zone == crate::state::TargetZone::DeckTop {
                    spec.limit = Some(limit);
                }
            }
            spec
        });
        effects.push(crate::effects::EffectSpec {
            id: effect_id,
            kind,
            target,
            optional: false,
        });
    }
    effects
}
