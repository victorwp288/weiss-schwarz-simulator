use super::prelude::*;

pub(super) fn set_terminal_outcome(
    env: &mut GameEnv,
    controller: u8,
    outcome: TerminalOutcomeSpec,
) {
    env.state.terminal = Some(match outcome {
        TerminalOutcomeSpec::WinSelf => TerminalResult::Win { winner: controller },
        TerminalOutcomeSpec::WinOpponent => TerminalResult::Win {
            winner: 1 - controller,
        },
        TerminalOutcomeSpec::Draw => TerminalResult::Draw,
        TerminalOutcomeSpec::Timeout => TerminalResult::Timeout,
    });
}

pub(super) fn apply_rule_override(env: &mut GameEnv, kind: RuleOverrideKind) {
    if !env.state.turn.rule_overrides.contains(&kind) {
        env.state.turn.rule_overrides.push(kind);
    }
}
