use std::collections::BTreeMap;

use anyhow::Result;
use serde::Serialize;

use crate::config::ObservationVisibility;
use crate::db::{CardColor, CardId, CardType, TriggerIcon};
use crate::encode::{action_desc_for_id, decode_action_id, ActionParamValue, MAX_STAGE};
use crate::events::Zone;
use crate::fingerprint::{hash_bytes, hash_postcard};
use crate::legal::{ActionDesc, DecisionKind};
use crate::state::{AttackType, ChoiceOptionRef, ChoiceReason, ChoiceZone, StageStatus};
use crate::visibility_policy::{
    target_zone_identity_visibility, zone_identity_visibility, ZoneIdentityVisibility,
};

use super::{GameEnv, VisibilityContext};

const HUMAN_VIEW_SCHEMA_VERSION: &str = "human_decision_view_v1";
const PUBLIC_EVENT_LOG_LIMIT: usize = 16;

#[derive(Clone, Debug, Serialize)]
struct HumanDecisionViewCore {
    schema_version: &'static str,
    simulator_version: &'static str,
    env_index: u32,
    episode_key: String,
    episode_seed: u64,
    episode_index: u32,
    decision_id: u32,
    summary: HumanSummaryView,
    stage_layout: HumanStageLayoutView,
    players: Vec<HumanPlayerView>,
    public_event_log: Vec<serde_json::Value>,
    legal_actions: Vec<HumanLegalActionView>,
    legal_action_ids: Vec<u16>,
    legal_fingerprint64: String,
}

#[derive(Clone, Debug, Serialize)]
struct HumanSummaryView {
    turn_player: u8,
    actor_seat: Option<u8>,
    viewer_seat: u8,
    phase: &'static str,
    decision_kind: Option<&'static str>,
    decision_id: u32,
    turn_count: u32,
    turn_number: u32,
    decision_count: u32,
    tick_count: u32,
    terminal: Option<String>,
    players: Vec<HumanPlayerCountsView>,
}

#[derive(Clone, Debug, Serialize)]
struct HumanPlayerCountsView {
    seat: u8,
    relative: &'static str,
    level_count: usize,
    clock_count: usize,
    hand_count: usize,
    stock_count: usize,
    deck_count: usize,
    waiting_room_count: usize,
    memory_count: usize,
    climax_count: usize,
    resolution_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct HumanStageLayoutView {
    center_slots: Vec<u8>,
    back_slots: Vec<u8>,
    slots: Vec<HumanStageSlotMetaView>,
}

#[derive(Clone, Debug, Serialize)]
struct HumanStageSlotMetaView {
    slot: u8,
    row: &'static str,
    label: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct HumanPlayerView {
    seat: u8,
    relative: &'static str,
    counts: HumanPlayerCountsView,
    zones: HumanZonesView,
    stage: Vec<HumanStageSlotView>,
}

#[derive(Clone, Debug, Serialize)]
struct HumanZonesView {
    deck: HumanZoneView,
    hand: HumanZoneView,
    waiting_room: HumanZoneView,
    clock: HumanZoneView,
    level: HumanZoneView,
    stock: HumanZoneView,
    memory: HumanZoneView,
    climax: HumanZoneView,
    resolution: HumanZoneView,
}

#[derive(Clone, Debug, Serialize)]
struct HumanZoneView {
    zone: &'static str,
    owner_seat: u8,
    relative_owner: &'static str,
    count: usize,
    visibility: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cards: Option<Vec<HumanZoneCardView>>,
}

#[derive(Clone, Debug, Serialize)]
struct HumanZoneCardView {
    card_ref: String,
    zone: &'static str,
    owner_seat: u8,
    relative_owner: &'static str,
    index: usize,
    visibility: &'static str,
    card: HumanCardRecord,
}

#[derive(Clone, Debug, Serialize)]
struct HumanStageSlotView {
    slot: u8,
    row: &'static str,
    label: &'static str,
    slot_ref: String,
    owner_seat: u8,
    relative_owner: &'static str,
    visibility: &'static str,
    empty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    card_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card: Option<HumanCardRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    orientation: Option<&'static str>,
    marker_count: usize,
    has_attacked: bool,
    cannot_attack: bool,
    attack_cost: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    power: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    soul: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_soul: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
struct HumanCardRecord {
    card_id: CardId,
    #[serde(skip_serializing_if = "Option::is_none")]
    card_type: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    power: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    soul: Option<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    triggers: Vec<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    traits: Vec<u16>,
}

#[derive(Clone, Debug, Serialize)]
struct HumanLegalActionView {
    index: usize,
    action_id: u16,
    family: String,
    label: String,
    short_label: String,
    description: String,
    params: BTreeMap<String, HumanActionParamValue>,
    source_refs: Vec<HumanActionRefView>,
    target_refs: Vec<HumanActionRefView>,
    is_pass: bool,
    is_attack: bool,
    is_play: bool,
    is_move: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum HumanActionParamValue {
    Int(i32),
    Str(String),
}

#[derive(Clone, Debug, Serialize)]
struct HumanActionRefView {
    ref_id: String,
    zone: &'static str,
    owner_seat: u8,
    relative_owner: &'static str,
    visibility: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card: Option<HumanCardRecord>,
}

impl GameEnv {
    /// Build a redacted, JSON-serialized view of the current decision for human UIs.
    ///
    /// Legal actions are decoded only from the current cached legal id list, preserving
    /// the simulator's decision-boundary action contract and ordering.
    pub fn human_decision_view_json(&self, perspective_seat: Option<u8>) -> Result<String> {
        let viewer = self.resolve_human_viewer(perspective_seat)?;
        let ctx = VisibilityContext {
            viewer: Some(viewer),
            mode: ObservationVisibility::Public,
            policies_enabled: true,
        };
        let event_ctx = VisibilityContext {
            viewer: None,
            mode: ObservationVisibility::Public,
            policies_enabled: true,
        };
        let legal_action_ids = self.action_ids_cache().to_vec();
        let legal_fingerprint64 = self.legal_fingerprint64(&legal_action_ids);
        let core = HumanDecisionViewCore {
            schema_version: HUMAN_VIEW_SCHEMA_VERSION,
            simulator_version: env!("CARGO_PKG_VERSION"),
            env_index: self.env_id,
            episode_key: format!(
                "env{}:episode{}:{:016x}",
                self.env_id, self.episode_index, self.episode_seed
            ),
            episode_seed: self.episode_seed,
            episode_index: self.episode_index,
            decision_id: self.decision_id(),
            summary: self.build_human_summary(viewer),
            stage_layout: self.build_human_stage_layout(),
            players: self.build_human_players(viewer, ctx),
            public_event_log: self.build_public_event_log(event_ctx),
            legal_actions: self.build_human_legal_actions(viewer, ctx, &legal_action_ids),
            legal_action_ids,
            legal_fingerprint64,
        };
        let view_hash64 = format_hash64(hash_postcard(&core));
        let mut value = serde_json::to_value(core)?;
        if let serde_json::Value::Object(map) = &mut value {
            map.insert(
                "view_hash64".to_string(),
                serde_json::Value::String(view_hash64),
            );
        }
        Ok(serde_json::to_string(&value)?)
    }

