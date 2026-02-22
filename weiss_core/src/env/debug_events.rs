use crate::replay::ReplayEvent;

use super::GameEnv;

/// Fixed-capacity ring storing recent replay events for debug inspection.
pub(crate) struct EventRing {
    capacity: usize,
    events: Vec<ReplayEvent>,
    next: usize,
    full: bool,
}

impl EventRing {
    /// Create a ring with `capacity` events.
    pub(crate) fn new(capacity: usize) -> Self {
        let mut events = Vec::with_capacity(capacity);
        events.reserve(capacity);
        Self {
            capacity,
            events,
            next: 0,
            full: false,
        }
    }

    /// Clear all buffered events.
    pub(crate) fn clear(&mut self) {
        self.events.clear();
        self.next = 0;
        self.full = false;
    }

    /// Push one event, evicting the oldest when full.
    pub(crate) fn push(&mut self, event: ReplayEvent) {
        if self.capacity == 0 {
            return;
        }
        if self.events.len() < self.capacity {
            self.events.push(event);
            if self.events.len() == self.capacity {
                self.full = true;
                self.next = 0;
            }
        } else {
            self.events[self.next] = event;
            self.next = (self.next + 1) % self.capacity;
        }
    }

    /// Number of currently stored events.
    pub(crate) fn len(&self) -> usize {
        if self.capacity == 0 {
            0
        } else if self.full {
            self.capacity
        } else {
            self.events.len()
        }
    }

    /// Copy a chronological snapshot as numeric event codes into `out`.
    ///
    /// Returns the number of written entries (remaining slots are zero-filled).
    pub(crate) fn snapshot_codes<F: Fn(&ReplayEvent) -> u32>(
        &self,
        out: &mut [u32],
        code_fn: F,
    ) -> usize {
        let len = self.len();
        if len == 0 {
            for slot in out.iter_mut() {
                *slot = 0;
            }
            return 0;
        }
        let cap = self.capacity;
        for (i, slot) in out.iter_mut().enumerate() {
            if i >= len {
                *slot = 0;
                continue;
            }
            let idx = if self.full { (self.next + i) % cap } else { i };
            *slot = code_fn(&self.events[idx]);
        }
        len
    }

    /// Clone a chronological snapshot of stored events.
    pub(crate) fn snapshot_events(&self) -> Vec<ReplayEvent> {
        let len = self.len();
        if len == 0 {
            return Vec::new();
        }
        let cap = self.capacity;
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            let idx = if self.full { (self.next + i) % cap } else { i };
            out.push(self.events[idx].clone());
        }
        out
    }
}

/// Stable numeric code for each canonical replay event variant.
pub(crate) fn event_code(event: &ReplayEvent) -> u32 {
    use crate::events::Event;
    match event {
        Event::Draw { .. } => 1,
        Event::Damage { .. } => 2,
        Event::DamageCancel { .. } => 3,
        Event::DamageIntent { .. } => 4,
        Event::DamageModifierApplied { .. } => 5,
        Event::DamageModified { .. } => 6,
        Event::DamageCommitted { .. } => 7,
        Event::ReversalCommitted { .. } => 8,
        Event::Reveal { .. } => 9,
        Event::TriggerQueued { .. } => 10,
        Event::TriggerGrouped { .. } => 11,
        Event::TriggerResolved { .. } => 12,
        Event::TriggerCanceled { .. } => 13,
        Event::TimingWindowEntered { .. } => 14,
        Event::PriorityGranted { .. } => 15,
        Event::PriorityPassed { .. } => 16,
        Event::StackGroupPresented { .. } => 17,
        Event::StackOrderChosen { .. } => 18,
        Event::StackPushed { .. } => 19,
        Event::StackResolved { .. } => 20,
        Event::AutoResolveCapExceeded { .. } => 21,
        Event::WindowAdvanced { .. } => 22,
        Event::ChoicePresented { .. } => 23,
        Event::ChoicePageChanged { .. } => 24,
        Event::ChoiceMade { .. } => 25,
        Event::ChoiceAutopicked { .. } => 26,
        Event::ChoiceSkipped { .. } => 27,
        Event::ZoneMove { .. } => 28,
        Event::ControlChanged { .. } => 29,
        Event::ModifierAdded { .. } => 30,
        Event::ModifierRemoved { .. } => 31,
        Event::Concede { .. } => 32,
        Event::Play { .. } => 33,
        Event::PlayEvent { .. } => 34,
        Event::PlayClimax { .. } => 35,
        Event::Trigger { .. } => 36,
        Event::Attack { .. } => 37,
        Event::AttackType { .. } => 38,
        Event::Counter { .. } => 39,
        Event::Clock { .. } => 40,
        Event::Shuffle { .. } => 41,
        Event::Refresh { .. } => 42,
        Event::RefreshPenalty { .. } => 43,
        Event::LevelUpChoice { .. } => 44,
        Event::Encore { .. } => 45,
        Event::Stand { .. } => 46,
        Event::EndTurn { .. } => 47,
        Event::Terminal { .. } => 48,
    }
}

impl GameEnv {
    /// Snapshot debug event ring as compact numeric codes.
    pub fn debug_event_ring_codes(&self, viewer: u8, out: &mut [u32]) -> u16 {
        let Some(rings) = self.debug_event_ring.as_ref() else {
            for slot in out.iter_mut() {
                *slot = 0;
            }
            return 0;
        };
        let ring = &rings[viewer as usize % 2];
        let count = ring.snapshot_codes(out, event_code);
        count as u16
    }

    /// Snapshot debug event ring as full event records.
    pub fn debug_event_ring_snapshot(&self, viewer: u8) -> Vec<ReplayEvent> {
        let Some(rings) = self.debug_event_ring.as_ref() else {
            return Vec::new();
        };
        rings[viewer as usize % 2].snapshot_events()
    }
}
