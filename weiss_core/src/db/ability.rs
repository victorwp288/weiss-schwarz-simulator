use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::events::RevealAudience;
use crate::state::{TargetSide, TargetZone};

use super::types::{CardId, CardType, EffectTemplate, TargetTemplate};

/// Cost requirements for an activated ability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilityCost {
    #[serde(default)]
    /// Stock cost to pay.
    pub stock: u8,
    #[serde(default)]
    /// Whether the source must rest itself.
    pub rest_self: bool,
    #[serde(default)]
    /// Number of other characters to rest.
    pub rest_other: u8,
    #[serde(default)]
    /// Cards to discard from hand.
    pub discard_from_hand: u8,
    #[serde(default)]
    /// Cards to clock from hand.
    pub clock_from_hand: u8,
    #[serde(default)]
    /// Cards to clock from top of deck.
    pub clock_from_deck_top: u8,
    #[serde(default)]
    /// Cards to reveal from hand.
    pub reveal_from_hand: u8,
}

impl AbilityCost {
    /// Whether this cost is empty (no payments required).
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

/// Fully specified ability definition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AbilityDef {
    /// Ability kind (continuous/activated/auto).
    pub kind: AbilityKind,
    /// Optional timing for auto/continuous effects.
    pub timing: Option<AbilityTiming>,
    /// Effect templates executed by this ability.
    pub effects: Vec<EffectTemplate>,
    /// Target templates for the ability.
    pub targets: Vec<TargetTemplate>,
    #[serde(default)]
    /// Costs required to activate (only for activated abilities).
    pub cost: AbilityCost,
    #[serde(default)]
    /// Optional target card type restriction.
    pub target_card_type: Option<CardType>,
    #[serde(default)]
    /// Optional target trait restriction.
    pub target_trait: Option<u16>,
    #[serde(default)]
    /// Optional target max level restriction.
    pub target_level_max: Option<u8>,
    #[serde(default)]
    /// Optional target max cost restriction.
    pub target_cost_max: Option<u8>,
    #[serde(default)]
    /// Optional target count limit.
    pub target_limit: Option<u8>,
}

impl AbilityDef {
    /// Validate structural constraints for the definition.
    pub fn validate(&self) -> Result<()> {
        if self.effects.is_empty() {
            anyhow::bail!("AbilityDef must contain at least one effect");
        }
        if self.effects.len() > u8::MAX as usize {
            anyhow::bail!("AbilityDef has too many effects");
        }
        if self.targets.len() > u8::MAX as usize {
            anyhow::bail!("AbilityDef has too many targets");
        }
        if self.kind != AbilityKind::Activated && !self.cost.is_empty() {
            anyhow::bail!("AbilityDef cost is only valid for activated abilities");
        }
        Ok(())
    }
}

/// Timing windows for triggered abilities.
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

/// Template-driven ability definitions used by the DB loader.
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

/// High-level ability kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityKind {
    /// Continuous modifiers that always apply.
    Continuous,
    /// Activated abilities with explicit costs.
    Activated,
    /// Auto abilities that trigger at timings.
    Auto,
}

/// Canonical ability specification after parsing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilitySpec {
    /// Ability kind (continuous/activated/auto).
    pub kind: AbilityKind,
    /// Template describing behavior.
    pub template: AbilityTemplate,
}