    fn resolve_human_viewer(&self, perspective_seat: Option<u8>) -> Result<u8> {
        let viewer = perspective_seat
            .or_else(|| self.decision.as_ref().map(|decision| decision.player))
            .unwrap_or(self.state.turn.active_player);
        if viewer > 1 {
            anyhow::bail!("perspective_seat must be 0, 1, or None (got {viewer})");
        }
        Ok(viewer)
    }

    fn build_human_summary(&self, viewer: u8) -> HumanSummaryView {
        let decision = self.decision.as_ref();
        HumanSummaryView {
            turn_player: self.state.turn.active_player,
            actor_seat: decision.map(|d| d.player),
            viewer_seat: viewer,
            phase: phase_name(self.state.turn.phase),
            decision_kind: decision.map(|d| decision_kind_name(d.kind)),
            decision_id: self.decision_id(),
            turn_count: self.state.turn.turn_number,
            turn_number: self.state.turn.turn_number,
            decision_count: self.state.turn.decision_count,
            tick_count: self.state.turn.tick_count,
            terminal: self.state.terminal.map(terminal_name),
            players: (0..2)
                .map(|seat| self.player_counts_view(seat, viewer))
                .collect(),
        }
    }

    fn build_human_stage_layout(&self) -> HumanStageLayoutView {
        let center_slots = if self.curriculum.reduced_stage_mode {
            vec![0]
        } else {
            vec![0, 1, 2]
        };
        let back_slots = if self.curriculum.reduced_stage_mode {
            Vec::new()
        } else {
            vec![3, 4]
        };
        let slots = (0..MAX_STAGE)
            .map(|slot| HumanStageSlotMetaView {
                slot: slot as u8,
                row: stage_row(slot as u8),
                label: stage_slot_label(slot as u8),
            })
            .collect();
        HumanStageLayoutView {
            center_slots,
            back_slots,
            slots,
        }
    }

    fn build_human_players(&self, viewer: u8, ctx: VisibilityContext) -> Vec<HumanPlayerView> {
        (0..2)
            .map(|seat| {
                let seat_u8 = seat as u8;
                HumanPlayerView {
                    seat: seat_u8,
                    relative: relative_owner(viewer, seat_u8),
                    counts: self.player_counts_view(seat_u8, viewer),
                    zones: self.build_human_zones(seat_u8, viewer, ctx),
                    stage: self.build_human_stage(seat_u8, viewer, ctx),
                }
            })
            .collect()
    }

    fn player_counts_view(&self, seat: u8, viewer: u8) -> HumanPlayerCountsView {
        let player = &self.state.players[seat as usize];
        HumanPlayerCountsView {
            seat,
            relative: relative_owner(viewer, seat),
            level_count: player.level.len(),
            clock_count: player.clock.len(),
            hand_count: player.hand.len(),
            stock_count: player.stock.len(),
            deck_count: player.deck.len(),
            waiting_room_count: player.waiting_room.len(),
            memory_count: player.memory.len(),
            climax_count: player.climax.len(),
            resolution_count: player.resolution.len(),
        }
    }

