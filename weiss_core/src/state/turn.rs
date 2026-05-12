use serde::{Deserialize, Serialize};

/// Turn phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Mulligan step before the first turn begins.
    Mulligan,
    /// Stand phase: stand rested characters.
    Stand,
    /// Draw phase: draw a card.
    Draw,
    /// Clock phase: optionally place a card into clock.
    Clock,
    /// Main phase: play cards and use main-phase abilities.
    Main,
    /// Climax phase: optionally place a climax.
    Climax,
    /// Attack phase.
    Attack,
    /// End phase cleanup.
    End,
}

/// Timing window for triggered effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimingWindow {
    /// Main phase timing window.
    MainWindow,
    /// Climax phase timing window.
    ClimaxWindow,
    /// After an attack is declared.
    AttackDeclarationWindow,
    /// During trigger reveal/resolution.
    TriggerResolutionWindow,
    /// During counter timing.
    CounterWindow,
    /// During damage resolution.
    DamageResolutionWindow,
    /// During encore timing.
    EncoreWindow,
    /// During end phase timing.
    EndPhaseWindow,
}
