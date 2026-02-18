/// Cost requirements for an activated ability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityCostStep {
    #[serde(alias = "restOther", alias = "rest_other")]
    RestOther,
    #[serde(alias = "sacrificeFromStage", alias = "sacrifice_from_stage")]
    SacrificeFromStage,
    #[serde(alias = "discardFromHand", alias = "discard_from_hand")]
    DiscardFromHand,
    #[serde(alias = "clockFromHand", alias = "clock_from_hand")]
    ClockFromHand,
    #[serde(alias = "clockFromDeckTop", alias = "clock_from_deck_top")]
    ClockFromDeckTop,
    #[serde(alias = "revealFromHand", alias = "reveal_from_hand")]
    RevealFromHand,
}

/// Cost requirements for an activated ability.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    #[serde(default, alias = "sacrificeFromStage")]
    /// Characters to put from stage into waiting room as cost.
    pub sacrifice_from_stage: u8,
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
    #[serde(default, alias = "moveSelfToWaitingRoom")]
    /// Whether the source card must be put into waiting room as cost.
    pub move_self_to_waiting_room: bool,
    #[serde(default, alias = "returnSelfToHand")]
    /// Whether the source card must be returned to hand as cost.
    pub return_self_to_hand: bool,
    #[serde(default, alias = "stepOrder")]
    /// Optional explicit ordering for staged cost steps.
    pub step_order: Vec<AbilityCostStep>,
}

impl AbilityCost {
    /// Whether this cost is empty (no payments required).
    pub fn is_empty(&self) -> bool {
        self.stock == 0
            && !self.rest_self
            && self.rest_other == 0
            && self.sacrifice_from_stage == 0
            && self.discard_from_hand == 0
            && self.clock_from_hand == 0
            && self.clock_from_deck_top == 0
            && self.reveal_from_hand == 0
            && !self.move_self_to_waiting_room
            && !self.return_self_to_hand
    }

    /// Return the next pending staged step in explicit order, if any.
    pub fn next_explicit_step(&self) -> Option<crate::state::CostStepKind> {
        for step in &self.step_order {
            match step {
                AbilityCostStep::RestOther if self.rest_other > 0 => {
                    return Some(crate::state::CostStepKind::RestOther);
                }
                AbilityCostStep::SacrificeFromStage if self.sacrifice_from_stage > 0 => {
                    return Some(crate::state::CostStepKind::SacrificeFromStage);
                }
                AbilityCostStep::DiscardFromHand if self.discard_from_hand > 0 => {
                    return Some(crate::state::CostStepKind::DiscardFromHand);
                }
                AbilityCostStep::ClockFromHand if self.clock_from_hand > 0 => {
                    return Some(crate::state::CostStepKind::ClockFromHand);
                }
                AbilityCostStep::ClockFromDeckTop if self.clock_from_deck_top > 0 => {
                    return Some(crate::state::CostStepKind::ClockFromDeckTop);
                }
                AbilityCostStep::RevealFromHand if self.reveal_from_hand > 0 => {
                    return Some(crate::state::CostStepKind::RevealFromHand);
                }
                _ => {}
            }
        }
        None
    }
}

const fn default_target_side_self() -> TargetSide {
    TargetSide::SelfSide
}

/// Ability-level conditional requirements.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AbilityDefConditions {
    #[serde(default, alias = "requiresApproxEffects")]
    /// If true, this ability only runs when curriculum approximation effects are enabled.
    pub requires_approx_effects: bool,
    #[serde(default, alias = "climaxArea")]
    /// Optional climax-area gate for this ability.
    pub climax_area: Option<AbilityDefClimaxAreaCondition>,
    #[serde(default)]
    /// Optional turn gate for this ability.
    pub turn: Option<ConditionTurn>,
    #[serde(default, alias = "handLevelDelta")]
    /// Optional play-from-hand level adjustment (typically negative).
    pub hand_level_delta: i8,
    #[serde(default, alias = "selfWaitingRoomClimaxAtMost")]
    /// Optional gate on self waiting-room climax count.
    pub self_waiting_room_climax_at_most: Option<u8>,
    #[serde(default, alias = "selfClockCardIdsAny")]
    /// Optional gate requiring at least one of these card ids in self clock.
    pub self_clock_card_ids_any: Vec<CardId>,
    #[serde(default, alias = "selfMemoryCardIdsAny")]
    /// Optional gate requiring at least one of these card ids in self memory.
    pub self_memory_card_ids_any: Vec<CardId>,
    #[serde(default, alias = "selfMemoryAtMost")]
    /// Optional gate requiring self memory count to be at most this value.
    pub self_memory_at_most: Option<u8>,
    #[serde(default, alias = "opponentStageHasLevelAtLeast")]
    /// Optional gate requiring the opponent to have a stage character at or above this level.
    pub opponent_stage_has_level_at_least: Option<u8>,
    #[serde(default, alias = "triggerCheckRevealedClimax")]
    /// Optional gate requiring the current trigger check card to be a climax.
    pub trigger_check_revealed_climax: bool,
    #[serde(default, alias = "triggerCheckRevealedIcon")]
    /// Optional gate requiring the current trigger check card to include this trigger icon.
    pub trigger_check_revealed_icon: Option<TriggerIcon>,
    #[serde(default, alias = "ignoreColorRequirement")]
    /// If true, this card can be played without color requirements while in hand.
    pub ignore_color_requirement: bool,
    #[serde(default, alias = "zoneCount")]
    /// Optional generic zone-count gate for this ability.
    pub zone_count: Option<ZoneCountCondition>,
    #[serde(default, alias = "sourceRuleId")]
    /// Optional parser-v2 rule-pack provenance identifier.
    pub source_rule_id: Option<String>,
}