    fn build_human_zones(&self, owner: u8, viewer: u8, ctx: VisibilityContext) -> HumanZonesView {
        let player = &self.state.players[owner as usize];
        HumanZonesView {
            deck: self.zone_view(owner, viewer, ctx, Zone::Deck, &player.deck),
            hand: self.zone_view(owner, viewer, ctx, Zone::Hand, &player.hand),
            waiting_room: self.zone_view(
                owner,
                viewer,
                ctx,
                Zone::WaitingRoom,
                &player.waiting_room,
            ),
            clock: self.zone_view(owner, viewer, ctx, Zone::Clock, &player.clock),
            level: self.zone_view(owner, viewer, ctx, Zone::Level, &player.level),
            stock: self.zone_view(owner, viewer, ctx, Zone::Stock, &player.stock),
            memory: self.zone_view(owner, viewer, ctx, Zone::Memory, &player.memory),
            climax: self.zone_view(owner, viewer, ctx, Zone::Climax, &player.climax),
            resolution: self.zone_view(owner, viewer, ctx, Zone::Resolution, &player.resolution),
        }
    }

    fn zone_view(
        &self,
        owner: u8,
        viewer: u8,
        ctx: VisibilityContext,
        zone: Zone,
        cards: &[crate::state::CardInstance],
    ) -> HumanZoneView {
        let hidden = self.zone_hidden_for_viewer(ctx, owner, zone);
        let visibility = zone_visibility_label(owner, viewer, zone, &self.curriculum, hidden);
        let zone_name = zone_name(zone);
        let card_views = (!hidden).then(|| {
            cards
                .iter()
                .enumerate()
                .map(|(index, card)| HumanZoneCardView {
                    card_ref: card_ref(viewer, owner, zone_name, index as u16),
                    zone: zone_name,
                    owner_seat: owner,
                    relative_owner: relative_owner(viewer, owner),
                    index,
                    visibility,
                    card: self.card_record(card.id),
                })
                .collect()
        });
        HumanZoneView {
            zone: zone_name,
            owner_seat: owner,
            relative_owner: relative_owner(viewer, owner),
            count: cards.len(),
            visibility,
            cards: card_views,
        }
    }

    fn build_human_stage(
        &self,
        owner: u8,
        viewer: u8,
        _ctx: VisibilityContext,
    ) -> Vec<HumanStageSlotView> {
        let visibility = zone_visibility_label(owner, viewer, Zone::Stage, &self.curriculum, false);
        self.state.players[owner as usize]
            .stage
            .iter()
            .enumerate()
            .map(|(slot, slot_state)| {
                let slot_u8 = slot as u8;
                let card = slot_state.card.map(|card| self.card_record(card.id));
                let power = slot_state
                    .card
                    .map(|card| self.effective_slot_power(owner as usize, slot, card.id));
                let level = slot_state
                    .card
                    .map(|_| self.compute_slot_level(owner as usize, slot).max(0));
                let effective_soul = slot_state
                    .card
                    .map(|card| self.effective_slot_soul(owner as usize, slot, card.id));
                HumanStageSlotView {
                    slot: slot_u8,
                    row: stage_row(slot_u8),
                    label: stage_slot_label(slot_u8),
                    slot_ref: card_ref(viewer, owner, "stage", slot_u8 as u16),
                    owner_seat: owner,
                    relative_owner: relative_owner(viewer, owner),
                    visibility,
                    empty: slot_state.card.is_none(),
                    card_ref: slot_state
                        .card
                        .map(|_| card_ref(viewer, owner, "stage", slot_u8 as u16)),
                    card,
                    orientation: slot_state
                        .card
                        .map(|_| stage_status_name(slot_state.status)),
                    marker_count: slot_state.markers.len(),
                    has_attacked: slot_state.has_attacked,
                    cannot_attack: slot_state.cannot_attack,
                    attack_cost: slot_state.attack_cost,
                    power,
                    soul: slot_state.card.map(|card| self.db.soul_by_id(card.id)),
                    effective_soul,
                    level,
                    cost: slot_state.card.map(|card| self.db.cost_by_id(card.id)),
                    color: slot_state
                        .card
                        .map(|card| color_name(self.db.color_by_id(card.id))),
                }
            })
            .collect()
    }

    fn build_public_event_log(&self, ctx: VisibilityContext) -> Vec<serde_json::Value> {
        let events = self.canonical_events();
        let start = events.len().saturating_sub(PUBLIC_EVENT_LOG_LIMIT);
        events[start..]
            .iter()
            .filter_map(|event| {
                let sanitized = self.sanitize_event_for_viewer(event, ctx);
                serde_json::to_value(sanitized)
                    .ok()
                    .map(strip_instance_ids_from_value)
            })
            .collect()
    }

