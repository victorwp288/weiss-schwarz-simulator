import re
from typing import List

try:
    from scraper.ability_common import COUNT_TOKEN_RE, ParseRule, RULE_MODE_APPROX, RULE_MODE_EXACT
except ModuleNotFoundError:
    from ability_common import COUNT_TOKEN_RE, ParseRule, RULE_MODE_APPROX, RULE_MODE_EXACT

CONT_RULES: List[ParseRule] = [
    ParseRule(
        id="Continuous.ConditionalPower.PerOtherTraitCount",
        pattern=re.compile(
            r"^This card gets \+(\d+) power for each of your other 《([^》]+)》 characters\.?$",
            re.I,
        ),
        mode=RULE_MODE_EXACT,
        risk_class="low",
    ),
    ParseRule(
        id="Continuous.ConditionalPower.MiddleCenter.Self",
        pattern=re.compile(
            r"^If this card is in the middle position of your center stage, this card gets \+(\d+) power\.?$",
            re.I,
        ),
        mode=RULE_MODE_EXACT,
        risk_class="low",
    ),
    ParseRule(
        id="Continuous.ConditionalPower.IfHasOtherTrait",
        pattern=re.compile(
            rf"^If you have ({COUNT_TOKEN_RE}) or more other 《([^》]+)》 characters, this card gets \+(\d+) power\.?$",
            re.I,
        ),
        mode=RULE_MODE_EXACT,
        risk_class="low",
    ),
    ParseRule(
        id="Continuous.ConditionalSoul.MiddleCenter.Self",
        pattern=re.compile(
            r"^If this card is in the middle position of your center stage, this card gets \+(\d+) soul\.?$",
            re.I,
        ),
        mode=RULE_MODE_EXACT,
        risk_class="low",
    ),
]

AUTO_RULES: List[ParseRule] = [
    ParseRule(
        id="Auto.TeamPowerOnClimaxPlaced",
        pattern=re.compile(
            rf"^When (?:your|a) climax is placed on your climax area, choose (?:up to )?({COUNT_TOKEN_RE}) of your characters, and that character gets \+(\d+) power until end of turn\.?$",
            re.I,
        ),
        mode=RULE_MODE_EXACT,
        risk_class="low",
    ),
    ParseRule(
        id="Auto.TeamPowerSoulOnClimaxPlaced",
        pattern=re.compile(
            rf"^When (?:your|a) climax is placed on your climax area, choose (?:up to )?({COUNT_TOKEN_RE}) of your characters, and that character gets \+(\d+) power and \+(\d+) soul until end of turn\.?$",
            re.I,
        ),
        mode=RULE_MODE_EXACT,
        risk_class="low",
    ),
    ParseRule(
        id="Auto.TeamPowerOnClimaxPlaced.OpponentNextTurn",
        pattern=re.compile(
            rf"^When (?:your|a) climax is placed on your climax area, choose (?:up to )?({COUNT_TOKEN_RE}) of your characters, and that character gets \+(\d+) power until the end of your opponent's next turn\.?$",
            re.I,
        ),
        mode=RULE_MODE_EXACT,
        risk_class="low",
    ),
    ParseRule(
        id="Auto.SelfBottomDeck.OnReverse",
        pattern=re.compile(
            r"^put this card (?:at|on) the bottom of your deck\.?$",
            re.I,
        ),
        mode=RULE_MODE_EXACT,
        risk_class="low",
    ),
]

ACT_RULES: List[ParseRule] = [
    ParseRule(
        id="Activated.Brainstorm.CustomAction.ApproxDraw",
        pattern=re.compile(
            rf"^Brainstorm\s+(?:Flip over|Reveal)\s+({COUNT_TOKEN_RE})\s+cards?\s+from the top of your deck, and put them into your waiting room\.\s+For each climax revealed(?: among those cards)?,\s+perform the following action\.\s+\".+\"\.?$",
            re.I,
        ),
        mode=RULE_MODE_APPROX,
        risk_class="medium",
    )
]
