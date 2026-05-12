use anyhow::{Context, Result};

use super::ability::{
    ability_sort_key, compile_effects_from_def, compile_effects_from_template, AbilitySpec,
    AbilityTemplate,
};
use super::card::CardStatic;
use super::types::{CardColor, CardId, CardType};

/// Upper bound for dense per-id lookup caches.
///
/// The simulator stores several lookup vectors indexed directly by `CardId`.
/// Rejecting sparse, very large ids keeps attacker-supplied WSDB files from
/// forcing unbounded allocations while leaving substantial headroom for real
/// card catalogs.
pub const MAX_CARD_ID_INDEX: CardId = 1_000_000;

/// Loaded card database with cached per-id lookups and compiled abilities.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "CardDbRaw", into = "CardDbRaw")]
pub struct CardDb {
    /// Canonical list of static card definitions.
    pub cards: Vec<CardStatic>,
    #[serde(skip)]
    index: Vec<usize>,
    #[serde(skip)]
    valid_by_id: Vec<bool>,
    #[serde(skip)]
    power_by_id: Vec<i32>,
    #[serde(skip)]
    soul_by_id: Vec<u8>,
    #[serde(skip)]
    level_by_id: Vec<u8>,
    #[serde(skip)]
    cost_by_id: Vec<u8>,
    #[serde(skip)]
    color_by_id: Vec<CardColor>,
    #[serde(skip)]
    card_type_by_id: Vec<CardType>,
    #[serde(skip)]
    ability_specs: Vec<Vec<AbilitySpec>>,
    #[serde(skip)]
    compiled_ability_effects: Vec<Vec<Vec<crate::effects::EffectSpec>>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct CardDbRaw {
    cards: Vec<CardStatic>,
}

impl TryFrom<CardDbRaw> for CardDb {
    type Error = anyhow::Error;

    fn try_from(raw: CardDbRaw) -> Result<Self> {
        CardDb::new(raw.cards)
    }
}

impl From<CardDb> for CardDbRaw {
    fn from(db: CardDb) -> Self {
        Self { cards: db.cards }
    }
}

impl CardDb {
    /// Build a new database from raw card definitions.
    ///
    /// Validates ids, canonicalizes templates, and builds per-id caches.
    pub fn new(cards: Vec<CardStatic>) -> Result<Self> {
        let mut db = Self {
            cards,
            index: Vec::new(),
            valid_by_id: Vec::new(),
            power_by_id: Vec::new(),
            soul_by_id: Vec::new(),
            level_by_id: Vec::new(),
            cost_by_id: Vec::new(),
            color_by_id: Vec::new(),
            card_type_by_id: Vec::new(),
            ability_specs: Vec::new(),
            compiled_ability_effects: Vec::new(),
        };
        db.build_index()?;
        Ok(db)
    }

    /// Look up a card by id. Returns `None` for invalid or zero ids.
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

    #[inline(always)]
    /// Maximum card id present in the index.
    pub fn max_card_id(&self) -> CardId {
        self.index.len().saturating_sub(1).try_into().unwrap_or(0)
    }

    #[inline(always)]
    /// Whether an id exists in this database.
    pub fn is_valid_id(&self, id: CardId) -> bool {
        self.valid_by_id.get(id as usize).copied().unwrap_or(false)
    }

    #[inline(always)]
    /// Cached power value for a card id, or 0 if invalid.
    pub fn power_by_id(&self, id: CardId) -> i32 {
        self.power_by_id.get(id as usize).copied().unwrap_or(0)
    }

    #[inline(always)]
    /// Cached soul value for a card id, or 0 if invalid.
    pub fn soul_by_id(&self, id: CardId) -> u8 {
        self.soul_by_id.get(id as usize).copied().unwrap_or(0)
    }

    #[inline(always)]
    /// Cached level value for a card id, or 0 if invalid.
    pub fn level_by_id(&self, id: CardId) -> u8 {
        self.level_by_id.get(id as usize).copied().unwrap_or(0)
    }

    #[inline(always)]
    /// Cached cost value for a card id, or 0 if invalid.
    pub fn cost_by_id(&self, id: CardId) -> u8 {
        self.cost_by_id.get(id as usize).copied().unwrap_or(0)
    }

    #[inline(always)]
    /// Cached color value for a card id, defaulting to red.
    pub fn color_by_id(&self, id: CardId) -> CardColor {
        self.color_by_id
            .get(id as usize)
            .copied()
            .unwrap_or(CardColor::Red)
    }

    #[inline(always)]
    /// Cached card type for a card id, defaulting to character.
    pub fn card_type_by_id(&self, id: CardId) -> CardType {
        self.card_type_by_id
            .get(id as usize)
            .copied()
            .unwrap_or(CardType::Character)
    }

    pub(super) fn build_index(&mut self) -> Result<()> {
        let mut max_id: usize = 0;
        for card in &mut self.cards {
            if card.id == 0 {
                anyhow::bail!("CardId 0 is reserved for empty and cannot appear in the db");
            }
            if card.id > MAX_CARD_ID_INDEX {
                anyhow::bail!(
                    "CardId {} exceeds maximum supported id {}",
                    card.id,
                    MAX_CARD_ID_INDEX
                );
            }
            if card.counter_timing
                && !matches!(card.card_type, CardType::Event | CardType::Character)
            {
                eprintln!(
                    "CardId {} has counter timing but card_type {:?} is not eligible; disabling counter timing",
                    card.id, card.card_type
                );
                card.counter_timing = false;
            }
            for def in &card.ability_defs {
                def.validate()
                    .with_context(|| format!("CardId {} AbilityDef invalid", card.id))?;
            }
            max_id = max_id.max(card.id as usize);
        }
        let mut index = vec![usize::MAX; max_id + 1];
        let mut valid_by_id = vec![false; max_id + 1];
        let mut power_by_id = vec![0i32; max_id + 1];
        let mut soul_by_id = vec![0u8; max_id + 1];
        let mut level_by_id = vec![0u8; max_id + 1];
        let mut cost_by_id = vec![0u8; max_id + 1];
        let mut color_by_id = vec![CardColor::Red; max_id + 1];
        let mut card_type_by_id = vec![CardType::Character; max_id + 1];
        for (i, card) in self.cards.iter().enumerate() {
            let id = card.id as usize;
            if index[id] != usize::MAX {
                anyhow::bail!("Duplicate CardId {id}");
            }
            index[id] = i;
            valid_by_id[id] = true;
            power_by_id[id] = card.power;
            soul_by_id[id] = card.soul;
            level_by_id[id] = card.level;
            cost_by_id[id] = card.cost;
            color_by_id[id] = card.color;
            card_type_by_id[id] = card.card_type;
        }
        self.index = index;
        self.valid_by_id = valid_by_id;
        self.power_by_id = power_by_id;
        self.soul_by_id = soul_by_id;
        self.level_by_id = level_by_id;
        self.cost_by_id = cost_by_id;
        self.color_by_id = color_by_id;
        self.card_type_by_id = card_type_by_id;
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
                let idx = u8::try_from(ability_index).map_err(|_| {
                    anyhow::anyhow!(
                        "Ability index out of range for card {}: {}",
                        card.id,
                        ability_index
                    )
                })?;
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

    /// Abilities in canonical order for a given card id.
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

    /// Compiled effects for a specific ability on a card.
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

    /// Flattened list of all compiled effects for a card.
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