    fn build_human_legal_actions(
        &self,
        viewer: u8,
        ctx: VisibilityContext,
        legal_action_ids: &[u16],
    ) -> Vec<HumanLegalActionView> {
        let actor = self.decision.as_ref().map(|d| d.player);
        legal_action_ids
            .iter()
            .enumerate()
            .map(|(index, &action_id)| {
                let desc = decode_action_id(action_id as usize);
                let family = desc
                    .as_ref()
                    .map(|d| d.family.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let params = desc.as_ref().map(action_params_map).unwrap_or_default();
                let action = action_desc_for_id(action_id as usize);
                let (source_refs, target_refs) = action
                    .as_ref()
                    .and_then(|action| actor.map(|actor| (actor, action)))
                    .map(|(actor, action)| self.action_refs(viewer, ctx, actor, action))
                    .unwrap_or_default();
                let (label, short_label, description) = action
                    .as_ref()
                    .map(|action| self.action_labels(action, actor))
                    .unwrap_or_else(|| {
                        (
                            format!("Action {action_id}"),
                            format!("#{action_id}"),
                            "Unknown action id in legal cache".to_string(),
                        )
                    });
                HumanLegalActionView {
                    index,
                    action_id,
                    family,
                    label,
                    short_label,
                    description,
                    params,
                    source_refs,
                    target_refs,
                    is_pass: matches!(action, Some(ActionDesc::Pass)),
                    is_attack: matches!(action, Some(ActionDesc::Attack { .. })),
                    is_play: matches!(
                        action,
                        Some(
                            ActionDesc::MainPlayCharacter { .. }
                                | ActionDesc::MainPlayEvent { .. }
                                | ActionDesc::ClimaxPlay { .. }
                                | ActionDesc::CounterPlay { .. }
                        )
                    ),
                    is_move: matches!(action, Some(ActionDesc::MainMove { .. })),
                }
            })
            .collect()
    }

    fn action_refs(
        &self,
        viewer: u8,
        ctx: VisibilityContext,
        actor: u8,
        action: &ActionDesc,
    ) -> (Vec<HumanActionRefView>, Vec<HumanActionRefView>) {
        match action {
            ActionDesc::MulliganSelect { hand_index }
            | ActionDesc::Clock { hand_index }
            | ActionDesc::MainPlayEvent { hand_index }
            | ActionDesc::ClimaxPlay { hand_index }
            | ActionDesc::CounterPlay { hand_index } => (
                vec![self.action_zone_ref(viewer, ctx, actor, Zone::Hand, *hand_index)],
                vec![],
            ),
            ActionDesc::MainPlayCharacter {
                hand_index,
                stage_slot,
            } => (
                vec![self.action_zone_ref(viewer, ctx, actor, Zone::Hand, *hand_index)],
                vec![self.action_stage_ref(viewer, actor, *stage_slot)],
            ),
            ActionDesc::MainMove { from_slot, to_slot } => (
                vec![self.action_stage_ref(viewer, actor, *from_slot)],
                vec![self.action_stage_ref(viewer, actor, *to_slot)],
            ),
            ActionDesc::MainActivateAbility { slot, .. }
            | ActionDesc::Attack { slot, .. }
            | ActionDesc::EncorePay { slot }
            | ActionDesc::EncoreDecline { slot } => (
                vec![self.action_stage_ref(viewer, actor, *slot)],
                self.attack_target_refs(viewer, actor, action),
            ),
            ActionDesc::LevelUp { index } => (
                vec![self.action_zone_ref(viewer, ctx, actor, Zone::Clock, *index)],
                vec![self.action_zone_target_ref(viewer, actor, Zone::Level, None)],
            ),
            ActionDesc::ChoiceSelect { index } => {
                let refs = self.choice_action_ref(viewer, ctx, actor, *index);
                (refs, vec![])
            }
            ActionDesc::TriggerOrder { index } => (
                vec![self.action_zone_target_ref(viewer, actor, Zone::Resolution, Some(*index))],
                vec![],
            ),
            ActionDesc::MulliganConfirm
            | ActionDesc::Pass
            | ActionDesc::ChoicePrevPage
            | ActionDesc::ChoiceNextPage
            | ActionDesc::Concede => (vec![], vec![]),
        }
    }

    fn action_zone_ref(
        &self,
        viewer: u8,
        ctx: VisibilityContext,
        owner: u8,
        zone: Zone,
        index: u8,
    ) -> HumanActionRefView {
        let hidden = self.zone_hidden_for_viewer(ctx, owner, zone);
        let zone_name = zone_name(zone);
        let card = if hidden {
            None
        } else {
            self.zone_card_id(owner, zone, index as usize)
                .map(|card_id| self.card_record(card_id))
        };
        HumanActionRefView {
            ref_id: if hidden {
                hidden_ref(viewer, owner, zone_name)
            } else {
                card_ref(viewer, owner, zone_name, index as u16)
            },
            zone: zone_name,
            owner_seat: owner,
            relative_owner: relative_owner(viewer, owner),
            visibility: zone_visibility_label(owner, viewer, zone, &self.curriculum, hidden),
            index: (!hidden).then_some(index as u16),
            slot: None,
            card,
        }
    }

    fn action_zone_target_ref(
        &self,
        viewer: u8,
        owner: u8,
        zone: Zone,
        index: Option<u8>,
    ) -> HumanActionRefView {
        let zone_name = zone_name(zone);
        HumanActionRefView {
            ref_id: index
                .map(|idx| card_ref(viewer, owner, zone_name, idx as u16))
                .unwrap_or_else(|| format!("{}.{}", relative_owner(viewer, owner), zone_name)),
            zone: zone_name,
            owner_seat: owner,
            relative_owner: relative_owner(viewer, owner),
            visibility: zone_visibility_label(owner, viewer, zone, &self.curriculum, false),
            index: index.map(u16::from),
            slot: None,
            card: None,
        }
    }

    fn action_stage_ref(&self, viewer: u8, owner: u8, slot: u8) -> HumanActionRefView {
        let card = self.state.players[owner as usize].stage[slot as usize]
            .card
            .map(|card| self.card_record(card.id));
        HumanActionRefView {
            ref_id: card_ref(viewer, owner, "stage", slot as u16),
            zone: "stage",
            owner_seat: owner,
            relative_owner: relative_owner(viewer, owner),
            visibility: "public",
            index: Some(slot as u16),
            slot: Some(slot),
            card,
        }
    }

    fn attack_target_refs(
        &self,
        viewer: u8,
        actor: u8,
        action: &ActionDesc,
    ) -> Vec<HumanActionRefView> {
        let ActionDesc::Attack { slot, attack_type } = action else {
            return Vec::new();
        };
        if *attack_type == AttackType::Direct {
            return Vec::new();
        }
        let opponent = 1 - actor;
        let slot_idx = *slot as usize;
        if slot_idx >= MAX_STAGE
            || self.state.players[opponent as usize].stage[slot_idx]
                .card
                .is_none()
        {
            return Vec::new();
        }
        vec![self.action_stage_ref(viewer, opponent, *slot)]
    }

    fn choice_action_ref(
        &self,
        viewer: u8,
        ctx: VisibilityContext,
        actor: u8,
        page_index: u8,
    ) -> Vec<HumanActionRefView> {
        let Some(choice) = self.state.turn.choice.as_ref() else {
            return vec![self.action_zone_target_ref(
                viewer,
                actor,
                Zone::Resolution,
                Some(page_index),
            )];
        };
        let global_idx = choice.page_start as usize + page_index as usize;
        let Some(option) = choice.options.get(global_idx) else {
            return Vec::new();
        };
        let sanitized =
            self.sanitize_choice_option_for_event(choice.reason, choice.player, ctx, option);
        vec![self.choice_option_ref(viewer, actor, choice.reason, &sanitized)]
    }

    fn choice_option_ref(
        &self,
        viewer: u8,
        actor: u8,
        reason: ChoiceReason,
        option: &ChoiceOptionRef,
    ) -> HumanActionRefView {
        let owner = self.choice_option_owner(reason, actor);
        let zone = choice_zone_name(option.zone);
        let hidden =
            option.card_id == 0 && option.index.is_none() && choice_zone_private(option.zone);
        HumanActionRefView {
            ref_id: match option.index {
                Some(index) => card_ref(viewer, owner, zone, index),
                None if option.zone == ChoiceZone::Stage => option
                    .target_slot
                    .map(|slot| card_ref(viewer, owner, "stage", slot as u16))
                    .unwrap_or_else(|| hidden_ref(viewer, owner, zone)),
                None => hidden_ref(viewer, owner, zone),
            },
            zone,
            owner_seat: owner,
            relative_owner: relative_owner(viewer, owner),
            visibility: choice_zone_visibility_label(
                owner,
                viewer,
                option.zone,
                &self.curriculum,
                hidden,
            ),
            index: option.index,
            slot: option.target_slot,
            card: (option.card_id != 0).then(|| self.card_record(option.card_id)),
        }
    }

    fn choice_option_owner(&self, reason: ChoiceReason, player: u8) -> u8 {
        if reason != ChoiceReason::TargetSelect {
            return player;
        }
        let Some(selection) = self.state.turn.target_selection.as_ref() else {
            return player;
        };
        match selection.spec.side {
            crate::state::TargetSide::SelfSide => selection.controller,
            crate::state::TargetSide::Opponent => 1 - selection.controller,
        }
    }

    fn action_labels(&self, action: &ActionDesc, actor: Option<u8>) -> (String, String, String) {
        match action {
            ActionDesc::MulliganConfirm => (
                "Confirm mulligan".to_string(),
                "Keep".to_string(),
                "Finish selecting cards for mulligan.".to_string(),
            ),
            ActionDesc::MulliganSelect { hand_index } => (
                format!("Toggle hand card {}", hand_index + 1),
                format!("Toggle {}", hand_index + 1),
                "Select or unselect this card for mulligan.".to_string(),
            ),
            ActionDesc::Pass => (
                "Pass".to_string(),
                "Pass".to_string(),
                "Take no optional action for this decision.".to_string(),
            ),
            ActionDesc::Clock { hand_index } => (
                format!("Clock hand card {}", hand_index + 1),
                format!("Clock {}", hand_index + 1),
                "Place this hand card into clock.".to_string(),
            ),
            ActionDesc::MainPlayCharacter {
                hand_index,
                stage_slot,
            } => (
                format!(
                    "Play hand card {} to {}",
                    hand_index + 1,
                    stage_slot_label(*stage_slot)
                ),
                format!("Play {}", hand_index + 1),
                "Play this character from hand to the selected stage slot.".to_string(),
            ),
            ActionDesc::MainPlayEvent { hand_index } => (
                format!("Play event from hand card {}", hand_index + 1),
                format!("Event {}", hand_index + 1),
                "Play this event card from hand.".to_string(),
            ),
            ActionDesc::MainMove { from_slot, to_slot } => (
                format!(
                    "Move {} to {}",
                    stage_slot_label(*from_slot),
                    stage_slot_label(*to_slot)
                ),
                "Move".to_string(),
                "Move a character between stage slots.".to_string(),
            ),
            ActionDesc::MainActivateAbility {
                slot,
                ability_index,
            } => (
                format!(
                    "Use ability {} from {}",
                    ability_index + 1,
                    stage_slot_label(*slot)
                ),
                format!("ACT {}", ability_index + 1),
                "Activate a stage character ability.".to_string(),
            ),
            ActionDesc::ClimaxPlay { hand_index } => (
                format!("Play climax from hand card {}", hand_index + 1),
                format!("Climax {}", hand_index + 1),
                "Play this climax from hand.".to_string(),
            ),
            ActionDesc::Attack { slot, attack_type } => (
                format!(
                    "{} attack with {}",
                    attack_type_label(*attack_type),
                    stage_slot_label(*slot)
                ),
                attack_type_short_label(*attack_type).to_string(),
                "Declare an attack with this center-stage character.".to_string(),
            ),
            ActionDesc::CounterPlay { hand_index } => (
                format!("Play counter from hand card {}", hand_index + 1),
                format!("Counter {}", hand_index + 1),
                "Play this counter card from hand.".to_string(),
            ),
            ActionDesc::LevelUp { index } => (
                format!("Level up with clock card {}", index + 1),
                format!("Level {}", index + 1),
                "Move this clock card to level.".to_string(),
            ),
            ActionDesc::EncorePay { slot } => (
                format!("Pay encore for {}", stage_slot_label(*slot)),
                "Encore".to_string(),
                "Pay stock to keep this character on stage.".to_string(),
            ),
            ActionDesc::EncoreDecline { slot } => (
                format!("Decline encore for {}", stage_slot_label(*slot)),
                "Decline".to_string(),
                "Do not pay encore for this character.".to_string(),
            ),
            ActionDesc::TriggerOrder { index } => (
                format!("Resolve trigger {}", index + 1),
                format!("Trigger {}", index + 1),
                "Choose this trigger to resolve next.".to_string(),
            ),
            ActionDesc::ChoiceSelect { index } => (
                format!("Choose option {}", index + 1),
                format!("Choice {}", index + 1),
                actor
                    .map(|_| "Select this option from the current choice page.".to_string())
                    .unwrap_or_else(|| "Select this choice option.".to_string()),
            ),
            ActionDesc::ChoicePrevPage => (
                "Previous choice page".to_string(),
                "Previous".to_string(),
                "Show the previous page of choice options.".to_string(),
            ),
            ActionDesc::ChoiceNextPage => (
                "Next choice page".to_string(),
                "Next".to_string(),
                "Show the next page of choice options.".to_string(),
            ),
            ActionDesc::Concede => (
                "Concede".to_string(),
                "Concede".to_string(),
                "Concede the game.".to_string(),
            ),
        }
    }

    fn card_record(&self, card_id: CardId) -> HumanCardRecord {
        if let Some(card) = self.db.get(card_id) {
            HumanCardRecord {
                card_id,
                card_type: Some(card_type_name(card.card_type)),
                color: Some(color_name(card.color)),
                level: Some(card.level),
                cost: Some(card.cost),
                power: Some(card.power),
                soul: Some(card.soul),
                triggers: card.triggers.iter().copied().map(trigger_name).collect(),
                traits: card.traits.clone(),
            }
        } else {
            HumanCardRecord {
                card_id,
                card_type: None,
                color: None,
                level: None,
                cost: None,
                power: None,
                soul: None,
                triggers: Vec::new(),
                traits: Vec::new(),
            }
        }
    }

    fn zone_card_id(&self, owner: u8, zone: Zone, index: usize) -> Option<CardId> {
        let player = &self.state.players[owner as usize];
        match zone {
            Zone::Deck => player.deck.get(index).map(|card| card.id),
            Zone::Hand => player.hand.get(index).map(|card| card.id),
            Zone::WaitingRoom => player.waiting_room.get(index).map(|card| card.id),
            Zone::Clock => player.clock.get(index).map(|card| card.id),
            Zone::Level => player.level.get(index).map(|card| card.id),
            Zone::Stock => player.stock.get(index).map(|card| card.id),
            Zone::Memory => player.memory.get(index).map(|card| card.id),
            Zone::Climax => player.climax.get(index).map(|card| card.id),
            Zone::Resolution => player.resolution.get(index).map(|card| card.id),
            Zone::Stage => player
                .stage
                .get(index)
                .and_then(|slot| slot.card.map(|card| card.id)),
        }
    }

    fn effective_slot_power(&self, player: usize, slot: usize, card_id: CardId) -> i32 {
        let slot_state = &self.state.players[player].stage[slot];
        let mut power =
            self.db.power_by_id(card_id) + slot_state.power_mod_turn + slot_state.power_mod_battle;
        for modifier in &self.state.modifiers {
            if modifier.kind != crate::state::ModifierKind::Power {
                continue;
            }
            if modifier.target_player as usize == player
                && modifier.target_slot as usize == slot
                && modifier.target_card == card_id
            {
                power = power.saturating_add(modifier.magnitude);
            }
        }
        power
    }

    fn effective_slot_soul(&self, player: usize, slot: usize, card_id: CardId) -> i32 {
        let mut soul = i32::from(self.db.soul_by_id(card_id));
        for modifier in &self.state.modifiers {
            if modifier.kind != crate::state::ModifierKind::Soul {
                continue;
            }
            if modifier.target_player as usize == player
                && modifier.target_slot as usize == slot
                && modifier.target_card == card_id
            {
                soul = soul.saturating_add(modifier.magnitude);
            }
        }
        soul.max(0)
    }

    fn legal_fingerprint64(&self, legal_action_ids: &[u16]) -> String {
        let mut bytes = Vec::with_capacity(24 + legal_action_ids.len() * 2);
        bytes.extend_from_slice(b"human-legal-v1");
        bytes.extend_from_slice(&self.decision_id().to_le_bytes());
        if let Some(decision) = self.decision.as_ref() {
            bytes.push(decision.player);
            bytes.push(decision_kind_code(decision.kind));
        } else {
            bytes.push(u8::MAX);
            bytes.push(u8::MAX);
        }
        for &action_id in legal_action_ids {
            bytes.extend_from_slice(&action_id.to_le_bytes());
        }
        format_hash64(hash_bytes(&bytes))
    }
}

fn action_params_map(
    desc: &crate::encode::ActionIdDesc,
) -> BTreeMap<String, HumanActionParamValue> {
    let mut params = BTreeMap::new();
    for param in &desc.params {
        let value = match &param.value {
            ActionParamValue::Int(value) => HumanActionParamValue::Int(*value),
            ActionParamValue::Str(value) => HumanActionParamValue::Str((*value).to_string()),
        };
        params.insert(param.name.to_string(), value);
    }
    params
}

fn strip_instance_ids_from_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(mut map) => {
            map.remove("instance_id");
            serde_json::Value::Object(
                map.into_iter()
                    .map(|(key, value)| (key, strip_instance_ids_from_value(value)))
                    .collect(),
            )
        }
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(strip_instance_ids_from_value)
                .collect(),
        ),
        other => other,
    }
}

fn format_hash64(value: u64) -> String {
    format!("{value:016x}")
}

fn relative_owner(viewer: u8, owner: u8) -> &'static str {
    if viewer == owner {
        "self"
    } else {
        "opponent"
    }
}

fn card_ref(viewer: u8, owner: u8, zone: &str, index: u16) -> String {
    format!("{}.{}.{}", relative_owner(viewer, owner), zone, index)
}

fn hidden_ref(viewer: u8, owner: u8, zone: &str) -> String {
    format!("{}.{}.*", relative_owner(viewer, owner), zone)
}

fn zone_visibility_label(
    owner: u8,
    viewer: u8,
    zone: Zone,
    curriculum: &crate::config::CurriculumConfig,
    hidden: bool,
) -> &'static str {
    if hidden {
        return "opponent_count_only";
    }
    match zone_identity_visibility(zone, curriculum) {
        ZoneIdentityVisibility::Public => "public",
        ZoneIdentityVisibility::OwnerOnly if owner == viewer => "self_private",
        ZoneIdentityVisibility::OwnerOnly => "opponent_count_only",
    }
}