/// Lightweight tags for ability templates (used in analytics/validation).
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
    /// Return the template tag for this ability.
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

    /// Return the stock cost for activated templates (if any).
    pub fn activation_cost(&self) -> Option<u8> {
        match self {
            AbilityTemplate::ActivatedPaidTargetedPower { cost, .. }
            | AbilityTemplate::ActivatedPaidTargetedMoveToHand { cost, .. }
            | AbilityTemplate::ActivatedPaidChangeController { cost, .. } => Some(*cost),
            _ => None,
        }
    }

    /// Return a full cost spec for activated templates.
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

    /// Return the implied timing for this template, if any.
    pub fn timing(&self) -> Option<AbilityTiming> {
        match self {
            AbilityTemplate::AutoOnPlayDraw { .. }
            | AbilityTemplate::AutoOnPlaySalvage { .. }
            | AbilityTemplate::AutoOnPlaySearchDeckTop { .. }
            | AbilityTemplate::AutoOnPlayRevealDeckTop { .. }
            | AbilityTemplate::AutoOnPlayStockCharge { .. }
            | AbilityTemplate::AutoOnPlayMillTop { .. }
            | AbilityTemplate::AutoOnPlayHeal { .. } => Some(AbilityTiming::OnPlay),
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

    /// Whether this template represents an event play.
    pub fn is_event_play(&self) -> bool {
        matches!(self, AbilityTemplate::EventDealDamage { .. })
    }
}

impl AbilitySpec {
    /// Build an ability spec from a template.
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

    /// Return the implied timing for this spec, if any.
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

fn i32_key(value: i32) -> u64 {
    // Map signed i32 to u64 while preserving numeric order.
    (value as u32 ^ 0x8000_0000) as u64
}

fn push_opt_u8(out: &mut Vec<u64>, value: Option<u8>) {
    match value {
        None => {
            out.push(0);
            out.push(0);
        }
        Some(val) => {
            out.push(1);
            out.push(val as u64);
        }
    }
}

fn push_opt_u16(out: &mut Vec<u64>, value: Option<u16>) {
    match value {
        None => {
            out.push(0);
            out.push(0);
        }
        Some(val) => {
            out.push(1);
            out.push(val as u64);
        }
    }
}

fn target_template_key(target: TargetTemplate) -> u64 {
    match target {
        TargetTemplate::OppFrontRow => 0,
        TargetTemplate::OppBackRow => 1,
        TargetTemplate::OppStage => 2,
        TargetTemplate::OppStageSlot { slot } => 3 + slot as u64,
        TargetTemplate::SelfFrontRow => 10,
        TargetTemplate::SelfBackRow => 11,
        TargetTemplate::SelfStage => 12,
        TargetTemplate::SelfStageSlot { slot } => 13 + slot as u64,
        TargetTemplate::This => 20,
        TargetTemplate::SelfWaitingRoom => 21,
        TargetTemplate::SelfHand => 22,
        TargetTemplate::SelfDeckTop => 23,
        TargetTemplate::SelfClock => 24,
        TargetTemplate::SelfLevel => 25,
        TargetTemplate::SelfStock => 26,
        TargetTemplate::SelfMemory => 27,
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
            out.push(i32_key(*amount));
            out.push(u64::from(*duration_turn));
        }
        EffectTemplate::MoveToHand => {
            out.push(3);
        }
        EffectTemplate::MoveToWaitingRoom => {
            out.push(4);
        }
        EffectTemplate::MoveToStock => {
            out.push(5);
        }
        EffectTemplate::MoveToClock => {
            out.push(6);
        }
        EffectTemplate::Heal => {
            out.push(7);
        }
        EffectTemplate::RestTarget => {
            out.push(8);
        }
        EffectTemplate::StandTarget => {
            out.push(9);
        }
        EffectTemplate::StockCharge { count } => {
            out.push(10);
            out.push(*count as u64);
        }
        EffectTemplate::MillTop { target, count } => {
            out.push(11);
            out.push(target_side_key(*target));
            out.push(*count as u64);
        }
        EffectTemplate::MoveStageSlot { slot } => {
            out.push(12);
            out.push(*slot as u64);
        }
        EffectTemplate::SwapStageSlots => {
            out.push(13);
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
            out.push(17);
        }
        EffectTemplate::CounterBackup { power } => {
            out.push(18);
            out.push(i32_key(*power));
        }
        EffectTemplate::CounterDamageReduce { amount } => {
            out.push(19);
            out.push(*amount as u64);
        }
        EffectTemplate::CounterDamageCancel => {
            out.push(20);
        }
    }
}

fn ability_def_key(def: &AbilityDef) -> Vec<u64> {
    let mut out = Vec::new();
    out.push(ability_kind_key(def.kind));
    out.push(ability_timing_key(def.timing));
    out.push(def.effects.len() as u64);
    for effect in &def.effects {
        effect_template_key(effect, &mut out);
    }
    out.push(def.targets.len() as u64);
    for target in &def.targets {
        out.push(target_template_key(*target));
    }
    out.push(def.cost.stock as u64);
    out.push(u64::from(def.cost.rest_self));
    out.push(def.cost.rest_other as u64);
    out.push(def.cost.discard_from_hand as u64);
    out.push(def.cost.clock_from_hand as u64);
    out.push(def.cost.clock_from_deck_top as u64);
    out.push(def.cost.reveal_from_hand as u64);
    out.push(card_type_key(def.target_card_type));
    push_opt_u16(&mut out, def.target_trait);
    push_opt_u8(&mut out, def.target_level_max);
    push_opt_u8(&mut out, def.target_cost_max);
    push_opt_u8(&mut out, def.target_limit);
    out
}

fn ability_template_key(template: &AbilityTemplate) -> Vec<u64> {
    let mut out = Vec::new();
    match template {
        AbilityTemplate::Vanilla => {
            out.push(0);
        }
        AbilityTemplate::ContinuousPower { amount } => {
            out.push(1);
            out.push(i32_key(*amount));
        }
        AbilityTemplate::ContinuousCannotAttack => {
            out.push(2);
        }
        AbilityTemplate::ContinuousAttackCost { cost } => {
            out.push(3);
            out.push(*cost as u64);
        }
        AbilityTemplate::AutoOnPlayDraw { count } => {
            out.push(4);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnPlaySalvage {
            count,
            optional,
            card_type,
        } => {
            out.push(5);
            out.push(*count as u64);
            out.push(u64::from(*optional));
            out.push(card_type_key(*card_type));
        }
        AbilityTemplate::AutoOnPlaySearchDeckTop {
            count,
            optional,
            card_type,
        } => {
            out.push(6);
            out.push(*count as u64);
            out.push(u64::from(*optional));
            out.push(card_type_key(*card_type));
        }
        AbilityTemplate::AutoOnPlayRevealDeckTop { count } => {
            out.push(7);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnPlayStockCharge { count } => {
            out.push(8);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnPlayMillTop { count } => {
            out.push(9);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnPlayHeal { count } => {
            out.push(10);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnAttackDealDamage { amount, cancelable } => {
            out.push(11);
            out.push(*amount as u64);
            out.push(u64::from(*cancelable));
        }
        AbilityTemplate::AutoEndPhaseDraw { count } => {
            out.push(12);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnReverseDraw { count } => {
            out.push(13);
            out.push(*count as u64);
        }
        AbilityTemplate::AutoOnReverseSalvage {
            count,
            optional,
            card_type,
        } => {
            out.push(14);
            out.push(*count as u64);
            out.push(u64::from(*optional));
            out.push(card_type_key(*card_type));
        }
        AbilityTemplate::EventDealDamage { amount, cancelable } => {
            out.push(15);
            out.push(*amount as u64);
            out.push(u64::from(*cancelable));
        }
        AbilityTemplate::ActivatedPlaceholder => {
            out.push(16);
        }
        AbilityTemplate::ActivatedTargetedPower {
            amount,
            count,
            target,
        } => {
            out.push(17);
            out.push(i32_key(*amount));
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedPaidTargetedPower {
            cost,
            amount,
            count,
            target,
        } => {
            out.push(18);
            out.push(*cost as u64);
            out.push(i32_key(*amount));
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedTargetedMoveToHand { count, target } => {
            out.push(19);
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedPaidTargetedMoveToHand {
            cost,
            count,
            target,
        } => {
            out.push(20);
            out.push(*cost as u64);
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedChangeController { count, target } => {
            out.push(21);
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::ActivatedPaidChangeController {
            cost,
            count,
            target,
        } => {
            out.push(22);
            out.push(*cost as u64);
            out.push(*count as u64);
            out.push(target_template_key(*target));
        }
        AbilityTemplate::CounterBackup { power } => {
            out.push(23);
            out.push(i32_key(*power));
        }
        AbilityTemplate::CounterDamageReduce { amount } => {
            out.push(24);
            out.push(*amount as u64);
        }
        AbilityTemplate::CounterDamageCancel => {
            out.push(25);
        }
        AbilityTemplate::AbilityDef(def) => {
            out.push(26);
            out.extend(ability_def_key(def));
        }
        AbilityTemplate::Unsupported { id } => {
            out.push(27);
            out.push(*id as u64);
        }
    }
    out
}

pub(crate) fn ability_sort_key(spec: &AbilitySpec) -> (u8, Vec<u64>) {
    let tag = spec.template.tag() as u8;
    (tag, ability_template_key(&spec.template))
}

pub(crate) fn target_spec_from_template(
    template: TargetTemplate,
    count: u8,
) -> crate::state::TargetSpec {
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

pub(crate) fn compile_effects_from_template(
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
                target: Some(target_spec_from_template(TargetTemplate::This, 1)),
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
                target: Some(target_spec_from_template(TargetTemplate::This, 1)),
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
                target: Some(target_spec_from_template(TargetTemplate::This, 1)),
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
                    duration: crate::state::ModifierDuration::WhileOnStage,
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
                    crate::effects::EffectSourceKind::Activated,
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
                    crate::effects::EffectSourceKind::Activated,
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
                    crate::effects::EffectSourceKind::Activated,
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
        AbilityTemplate::ActivatedPlaceholder => {}
    }
    out
}

pub(crate) fn compile_effects_from_def(
    card_id: CardId,
    ability_index: u8,
    def: &AbilityDef,
) -> Vec<crate::effects::EffectSpec> {
    let mut out = Vec::new();
    let max_len = def.effects.len().max(def.targets.len());
    let source_kind = match def.kind {
        AbilityKind::Activated => crate::effects::EffectSourceKind::Activated,
        AbilityKind::Auto => crate::effects::EffectSourceKind::Auto,
        AbilityKind::Continuous => crate::effects::EffectSourceKind::Continuous,
    };
    for idx in 0..max_len {
        let Some(effect) = def.effects.get(idx) else {
            continue;
        };
        let target = def
            .targets
            .get(idx)
            .copied()
            .or_else(|| def.targets.first().copied());
        let target_spec = target.map(|t| {
            let mut spec = target_spec_from_template(t, 1);
            spec.card_type = def.target_card_type;
            spec.card_trait = def.target_trait;
            spec.level_max = def.target_level_max;
            spec.cost_max = def.target_cost_max;
            spec.limit = def.target_limit;
            spec
        });
        let effect_index = match u8::try_from(idx) {
            Ok(val) => val,
            Err(_) => {
                debug_assert!(false, "Effect index out of range for card {}", card_id);
                continue;
            }
        };
        out.push(crate::effects::EffectSpec {
            id: crate::effects::EffectId::new(source_kind, card_id, ability_index, effect_index),
            kind: match effect {
                EffectTemplate::Draw { count } => {
                    crate::effects::EffectKind::Draw { count: *count }
                }
                EffectTemplate::DealDamage { amount, cancelable } => {
                    crate::effects::EffectKind::Damage {
                        amount: *amount as i32,
                        cancelable: *cancelable,
                        damage_type: crate::state::DamageType::Effect,
                    }
                }
                EffectTemplate::AddPower {
                    amount,
                    duration_turn,
                } => crate::effects::EffectKind::AddModifier {
                    kind: crate::state::ModifierKind::Power,
                    magnitude: *amount,
                    duration: if *duration_turn {
                        crate::state::ModifierDuration::UntilEndOfTurn
                    } else {
                        crate::state::ModifierDuration::WhileOnStage
                    },
                },
                EffectTemplate::MoveToHand => crate::effects::EffectKind::MoveToHand,
                EffectTemplate::MoveToWaitingRoom => crate::effects::EffectKind::MoveToWaitingRoom,
                EffectTemplate::MoveToStock => crate::effects::EffectKind::MoveToStock,
                EffectTemplate::MoveToClock => crate::effects::EffectKind::MoveToClock,
                EffectTemplate::Heal => crate::effects::EffectKind::Heal,
                EffectTemplate::RestTarget => crate::effects::EffectKind::RestTarget,
                EffectTemplate::StandTarget => crate::effects::EffectKind::StandTarget,
                EffectTemplate::StockCharge { count } => {
                    crate::effects::EffectKind::StockCharge { count: *count }
                }
                EffectTemplate::MillTop { target, count } => crate::effects::EffectKind::MillTop {
                    target: *target,
                    count: *count,
                },
                EffectTemplate::MoveStageSlot { slot } => {
                    crate::effects::EffectKind::MoveStageSlot { slot: *slot }
                }
                EffectTemplate::SwapStageSlots => crate::effects::EffectKind::SwapStageSlots,
                EffectTemplate::RandomDiscardFromHand { target, count } => {
                    crate::effects::EffectKind::RandomDiscardFromHand {
                        target: *target,
                        count: *count,
                    }
                }
                EffectTemplate::RandomMill { target, count } => {
                    crate::effects::EffectKind::RandomMill {
                        target: *target,
                        count: *count,
                    }
                }
                EffectTemplate::RevealZoneTop {
                    target,
                    zone,
                    count,
                    audience,
                } => crate::effects::EffectKind::RevealZoneTop {
                    target: *target,
                    zone: *zone,
                    count: *count,
                    audience: *audience,
                },
                EffectTemplate::ChangeController => crate::effects::EffectKind::ChangeController {
                    new_controller: crate::state::TargetSide::SelfSide,
                },
                EffectTemplate::CounterBackup { power } => {
                    crate::effects::EffectKind::CounterBackup { power: *power }
                }
                EffectTemplate::CounterDamageReduce { amount } => {
                    crate::effects::EffectKind::CounterDamageReduce { amount: *amount }
                }
                EffectTemplate::CounterDamageCancel => {
                    crate::effects::EffectKind::CounterDamageCancel
                }
            },
            target: target_spec,
            optional: idx >= def.targets.len(),
        });
    }
    out
}