/// Climax-area gate configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AbilityDefClimaxAreaCondition {
    #[serde(default = "default_target_side_self")]
    /// Side whose climax area is checked.
    pub side: TargetSide,
    #[serde(default, alias = "cardIds")]
    /// Allowed climax ids. Empty means any climax card satisfies the gate.
    pub card_ids: Vec<CardId>,
}

impl Default for AbilityDefClimaxAreaCondition {
    fn default() -> Self {
        Self {
            side: default_target_side_self(),
            card_ids: Vec::new(),
        }
    }
}

impl AbilityDefConditions {
    fn has_play_requirement_overrides(&self) -> bool {
        self.hand_level_delta != 0 || self.ignore_color_requirement
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
    #[serde(default)]
    /// Optionality flags per effect index.
    pub effect_optional: Vec<bool>,
    /// Target templates for the ability.
    pub targets: Vec<TargetTemplate>,
    #[serde(default)]
    /// Costs required to activate (only for activated abilities).
    pub cost: AbilityCost,
    #[serde(default, alias = "condition")]
    /// Optional ability-level conditions.
    pub conditions: AbilityDefConditions,
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
    #[serde(default, alias = "targetCardIds")]
    /// Optional target card-id restriction.
    pub target_card_ids: Vec<CardId>,
    #[serde(default)]
    /// Optional target count limit.
    pub target_limit: Option<u8>,
}

impl AbilityDef {
    /// Validate structural constraints for the definition.
    pub fn validate(&self) -> Result<()> {
        if self.effects.is_empty() && !self.conditions.has_play_requirement_overrides() {
            anyhow::bail!("AbilityDef must contain at least one effect");
        }
        if self.effects.len() > u8::MAX as usize {
            anyhow::bail!("AbilityDef has too many effects");
        }
        if self.effect_optional.len() > self.effects.len() {
            anyhow::bail!("AbilityDef optional flags exceed effects length");
        }
        if self.targets.len() > u8::MAX as usize {
            anyhow::bail!("AbilityDef has too many targets");
        }
        if self.kind == AbilityKind::Continuous && !self.cost.is_empty() {
            anyhow::bail!("AbilityDef cost is invalid for continuous abilities");
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
    LevelUp,
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
    OtherAttackDeclaration,
    TriggerResolution,
    Counter,
    UseAct,
    DamageResolution,
    Encore,
    OnPlay,
    OnReverse,
    BattleOpponentReverse,
    DamageDealtNotCanceled,
    DamageReceivedNotCanceled,
    DamageDealtCanceled,
    DamageReceivedCanceled,
}

/// Template-driven ability definitions used by the DB loader.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(clippy::large_enum_variant)]
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
    Bond {
        cost: AbilityCost,
        count: u8,
        #[serde(default)]
        target_ids: Vec<CardId>,
    },
    EncoreVariant {
        cost: AbilityCost,
    },
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
    Bond,
    EncoreVariant,
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
            AbilityTemplate::Bond { .. } => AbilityTemplateTag::Bond,
            AbilityTemplate::EncoreVariant { .. } => AbilityTemplateTag::EncoreVariant,
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
            AbilityTemplate::Bond { cost, .. } => cost.clone(),
            AbilityTemplate::AbilityDef(def) => def.cost.clone(),
            _ => AbilityCost::default(),
        }
    }

    /// Return encore variant cost for keyword encore templates.
    pub fn encore_variant_cost(&self) -> Option<AbilityCost> {
        match self {
            AbilityTemplate::EncoreVariant { cost } => Some(cost.clone()),
            _ => None,
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
            | AbilityTemplate::AutoOnPlayHeal { .. }
            | AbilityTemplate::Bond { .. } => Some(AbilityTiming::OnPlay),
            AbilityTemplate::AutoOnAttackDealDamage { .. } => {
                Some(AbilityTiming::AttackDeclaration)
            }
            AbilityTemplate::AutoEndPhaseDraw { .. } => Some(AbilityTiming::EndPhase),
            AbilityTemplate::AutoOnReverseDraw { .. } => Some(AbilityTiming::OnReverse),
            AbilityTemplate::AutoOnReverseSalvage { .. } => Some(AbilityTiming::OnReverse),
            AbilityTemplate::CounterBackup { .. }
            | AbilityTemplate::CounterDamageReduce { .. }
            | AbilityTemplate::CounterDamageCancel => Some(AbilityTiming::Counter),
            AbilityTemplate::EncoreVariant { .. } => Some(AbilityTiming::Encore),
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
        self.template.timing()
    }
}