fn choice_zone_visibility_label(
    owner: u8,
    viewer: u8,
    zone: ChoiceZone,
    curriculum: &crate::config::CurriculumConfig,
    hidden: bool,
) -> &'static str {
    if hidden {
        return "opponent_count_only";
    }
    let target_zone = match choice_zone_to_target_zone(zone) {
        Some(zone) => zone,
        None => return "public",
    };
    match target_zone_identity_visibility(target_zone, curriculum) {
        ZoneIdentityVisibility::Public => "public",
        ZoneIdentityVisibility::OwnerOnly if owner == viewer => "self_private",
        ZoneIdentityVisibility::OwnerOnly => "opponent_count_only",
    }
}

fn choice_zone_to_target_zone(zone: ChoiceZone) -> Option<crate::state::TargetZone> {
    match zone {
        ChoiceZone::WaitingRoom => Some(crate::state::TargetZone::WaitingRoom),
        ChoiceZone::Stage => Some(crate::state::TargetZone::Stage),
        ChoiceZone::Hand => Some(crate::state::TargetZone::Hand),
        ChoiceZone::DeckTop => Some(crate::state::TargetZone::DeckTop),
        ChoiceZone::Clock => Some(crate::state::TargetZone::Clock),
        ChoiceZone::Level => Some(crate::state::TargetZone::Level),
        ChoiceZone::Stock => Some(crate::state::TargetZone::Stock),
        ChoiceZone::Memory => Some(crate::state::TargetZone::Memory),
        ChoiceZone::Climax => Some(crate::state::TargetZone::Climax),
        ChoiceZone::Resolution => Some(crate::state::TargetZone::Resolution),
        ChoiceZone::Stack
        | ChoiceZone::PriorityCounter
        | ChoiceZone::PriorityAct
        | ChoiceZone::PriorityPass
        | ChoiceZone::Skip => None,
    }
}

fn choice_zone_private(zone: ChoiceZone) -> bool {
    matches!(
        zone,
        ChoiceZone::Hand | ChoiceZone::DeckTop | ChoiceZone::Stock | ChoiceZone::PriorityCounter
    )
}

fn phase_name(phase: crate::state::Phase) -> &'static str {
    match phase {
        crate::state::Phase::Mulligan => "mulligan",
        crate::state::Phase::Stand => "stand",
        crate::state::Phase::Draw => "draw",
        crate::state::Phase::Clock => "clock",
        crate::state::Phase::Main => "main",
        crate::state::Phase::Climax => "climax",
        crate::state::Phase::Attack => "attack",
        crate::state::Phase::End => "end",
    }
}

fn decision_kind_name(kind: DecisionKind) -> &'static str {
    match kind {
        DecisionKind::Mulligan => "mulligan",
        DecisionKind::Clock => "clock",
        DecisionKind::Main => "main",
        DecisionKind::Climax => "climax",
        DecisionKind::AttackDeclaration => "attack_declaration",
        DecisionKind::LevelUp => "level_up",
        DecisionKind::Encore => "encore",
        DecisionKind::TriggerOrder => "trigger_order",
        DecisionKind::Choice => "choice",
    }
}

fn decision_kind_code(kind: DecisionKind) -> u8 {
    match kind {
        DecisionKind::Mulligan => 0,
        DecisionKind::Clock => 1,
        DecisionKind::Main => 2,
        DecisionKind::Climax => 3,
        DecisionKind::AttackDeclaration => 4,
        DecisionKind::LevelUp => 5,
        DecisionKind::Encore => 6,
        DecisionKind::TriggerOrder => 7,
        DecisionKind::Choice => 8,
    }
}

fn terminal_name(terminal: crate::state::TerminalResult) -> String {
    match terminal {
        crate::state::TerminalResult::Win { winner } => format!("win_p{winner}"),
        crate::state::TerminalResult::Draw => "draw".to_string(),
        crate::state::TerminalResult::Timeout => "timeout".to_string(),
    }
}

fn zone_name(zone: Zone) -> &'static str {
    match zone {
        Zone::Deck => "deck",
        Zone::Hand => "hand",
        Zone::WaitingRoom => "waiting_room",
        Zone::Clock => "clock",
        Zone::Level => "level",
        Zone::Stock => "stock",
        Zone::Memory => "memory",
        Zone::Climax => "climax",
        Zone::Resolution => "resolution",
        Zone::Stage => "stage",
    }
}

fn choice_zone_name(zone: ChoiceZone) -> &'static str {
    match zone {
        ChoiceZone::WaitingRoom => "waiting_room",
        ChoiceZone::Stage => "stage",
        ChoiceZone::Hand => "hand",
        ChoiceZone::DeckTop => "deck_top",
        ChoiceZone::Clock => "clock",
        ChoiceZone::Level => "level",
        ChoiceZone::Stock => "stock",
        ChoiceZone::Memory => "memory",
        ChoiceZone::Climax => "climax",
        ChoiceZone::Resolution => "resolution",
        ChoiceZone::Stack => "stack",
        ChoiceZone::PriorityCounter => "priority_counter",
        ChoiceZone::PriorityAct => "priority_act",
        ChoiceZone::PriorityPass => "priority_pass",
        ChoiceZone::Skip => "skip",
    }
}

fn stage_row(slot: u8) -> &'static str {
    if slot < 3 {
        "center"
    } else {
        "back"
    }
}

fn stage_slot_label(slot: u8) -> &'static str {
    match slot {
        0 => "center left",
        1 => "center middle",
        2 => "center right",
        3 => "back left",
        4 => "back right",
        _ => "unknown slot",
    }
}

fn stage_status_name(status: StageStatus) -> &'static str {
    match status {
        StageStatus::Stand => "standing",
        StageStatus::Rest => "rested",
        StageStatus::Reverse => "reversed",
    }
}

fn card_type_name(card_type: CardType) -> &'static str {
    match card_type {
        CardType::Character => "character",
        CardType::Event => "event",
        CardType::Climax => "climax",
    }
}

fn color_name(color: CardColor) -> &'static str {
    match color {
        CardColor::Yellow => "yellow",
        CardColor::Green => "green",
        CardColor::Red => "red",
        CardColor::Blue => "blue",
        CardColor::Colorless => "colorless",
    }
}

fn trigger_name(icon: TriggerIcon) -> &'static str {
    match icon {
        TriggerIcon::Soul => "soul",
        TriggerIcon::Shot => "shot",
        TriggerIcon::Bounce => "bounce",
        TriggerIcon::Draw => "draw",
        TriggerIcon::Choice => "choice",
        TriggerIcon::Pool => "pool",
        TriggerIcon::Treasure => "treasure",
        TriggerIcon::Gate => "gate",
        TriggerIcon::Standby => "standby",
    }
}

fn attack_type_label(attack_type: AttackType) -> &'static str {
    match attack_type {
        AttackType::Frontal => "Frontal",
        AttackType::Side => "Side",
        AttackType::Direct => "Direct",
    }
}

fn attack_type_short_label(attack_type: AttackType) -> &'static str {
    match attack_type {
        AttackType::Frontal => "Frontal",
        AttackType::Side => "Side",
        AttackType::Direct => "Direct",
    }
}
