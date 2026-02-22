import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scraper.convert import (  # noqa: E402
    APPROX_PROFILE_APPROX,
    APPROX_PROFILE_STRICT,
    AbilityParseStats,
    map_triggers,
    normalize_ability_line,
    normalize_approx_profile,
    parse_abilities,
    parse_cost as parse_cost_v1,
)
from scraper.parser_v2.cost import parse_cost as parse_cost_v2  # noqa: E402
from scraper.parser_v2.engine import parse_line as parse_line_v2  # noqa: E402


class ConvertParsingTests(unittest.TestCase):
    def assert_single_approx_def(self, defs):
        self.assertEqual(len(defs), 1)
        self.assertTrue(defs[0].get("conditions", {}).get("requires_approx_effects") is True)

    def test_approx_profile_normalize_to_canonical_names(self):
        self.assertEqual(normalize_approx_profile("strict"), APPROX_PROFILE_STRICT)
        self.assertEqual(normalize_approx_profile("approx"), APPROX_PROFILE_APPROX)
        with self.assertRaises(ValueError):
            normalize_approx_profile("none")

    def test_continuous_power_and_soul_all_characters(self):
        stats = AbilityParseStats()
        text = "【CONT】 All of your characters get +500 power and +1 soul."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {"AddPower": {"amount": 500, "duration_turn": False}},
                {"AddSoul": {"amount": 1, "duration_turn": False}},
            ],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage", "SelfStage"])

    def test_continuous_soul_all_characters(self):
        stats = AbilityParseStats()
        text = "【CONT】 All of your characters get +1 soul."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddSoul": {"amount": 1, "duration_turn": False}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])

    def test_continuous_assist_front_row_power(self):
        stats = AbilityParseStats()
        text = "【CONT】 Assist All of your characters in front of this card get +500 power."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 500, "duration_turn": False}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfFrontRow"])

    def test_continuous_assist_front_row_power_by_level(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 Assist All of your characters in front of this card get +X power. "
            "X is equal to that character's level ×500."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPowerByLevel": {"multiplier": 500, "duration_turn": False}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfFrontRow"])

    def test_continuous_assist_front_row_level_min(self):
        stats = AbilityParseStats()
        text = "【CONT】 Assist All of your level 1 or higher characters in front of this card get +500 power."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "AddPowerIfTargetLevelAtLeast": {
                        "amount": 500,
                        "min_level": 1,
                        "duration_turn": False,
                    }
                }
            ],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfFrontRow"])

    def test_continuous_cannot_side_attack(self):
        stats = AbilityParseStats()
        text = "【CONT】 This card cannot side attack."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"CannotSideAttack": {"duration_turn": False}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["This"])

    def test_continuous_cannot_be_chosen_by_opponent_effects(self):
        stats = AbilityParseStats()
        text = "【CONT】 This card cannot be chosen by your opponent's effects."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"CannotBeChosenByOpponentEffects": {"duration_turn": False}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["This"])

    def test_continuous_cannot_become_reverse_if_facing_cost_max(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If the character facing this card is cost 0 or lower, "
            "this card cannot become 【REVERSE】."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "SelfCannotBecomeReverseIfFacingOpponent": {
                        "max_level": None,
                        "max_cost": 0,
                        "level_gt_source_level": False,
                    }
                }
            ],
        )

    def test_continuous_cannot_frontal_attack_if_facing_higher_level(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If the character facing this card is a higher level than this card, "
            "this card cannot frontal attack."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            ["SelfCannotFrontalAttackIfFacingOpponentHigherLevel"],
        )

    def test_continuous_facing_opponent_cannot_move_stage_position(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 The character facing this card cannot move to another position of the stage."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            ["FacingOpponentCannotMoveStagePosition"],
        )

    def test_continuous_all_characters_following_ability_flattened(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 All of your characters get the following ability. "
            '"【CONT】 This card cannot side attack."'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"CannotSideAttack": {"duration_turn": False}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])

    def test_continuous_all_opponent_characters_gain_encore_stock_cost(self):
        stats = AbilityParseStats()
        text = '【CONT】 All of your opponent\'s characters get "【AUTO】 Encore [(2)]".'
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"EncoreStockCost": {"cost": 2, "duration_turn": False}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["OppStage"])

    def test_on_play_following_ability_cannot_move_stage_position(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, choose 1 of your opponent's characters, "
            "and that character gets the following ability until the end of your opponent's next turn. "
            '"【CONT】 This card cannot move to another position of the stage."'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(list(ability_defs[0]["effects"][0].keys()), ["GrantAbilityDef"])
        granted = ability_defs[0]["effects"][0]["GrantAbilityDef"]
        self.assertEqual(granted["duration"], "UntilEndOfOpponentsNextTurn")
        self.assertEqual(
            granted["ability"]["effects"],
            [{"CannotMoveStagePosition": {"duration_turn": True}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["OppStage"])

    def test_use_this_card_following_ability_battle_opponent_memory(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 When you use this card\'s "Backup", choose 1 of your characters in battle, '
            "and that character gets the following ability until end of turn. "
            "\"【AUTO】 When this card's battle opponent becomes 【REVERSE】, put that character into your opponent's memory.\""
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "UseAct")
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])
        self.assertEqual(ability_defs[0]["target_limit"], 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"BattleOpponentMoveToMemoryOnReverse": {"duration_turn": True}}],
        )

    def test_on_play_may_heal_top_clock(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, "
            "you may put the top card of your clock into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["Heal"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfClock"])
        self.assertEqual(ability_defs[0]["target_limit"], 1)
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_on_play_self_power_until_end_of_turn(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, "
            "this card gets +1500 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 1500, "duration_turn": True}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["This"])

    def test_on_play_look_top_and_may_mill(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, "
            "look at the top card of your deck, and put it on the top of your deck or into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToWaitingRoom"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True])
        self.assertEqual(ability_defs[0]["target_limit"], 1)

    def test_on_play_look_top_and_may_bottom(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, "
            "look at the top card of your deck, and put it on the top or at the bottom of your deck."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToDeckBottom"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True])
        self.assertEqual(ability_defs[0]["target_limit"], 1)

    def test_trigger_check_climax_team_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When your character's trigger check reveals a climax, "
            "choose 1 of your characters, and that character gets +1000 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "TriggerResolution")
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"trigger_check_revealed_climax": True},
        )
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 1000, "duration_turn": True}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])
        self.assertEqual(ability_defs[0]["target_limit"], 1)

    def test_attack_look_top_two_keep_one(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card attacks, look at up to 2 cards from the top of your deck, "
            "choose 1 card from among them, put it on the top of your deck, "
            "and put the rest into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(ability_defs[0]["effects"], ["MoveToWaitingRoom"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True])
        self.assertEqual(ability_defs[0]["target_limit"], 2)

    def test_paid_encore_rest_on_center_stage_wording(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1)] At the beginning of the encore step, if you do not have another "
            "【REST】 character on your center stage, you may pay the cost. If you do, 【REST】 this card."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BeginEncoreStep")
        self.assertEqual(ability_defs[0]["effects"], ["RestThisIfNoOtherRestCenter"])

    def test_on_reverse_reverse_battle_opponent_level(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card becomes 【REVERSE】, if this card's battle opponent is level 1 or lower, "
            "you may 【REVERSE】 that character."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "BattleOpponentReverseIf": {
                        "max_level": 1,
                        "max_cost": None,
                        "level_gt_opponent_level": False,
                    }
                }
            ],
        )
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")

    def test_on_reverse_bottom_deck_battle_opponent(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card becomes 【REVERSE】 in battle, if this card's battle opponent is cost 0 or lower, "
            "you may put that character at the bottom of your opponent's deck."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "BattleOpponentMoveToDeckBottomIf": {
                        "max_level": None,
                        "max_cost": 0,
                        "level_gt_opponent_level": False,
                    }
                }
            ],
        )
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")

    def test_on_reverse_stock_swap_battle_opponent(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card becomes 【REVERSE】, if this card's battle opponent is level 0 or lower, "
            "you may put that character into your opponent's stock. If you do, put the bottom card of your "
            "opponent's stock into their waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "BattleOpponentMoveToStockThenBottomStockToWaitingRoomIf": {
                        "max_level": 0,
                        "max_cost": None,
                        "level_gt_opponent_level": False,
                    }
                }
            ],
        )
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")

    def test_on_reverse_clock_swap_battle_opponent(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card becomes 【REVERSE】, if the level of this card's battle opponent is "
            "higher than your opponent's level, you may put the top card of your opponent's clock into "
            "their waiting room. If you do, put that character into your opponent's clock."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "BattleOpponentMoveToClockAfterClockTopToWaitingRoomIf": {
                        "max_level": None,
                        "max_cost": None,
                        "level_gt_opponent_level": True,
                    }
                }
            ],
        )
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")

    def test_on_reverse_move_self_to_memory(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card becomes 【REVERSE】 in battle, put this card into your memory."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")
        self.assertEqual(ability_defs[0]["effects"], ["MoveToMemory"])
        self.assertEqual(ability_defs[0]["targets"], ["This"])

    def test_on_reverse_move_self_to_memory_with_memory_gate(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card becomes 【REVERSE】 in battle, if your memory has 2 or less cards, "
            "you may put this card into your memory."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")
        self.assertEqual(ability_defs[0]["conditions"]["self_memory_at_most"], 2)
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_battle_opponent_reverse_stock_if_climax(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card's battle opponent becomes 【REVERSE】, if there is a climax in your "
            "climax area, you may put the top card of your deck into your stock."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BattleOpponentReverse")
        self.assertEqual(ability_defs[0]["effects"], [{"StockCharge": {"count": 1}}])
        self.assertEqual(
            ability_defs[0]["conditions"]["climax_area"],
            {"side": "SelfSide", "card_ids": []},
        )
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_battle_opponent_reverse_move_to_memory(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card's battle opponent becomes 【REVERSE】, you may put that character "
            "into your opponent's memory."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BattleOpponentReverse")
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "BattleOpponentMoveToMemoryIf": {
                        "max_level": None,
                        "max_cost": None,
                        "level_gt_opponent_level": False,
                    }
                }
            ],
        )
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_on_climax_play_perform_standby(self):
        stats = AbilityParseStats()
        text = "【AUTO】 When this card is placed on your climax area from your hand, perform the [STANDBY] effect."
        abilities, ability_defs, _ = parse_abilities(text, "Climax", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"TriggerIcon": {"icon": "Standby"}}],
        )
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")

    def test_on_climax_play_draw_power_soul(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on your climax area from your hand, "
            "draw a card, choose one of your characters, and that character gets +1000 power and +1 soul until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Climax", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {"Draw": {"count": 1}},
                {"AddPower": {"amount": 1000, "duration_turn": True}},
                {"AddSoul": {"amount": 1, "duration_turn": True}},
            ],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage", "SelfStage"])

    def test_choice_and_pool_triggers_are_kept(self):
        stats = {"triggers_dropped": {}, "triggers_coerced": {}}
        mapped = map_triggers(["Choice", "Pool"], stats)
        self.assertEqual(mapped, ["Choice", "Pool"])
        self.assertEqual(stats["triggers_dropped"], {})

    def test_act_brainstorm_draw_maps_to_ability_def(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 Brainstorm [(1) 【REST】 this card] Flip over 4 cards from the top of your "
            "deck, and put them into your waiting room. For each climax revealed among those "
            "cards, draw up to 1 card."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"Brainstorm": {"reveal_count": 4, "per_climax": 1, "mode": "Draw"}}],
        )
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)
        self.assertTrue(ability_defs[0]["cost"]["rest_self"])

    def test_act_brainstorm_salvage_maps_to_ability_def(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 Brainstorm [(1) 【REST】 this card] Reveal four cards from the top of your "
            "deck, and put them into your waiting room. For each climax revealed, choose up to "
            "one character in your waiting room, and return it to your hand."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "Brainstorm": {
                        "reveal_count": 4,
                        "per_climax": 1,
                        "mode": "SalvageCharacter",
                    }
                }
            ],
        )

    def test_auto_encore_variant_character_discard(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 Encore [Put a character from your hand into your waiting room] "
            "(When this card is put into your waiting room from the stage, you may pay the cost. "
            "If you do, return this card to its previous stage position as 【REST】)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(ability_defs, [])
        self.assertEqual(len(abilities), 1)
        self.assertEqual(
            abilities[0],
            {
                "EncoreVariant": {
                    "cost": {
                        "stock": 0,
                        "rest_self": False,
                        "rest_other": 0,
                        "discard_from_hand": 1,
                        "clock_from_hand": 0,
                        "clock_from_deck_top": 0,
                        "reveal_from_hand": 0,
                        "cost_steps": [{"DiscardFromHand": {"count": 1}}],
                    }
                }
            },
        )

    def test_auto_encore_variant_clock_top_fullwidth(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 Encore [Put the top card of your deck into your clock] "
            "（When this card is put into your waiting room from the stage, you may pay the cost. "
            "If you do, return this card to its previous stage position as 【REST】)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(ability_defs, [])
        self.assertEqual(len(abilities), 1)
        self.assertEqual(
            abilities[0]["EncoreVariant"]["cost"]["clock_from_deck_top"],
            1,
        )

    def test_auto_bond_maps_to_template(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 Bond/"Target Name" [Put a card from your hand into your waiting room] '
            "(When this card is played and placed on stage, you may pay the cost. If you do, "
            'choose a card named "Target Name" in your waiting room, and return it to your hand.)'
        )
        abilities, ability_defs, _ = parse_abilities(
            text, "Character", stats, {"Target Name": [101, 102]}
        )
        self.assertEqual(ability_defs, [])
        self.assertEqual(len(abilities), 1)
        self.assertEqual(
            abilities[0],
            {
                "Bond": {
                    "cost": {
                        "stock": 0,
                        "rest_self": False,
                        "rest_other": 0,
                        "discard_from_hand": 1,
                        "clock_from_hand": 0,
                        "clock_from_deck_top": 0,
                        "reveal_from_hand": 0,
                        "cost_steps": [{"DiscardFromHand": {"count": 1}}],
                    },
                    "count": 1,
                    "target_ids": [101, 102],
                }
            },
        )

    def test_auto_bond_collects_multiple_named_targets(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 Bond/"Alpha" "Beta" [Put 1 card from your hand into your waiting room] '
            "(When this card is played and placed on the stage, you may pay the cost. If you do, "
            'choose 1 "Alpha" or "Beta" in your waiting room, and return it to your hand)'
        )
        abilities, ability_defs, _ = parse_abilities(
            text, "Character", stats, {"Alpha": [11], "Beta": [22, 23]}
        )
        self.assertEqual(ability_defs, [])
        self.assertEqual(len(abilities), 1)
        self.assertEqual(abilities[0]["Bond"]["target_ids"], [11, 22, 23])
        self.assertEqual(abilities[0]["Bond"]["count"], 1)

    def test_continuous_during_turn_conditional_power(self):
        stats = AbilityParseStats()
        text = "【CONT】 During your turn, this card gets +2000 power."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertEqual(effect["amount"], 2000)
        self.assertEqual(effect["turn"], "SelfTurn")
        self.assertFalse(effect["exclude_source"])
        self.assertEqual(ability_defs[0]["targets"], ["This"])

    def test_continuous_all_other_trait_conditional_power(self):
        stats = AbilityParseStats()
        text = "【CONT】 All of your other 《Music》 characters get +500 power."
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids=None,
            trait_map={"Music": 7},
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertTrue(effect["exclude_source"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])
        self.assertEqual(ability_defs[0]["target_trait"], 7)

    def test_continuous_stock_threshold_conditional_power(self):
        stats = AbilityParseStats()
        text = "【CONT】 If your stock has 2 or less cards, this card gets +1000 power."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertEqual(
            effect["zone_count"],
            {"side": "SelfSide", "zone": "Stock", "cmp": "AtMost", "value": 2},
        )

    def test_continuous_marker_based_conditional_power(self):
        stats = AbilityParseStats()
        text = "【CONT】 If there is a marker underneath this card, this card gets +1500 power."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertTrue(effect["require_source_marker"])
        self.assertFalse(effect["per_source_marker"])

    def test_continuous_power_per_marker(self):
        stats = AbilityParseStats()
        text = "【CONT】 This card gets +500 power for each marker underneath this card."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertTrue(effect["per_source_marker"])
        self.assertFalse(effect["require_source_marker"])

    def test_auto_on_play_move_waiting_room_to_marker(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, you may choose 1 "
            '"Target Name" in your waiting room, and put it face up underneath this card as a marker.'
        )
        abilities, ability_defs, _ = parse_abilities(
            text, "Character", stats, {"Target Name": [44, 45]}
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"MoveToMarker": {"target_ids": [44, 45]}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfWaitingRoom"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_auto_paid_attack_sets_trigger_check_count(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1)] When this card attacks, you may pay the cost. If you do, during "
            "that attack, perform a trigger check 2 times on the trigger step."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"SetTriggerCheckCount": {"count": 2}}],
        )
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)

    def test_auto_begin_opponent_attack_phase_move_open_center(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 At the beginning of your opponent's attack phase, you may move this card "
            "to an open position of your center stage with a character facing this card."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BeginAttackPhase")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"MoveThisToOpenCenter": {"require_facing": True}}],
        )
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_auto_begin_opponent_draw_phase_reveal_gate_sets_turn_condition(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 At the beginning of your opponent's draw phase, reveal the top card of your "
            "deck. If that card is level 1 or higher, you may return this card to your hand. "
            "(Climax are regarded as level 0. Return the revealed card to its original place)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BeginDrawPhase")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"RevealTopIfLevelAtLeastMoveThisToHand": {"min_level": 1}}],
        )
        self.assertEqual(ability_defs[0]["conditions"]["turn"], "OpponentTurn")
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_auto_on_reverse_reveal_gate_rest_self(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card becomes 【REVERSE】, reveal the top card of your deck. If that "
            "card is level 1 or higher, you may 【REST】 this card. (Climax are regarded as level "
            "0. Return the revealed card to its original place)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"RevealTopIfLevelAtLeastRestThis": {"min_level": 1}}],
        )
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_auto_on_play_reveal_gate_move_top_to_stock(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, reveal the top card of "
            "your deck. If that card is level 1 or higher, put it into your stock. (Otherwise, "
            "return it to its original place. Climax are regarded as level 0)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"RevealTopIfLevelAtLeastMoveTopToStock": {"min_level": 1}}],
        )

    def test_act_choose_up_to_opponent_characters_negative_power(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 [Put 1 card from your hand into your waiting room] Choose up to 2 of your "
            "opponent's characters, and that character gets -1000 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], None)
        self.assertEqual(ability_defs[0]["cost"]["discard_from_hand"], 1)
        self.assertEqual(ability_defs[0]["targets"], ["OppStage"])
        self.assertEqual(ability_defs[0]["target_limit"], 2)
        self.assertEqual(ability_defs[0]["effect_optional"], [True])
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": -1000, "duration_turn": True}}],
        )

    def test_auto_paid_begin_encore_step_rest_self(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1)] At the beginning of the encore step, if you do not have another "
            "【REST】 character in your center stage, you may pay the cost. If you do, "
            "【REST】 this card."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BeginEncoreStep")
        self.assertEqual(ability_defs[0]["effects"], ["RestThisIfNoOtherRestCenter"])
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)

    def test_auto_on_use_act_choose_character_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 This ability activates up to one time per turn. When you use an 【ACT】, "
            "choose one of your characters, and that character gets +1000 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "UseAct")
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])
        self.assertEqual(ability_defs[0]["target_limit"], 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 1000, "duration_turn": True}}],
        )

    def test_auto_on_use_act_self_power(self):
        stats = AbilityParseStats()
        text = "【AUTO】 When you use an 【ACT】, this card gets +500 power until end of turn."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "UseAct")
        self.assertEqual(ability_defs[0]["targets"], ["This"])
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 500, "duration_turn": True}}],
        )

    def test_auto_attack_facing_level_at_least_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card attacks, if the character facing this card is level 2 or "
            "higher, this card gets +4000 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "AddPowerIfBattleOpponentLevelAtLeast": {
                        "amount": 4000,
                        "min_level": 2,
                        "duration_turn": True,
                    }
                }
            ],
        )

    def test_auto_attack_facing_level_exact_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card attacks, if the character facing this card is level 0, this "
            "card gets +1000 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "AddPowerIfBattleOpponentLevelExact": {
                        "amount": 1000,
                        "level": 0,
                        "duration_turn": True,
                    }
                }
            ],
        )

    def test_continuous_power_per_opponent_back_stage_character(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 This card gets +500 power for each character in your opponent's back stage."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertEqual(effect["zone_count"]["side"], "Opponent")
        self.assertEqual(effect["zone_count"]["zone"], "BackStage")
        self.assertTrue(effect["per_zone_count"])

    def test_continuous_middle_center_other_character_power(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 Your other character in the middle position of your center stage gets "
            "+1000 power."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["targets"], [{"SelfStageSlot": {"slot": 1}}])
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertTrue(effect["exclude_source"])

    def test_brainstorm_following_action_draw(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 Brainstorm [(1) 【REST】 this card] Flip over 4 cards from the top of your "
            "deck, and put them into your waiting room. For each climax revealed among those "
            'cards, perform the following action. "Draw up to 1 card."'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"Brainstorm": {"reveal_count": 4, "per_climax": 1, "mode": "Draw"}}],
        )

    def test_on_play_look_up_to_top_cards_in_order(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, look at up to 3 cards "
            "from the top of your deck, and put them on the top of your deck in any order."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(ability_defs, [])
        self.assertEqual(abilities, [{"AutoOnPlayRevealDeckTop": {"count": 3}}])

    def test_auto_paid_on_play_clock_to_stock(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [Put 1 card from your hand into your waiting room] When this card is placed "
            "on the stage from your hand, you may pay the cost. If you do, put the top card of "
            "your clock into your stock."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["cost"]["discard_from_hand"], 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToStock"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfClock"])

    def test_normalize_ability_line_strips_control_chars_and_jsx_fragments(self):
        line = (
            "【AUTO】 Encore [Put 1 card from your hand into your waiting room] "
            "(When this card is put into your waiting room from the stage, you may pay the cost. "
            "If you do, return this card to its previous stage position as 【REST】.)'\x10 />"
        )
        normalized = normalize_ability_line(line)
        self.assertNotIn("\x10", normalized)
        self.assertNotIn("/>", normalized)
        self.assertTrue(normalized.endswith("as 【REST】.)"))

    def test_bond_without_resolvable_target_is_marked_unsupported(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 Bond / "Missing Card" '
            "[Put 1 card from your hand into your waiting room] "
            "When this card is played and placed on the stage from your hand, "
            "you may pay the cost. If you do, choose 1 character with "
            '"Missing Card" in your waiting room, and return it to your hand.'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats, name_to_ids={})
        self.assertEqual(abilities, [])
        self.assertEqual(ability_defs, [])
        self.assertEqual(sum(stats.unsupported_signatures.values()), 1)

    def test_on_play_topdeck_search_put_rest_waiting_room_maps_to_def(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, look at up to 3 cards "
            "from the top of your deck, choose up to 1 card from among them, put it into your hand, "
            "and put the rest into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True])
        self.assertEqual(ability_defs[0]["target_limit"], 3)

    def test_on_play_topdeck_search_trait_selector_maps_to_trait_target(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, look at up to 3 cards "
            "from the top of your deck, choose up to 1 《Music》 character from among them, reveal it "
            "to your opponent, put it into your hand, and put the rest into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids=None,
            trait_map={"Music": 7},
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["target_card_type"], "Character")
        self.assertEqual(ability_defs[0]["target_trait"], 7)
        self.assertEqual(ability_defs[0]["target_limit"], 3)

    def test_on_play_topdeck_search_named_selector_maps_to_target_ids(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, look at up to 4 cards "
            'from the top of your deck, choose up to 1 character with "Target" in its card name '
            "from among them, reveal it to your opponent, put it into your hand, and put the rest "
            "into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={
                "Target Name": [101],
                "Another Target Unit": [102],
                "Unrelated": [103],
            },
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["target_card_ids"], [101, 102])
        self.assertEqual(ability_defs[0]["target_limit"], 4)

    def test_on_play_search_deck_to_hand_named_selector_maps_to_target_ids(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, search your deck for up to 1 "
            'character with "Target" in its card name, reveal it to your opponent, put it into your '
            "hand, and shuffle your deck."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={
                "Target Name": [101],
                "Another Target Unit": [102],
                "Unrelated": [103],
            },
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["target_card_ids"], [101, 102])
        self.assertEqual(ability_defs[0]["target_limit"], 1)

    def test_on_play_search_deck_to_hand_dual_trait_selector_maps_to_target_ids(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, search your deck for up to 1 "
            "《Music》 or 《Band》 character, reveal it to your opponent, put it into your hand, and "
            "shuffle your deck."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_map={"Music": 7, "Band": 8},
            trait_to_ids={"Music": [201, 202], "Band": [202, 203]},
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["target_card_ids"], [201, 202, 203])
        self.assertEqual(ability_defs[0]["target_limit"], 1)

    def test_on_play_salvage_named_selector_maps_to_target_ids(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, choose up to 1 "
            'character with "Target" in its card name in your waiting room, and return it to your hand.'
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={
                "Target Name": [101],
                "Another Target Unit": [102],
                "Unrelated": [103],
            },
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfWaitingRoom"])
        self.assertEqual(ability_defs[0]["target_card_ids"], [101, 102])
        self.assertEqual(ability_defs[0]["target_limit"], 1)

    def test_on_play_following_ability_cannot_side_attack_is_flattened(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, choose 1 of your "
            "opponent's characters, and that character gets the following ability until end of turn. "
            '"This card cannot side attack."'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"CannotSideAttack": {"duration_turn": True}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["OppStage"])

    def test_on_attack_following_ability_power_is_flattened(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card attacks, choose 1 of your characters, and that character gets "
            'the following ability until end of turn. "This card gets +2000 power until end of turn."'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 2000, "duration_turn": True}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")

    def test_cxcombo_attack_paid_damage_with_condition_maps_to_def(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 【CXCOMBO】 [(1)] When this card attacks, if "Combo Name" is in your climax '
            "area, you may pay the cost. If you do, deal 2 damage to your opponent. "
            "(Damage may be canceled)"
        )
        abilities, ability_defs, _ = parse_abilities(
            text, "Character", stats, name_to_ids={"Combo Name": [91]}
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"DealDamage": {"amount": 2, "cancelable": True}}],
        )
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"climax_area": {"side": "SelfSide", "card_ids": [91]}},
        )

    def test_cxcombo_attack_optional_damage_with_condition_maps_to_def(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 【CXCOMBO】 When this card attacks, if "Combo Name" is in your climax area, '
            "you may deal 1 damage to your opponent. (Damage may be canceled)"
        )
        abilities, ability_defs, _ = parse_abilities(
            text, "Character", stats, name_to_ids={"Combo Name": [56, 57]}
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"DealDamage": {"amount": 1, "cancelable": True}}],
        )
        self.assertEqual(ability_defs[0]["effect_optional"], [True])
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"climax_area": {"side": "SelfSide", "card_ids": [56, 57]}},
        )

    def test_cxcombo_attack_optional_damage_generic_climax_condition(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 【CXCOMBO】 When this card attacks, if there is a climax in your climax area, "
            "you may deal 1 damage to your opponent. (Damage may be canceled)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"climax_area": {"side": "SelfSide", "card_ids": []}},
        )

    def test_auto_climax_placed_team_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When your climax is placed on your climax area, choose 1 of your characters, "
            "and that character gets +1000 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "AfterClimaxPhase")
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"climax_area": {"side": "SelfSide", "card_ids": []}},
        )
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 1000, "duration_turn": True}}],
        )

    def test_auto_on_reverse_self_bottom_deck(self):
        stats = AbilityParseStats()
        text = "【AUTO】 When this card becomes 【REVERSE】 in battle, put this card at the bottom of your deck."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")
        self.assertEqual(ability_defs[0]["effects"], ["MoveToDeckBottom"])
        self.assertEqual(ability_defs[0]["targets"], ["This"])

    def test_continuous_facing_opponent_gets_soul_modifier(self):
        stats = AbilityParseStats()
        text = "【CONT】 The character facing this card gets -1 soul."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"FacingOpponentAddSoul": {"amount": -1}}],
        )
        self.assertEqual(ability_defs[0]["targets"], [])

    def test_auto_on_damage_dealt_canceled_move_self_to_hand(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 During this card's battle, when damage dealt by this card is canceled, "
            "you may return this card to your hand."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "DamageDealtCanceled")
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["This"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_auto_on_damage_received_canceled_move_self_to_stock(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 During this card's battle, when the damage you received is canceled, "
            "you may put this card into your stock."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "DamageReceivedCanceled")
        self.assertEqual(ability_defs[0]["effects"], ["MoveToStock"])
        self.assertEqual(ability_defs[0]["targets"], ["This"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_on_play_search_top_with_level_cap(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, look at up to 4 cards "
            "from the top of your deck, choose up to 1 level 1 or lower character from among them, "
            "reveal it to your opponent, put it into your hand, and put the rest into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["target_level_max"], 1)
        self.assertEqual(ability_defs[0]["target_limit"], 4)

    def test_act_brainstorm_custom_action_approx_profile(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 Brainstorm [(1)] Flip over 4 cards from the top of your deck, and put them into "
            'your waiting room. For each climax revealed among those cards, perform the following action. "Draw a card."'
        )
        abilities, ability_defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="approx"
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"Brainstorm": {"reveal_count": 4, "per_climax": 1, "mode": "Draw"}}],
        )
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"requires_approx_effects": True},
        )

    def test_act_brainstorm_custom_action_draw_parsed_without_approx_profile(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 Brainstorm [(1)] Flip over 4 cards from the top of your deck, and put them into "
            'your waiting room. For each climax revealed among those cards, perform the following action. "Draw a card."'
        )
        abilities, ability_defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict"
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"Brainstorm": {"reveal_count": 4, "per_climax": 1, "mode": "Draw"}}],
        )
        self.assertEqual(ability_defs[0]["conditions"], {})

    def test_on_play_stage_variant_without_the_parses(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on stage from your hand, "
            "this card gets +1500 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 1500, "duration_turn": True}}],
        )

    def test_auto_on_any_climax_placed_self_power(self):
        stats = AbilityParseStats()
        text = "【AUTO】 When a climax is placed on your climax area, this card gets +1000 power until end of turn."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "AfterClimaxPhase")
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"climax_area": {"side": "SelfSide", "card_ids": []}},
        )
        self.assertEqual(ability_defs[0]["targets"], ["This"])

    def test_auto_on_opponent_climax_placed_move_self_to_stock(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When a climax is placed on your opponent's climax area, "
            "you may put this card into your stock."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "AfterClimaxPhase")
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"climax_area": {"side": "Opponent", "card_ids": []}},
        )
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_act_heal_top_clock(self):
        stats = AbilityParseStats()
        text = "【ACT】 [(1) 【REST】 this card] Put the top card of your clock into your waiting room."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["Heal"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfClock"])
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)
        self.assertTrue(ability_defs[0]["cost"]["rest_self"])

    def test_paid_on_play_topdeck_search_to_hand(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1) Put 1 card from your hand into your waiting room] "
            "When this card is placed on the stage from your hand, you may pay the cost. If you do, "
            "look at up to 4 cards from the top of your deck, choose up to 1 card from among them, "
            "put it into your hand, and put the rest into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)
        self.assertEqual(ability_defs[0]["cost"]["discard_from_hand"], 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["target_limit"], 4)

    def test_on_play_reduce_power_opponent_center(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, choose 1 character in your "
            "opponent's center stage, and that character gets -1000 power until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["targets"], ["OppFrontRow"])
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": -1000, "duration_turn": True}}],
        )

    def test_on_attack_per_opponent_stage_count_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card attacks, this card gets +X power until end of turn. "
            "X is equal to the number of characters your opponent has ×2000."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "TimedConditionalAddPower": {
                        "amount": 2000,
                        "duration_turn": True,
                        "turn": None,
                        "zone_count": {
                            "side": "Opponent",
                            "zone": "Stage",
                            "cmp": "AtLeast",
                            "value": 0,
                        },
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": True,
                        "exclude_source": False,
                        "target_ids": [],
                    }
                }
            ],
        )
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")

    def test_continuous_middle_center_other_power_allows_the_center_stage_wording(self):
        stats = AbilityParseStats()
        text = "【CONT】 Your other character in the middle position of the center stage gets +1000 power."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["targets"], [{"SelfStageSlot": {"slot": 1}}])
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "ConditionalAddPower": {
                        "amount": 1000,
                        "turn": None,
                        "zone_count": None,
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": False,
                        "exclude_source": True,
                        "target_ids": [],
                    }
                }
            ],
        )

    def test_paid_begin_attack_phase_move_to_open_back_stage(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1)] At the beginning of your opponent's attack phase, you may pay the cost. "
            "If you do, move this card to an open position of your back stage."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BeginAttackPhase")
        self.assertEqual(ability_defs[0]["effects"], ["MoveThisToOpenBack"])
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)

    def test_continuous_per_other_trait_is_exact_in_none_profile(self):
        stats = AbilityParseStats()
        text = "【CONT】 This card gets +500 power for each of your other 《Music》 characters."
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_map={"Music": 9},
            trait_to_ids={"Music": [21, 22, 23]},
            source_card_id=21,
            approx_profile="strict",
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["conditions"], {})
        self.assertEqual(ability_defs[0]["targets"], ["This"])
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "ConditionalAddPower": {
                        "amount": 500,
                        "turn": None,
                        "zone_count": {
                            "side": "SelfSide",
                            "zone": "Stage",
                            "cmp": "AtLeast",
                            "value": 0,
                            "card_ids": [21, 22, 23],
                        },
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": True,
                        "exclude_source": False,
                        "target_ids": [],
                    }
                },
                {"AddPower": {"amount": -500, "duration_turn": False}},
            ],
        )

    def test_continuous_middle_center_self_is_exact_in_none_profile(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If this card is in the middle position of your center stage, "
            "this card gets +2000 power."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            approx_profile="strict",
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["conditions"], {})

    def test_continuous_if_has_other_trait_is_exact_in_none_profile(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If you have 2 or more other 《Music》 characters, this card gets +1000 power."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_map={"Music": 11},
            trait_to_ids={"Music": [21, 22, 23]},
            source_card_id=21,
            approx_profile="strict",
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["conditions"], {})
        self.assertEqual(ability_defs[0]["target_trait"], 11)
        zone = ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"]
        self.assertEqual(zone["value"], 3)
        self.assertEqual(zone["card_ids"], [21, 22, 23])

    def test_continuous_same_name_deck_construction_rule_counts_as_supported(self):
        stats = AbilityParseStats()
        text = "【CONT】 You can put any number of cards with the same card name as this card into your deck."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(ability_defs, [])
        self.assertEqual(stats.parsed_lines, 1)

    def test_continuous_hand_level_delta_waiting_room_climax_gate(self):
        stats = AbilityParseStats()
        text = "【CONT】 If your waiting room has 2 or less climax, this card gets -1 level while in your hand."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], [])
        self.assertEqual(ability_defs[0]["conditions"]["hand_level_delta"], -1)
        self.assertEqual(
            ability_defs[0]["conditions"]["self_waiting_room_climax_at_most"],
            2,
        )

    def test_continuous_hand_level_delta_clock_contains_named(self):
        stats = AbilityParseStats()
        text = (
            '【CONT】 If "Named Card" is in your clock, this card gets -1 level while in your hand.'
        )
        abilities, ability_defs, _ = parse_abilities(
            text, "Character", stats, name_to_ids={"Named Card": [42, 99]}
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], [])
        self.assertEqual(ability_defs[0]["conditions"]["hand_level_delta"], -1)
        self.assertEqual(
            ability_defs[0]["conditions"]["self_clock_card_ids_any"],
            [42, 99],
        )

    def test_continuous_hand_level_delta_if_opponent_has_high_level_character(self):
        stats = AbilityParseStats()
        text = "【CONT】 If your opponent has a level 2 or higher character, this card gets -1 level while in your hand."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], [])
        self.assertEqual(ability_defs[0]["conditions"]["hand_level_delta"], -1)
        self.assertEqual(
            ability_defs[0]["conditions"]["opponent_stage_has_level_at_least"],
            2,
        )

    def test_continuous_ignore_color_requirement(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 This card can be played from your hand without fulfilling color requirements."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], [])
        self.assertEqual(ability_defs[0]["conditions"], {"ignore_color_requirement": True})

    def test_continuous_if_have_another_named_card_power(self):
        stats = AbilityParseStats()
        text = '【CONT】 If you have another "Named Card", this card gets +4000 power.'
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Named Card": [7, 8]},
            source_card_id=7,
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        zone = ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"]
        self.assertEqual(zone["value"], 2)
        self.assertEqual(zone["card_ids"], [7, 8])

    def test_continuous_if_have_another_name_fragment_power(self):
        stats = AbilityParseStats()
        text = (
            '【CONT】 If you have another character with "Shinobu" in its card name, '
            "this card gets +1500 power."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Shinobu A": [3], "Other Shinobu B": [9], "Not Match": [10]},
            source_card_id=11,
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        zone = ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"]
        self.assertEqual(zone["value"], 1)
        self.assertEqual(zone["card_ids"], [3, 9])

    def test_continuous_all_other_dual_trait_power(self):
        stats = AbilityParseStats()
        text = "【CONT】 All of your other 《Avatar》 or 《Net》 characters get +500 power."
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_to_ids={"Avatar": [10, 12], "Net": [11, 12]},
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "ConditionalAddPower": {
                        "amount": 500,
                        "turn": None,
                        "zone_count": None,
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": False,
                        "exclude_source": True,
                        "target_ids": [10, 11, 12],
                    }
                }
            ],
        )

    def test_continuous_all_other_following_cannot_side_attack(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 All of your other characters get the following ability. "
            '"【CONT】 This card cannot side attack."'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "ConditionalCannotSideAttack": {
                        "turn": None,
                        "zone_count": None,
                        "require_source_marker": False,
                        "exclude_source": True,
                    }
                }
            ],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])

    def test_on_climax_play_colored_waiting_room_stock_and_team_soul(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on your climax area from your hand, choose up to 1 red card "
            "in your waiting room, put it into your stock, and all of your characters get +1 soul until end of turn."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Climax", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")
        self.assertEqual(
            ability_defs[0]["effects"],
            ["MoveToStock", {"AddSoul": {"amount": 1, "duration_turn": True}}],
        )
        self.assertEqual(ability_defs[0]["target_limit"], 1)
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_on_play_draw_discard_then_optional_stock_top(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, draw up to 2 cards, "
            "choose 2 cards in your hand, put them into your waiting room, and put up to 1 card "
            "from the top of your deck into your stock."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 2)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"Draw": {"count": 2}}, "MoveToWaitingRoom"],
        )
        self.assertEqual(ability_defs[0]["targets"], ["SelfHand"])
        self.assertEqual(ability_defs[0]["target_limit"], 2)
        self.assertEqual(ability_defs[0]["effect_optional"], [True])
        self.assertEqual(ability_defs[1]["effects"], ["MoveToStock"])
        self.assertEqual(ability_defs[1]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[1]["target_limit"], 1)
        self.assertEqual(ability_defs[1]["effect_optional"], [True])

    def test_continuous_experience_power(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 Experience If the total level of the cards in your level is 3 or higher, "
            "this card gets +4000 power."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertEqual(effect["amount"], 4000)
        self.assertEqual(effect["turn"], None)
        self.assertEqual(
            effect["zone_count"],
            {"side": "SelfSide", "zone": "LevelTotal", "cmp": "AtLeast", "value": 3},
        )

    def test_continuous_experience_power_during_your_turn(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 Experience During your turn, if the total level of the cards in your level "
            "is 2 or higher, this card gets +2000 power."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertEqual(effect["amount"], 2000)
        self.assertEqual(effect["turn"], "SelfTurn")
        self.assertEqual(
            effect["zone_count"],
            {"side": "SelfSide", "zone": "LevelTotal", "cmp": "AtLeast", "value": 2},
        )

    def test_continuous_power_if_stock_or_hand_count_condition(self):
        stats = AbilityParseStats()
        text = "【CONT】 If the number of cards in your stock is two or less, this card gets +1500 power."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"],
            {"side": "SelfSide", "zone": "Stock", "cmp": "AtMost", "value": 2},
        )

    def test_continuous_power_if_has_other_characters_count(self):
        stats = AbilityParseStats()
        text = "【CONT】 If you have 3 or more other characters, this card gets +1000 power."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"],
            {"side": "SelfSide", "zone": "Stage", "cmp": "AtLeast", "value": 4},
        )

    def test_continuous_if_has_other_trait_rule_uses_card_ids_and_self_adjust(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If you have 2 or more other 《Music》 characters, this card gets +1000 power."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_map={"Music": 7},
            trait_to_ids={"Music": [10, 20, 30]},
            source_card_id=10,
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        zone = ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"]
        self.assertEqual(zone["value"], 3)
        self.assertEqual(zone["card_ids"], [10, 20, 30])

    def test_continuous_other_trait_count_form(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If the number of other 《Music》 characters you have is three or more, "
            "this card gets +1000 power."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_to_ids={"Music": [1, 2, 5]},
            source_card_id=5,
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        zone = ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"]
        self.assertEqual(zone["value"], 4)
        self.assertEqual(zone["card_ids"], [1, 2, 5])

    def test_continuous_other_dual_trait_count_form(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If the number of your other 《Avatar》 or 《Net》 characters is three or more, "
            "this card gets +500 power."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_to_ids={"Avatar": [7, 8], "Net": [8, 9]},
            source_card_id=9,
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        zone = ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"]
        self.assertEqual(zone["value"], 4)
        self.assertEqual(zone["card_ids"], [7, 8, 9])

    def test_continuous_per_other_dual_trait_power(self):
        stats = AbilityParseStats()
        text = "【CONT】 This card gets +500 power for each of your other 《Avatar》 or 《Net》 characters."
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_to_ids={"Avatar": [31, 32], "Net": [32, 33]},
            source_card_id=33,
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"]["card_ids"],
            [31, 32, 33],
        )
        self.assertEqual(
            ability_defs[0]["effects"][1],
            {"AddPower": {"amount": -500, "duration_turn": False}},
        )

    def test_continuous_all_other_cards_named_power(self):
        stats = AbilityParseStats()
        text = '【CONT】 All of your other cards named "Homunculus" get +1000 power.'
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Homunculus": [42, 43]},
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertEqual(effect["amount"], 1000)
        self.assertEqual(effect["exclude_source"], True)
        self.assertEqual(effect["target_ids"], [42, 43])

    def test_continuous_all_other_name_fragment_power(self):
        stats = AbilityParseStats()
        text = (
            '【CONT】 All of your other characters with "Hitagi" in its card name get +500 power.'
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Hitagi A": [11], "B Hitagi": [12], "Other": [13]},
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        effect = ability_defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertEqual(effect["amount"], 500)
        self.assertEqual(effect["exclude_source"], True)
        self.assertEqual(effect["target_ids"], [11, 12])

    def test_continuous_all_opponent_following_unknown_is_approx_approx_only(self):
        text = (
            "【CONT】 All of your opponent's characters get "
            '"【AUTO】 Encore [Put the top card of your deck into your clock]".'
        )
        none_stats = AbilityParseStats()
        none_abilities, none_defs, _ = parse_abilities(
            text, "Character", none_stats, approx_profile="strict"
        )
        self.assertEqual(none_abilities, [])
        self.assertEqual(none_defs, [])
        self.assertEqual(none_stats.parsed_lines, 0)

        rl_stats = AbilityParseStats()
        rl_abilities, rl_defs, _ = parse_abilities(
            text, "Character", rl_stats, approx_profile="approx"
        )
        self.assertEqual(rl_abilities, [])
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["effects"], [{"Draw": {"count": 0}}])
        self.assertEqual(rl_defs[0]["conditions"], {"requires_approx_effects": True})

    def test_auto_on_play_following_next_turn_encore_stock_is_exact_in_both_profiles(self):
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, choose 1 of your opponent's "
            "characters, and that character gets the following ability until the end of your opponent's "
            'next turn. "【AUTO】 Encore [(2)]".'
        )
        none_stats = AbilityParseStats()
        none_abilities, none_defs, _ = parse_abilities(
            text, "Character", none_stats, approx_profile="strict"
        )
        self.assertEqual(none_abilities, [])
        self.assertEqual(len(none_defs), 1)
        self.assertEqual(none_defs[0]["timing"], "OnPlay")
        self.assertEqual(list(none_defs[0]["effects"][0].keys()), ["GrantAbilityDef"])
        none_grant = none_defs[0]["effects"][0]["GrantAbilityDef"]
        self.assertEqual(none_grant["duration"], "UntilEndOfOpponentsNextTurn")
        self.assertEqual(
            none_grant["ability"]["effects"],
            [{"EncoreStockCost": {"cost": 2, "duration_turn": True}}],
        )
        self.assertEqual(none_defs[0]["conditions"], {})

        rl_stats = AbilityParseStats()
        rl_abilities, rl_defs, _ = parse_abilities(
            text, "Character", rl_stats, approx_profile="approx"
        )
        self.assertEqual(rl_abilities, [])
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "OnPlay")
        self.assertEqual(list(rl_defs[0]["effects"][0].keys()), ["GrantAbilityDef"])
        rl_grant = rl_defs[0]["effects"][0]["GrantAbilityDef"]
        self.assertEqual(rl_grant["duration"], "UntilEndOfOpponentsNextTurn")
        self.assertEqual(
            rl_grant["ability"]["effects"],
            [{"EncoreStockCost": {"cost": 2, "duration_turn": True}}],
        )
        self.assertEqual(rl_defs[0]["conditions"], {})

    def test_auto_when_use_this_cards_ability_with_encore_stock_is_exact_in_both_profiles(self):
        text = (
            '【AUTO】 When you use this card\'s "Resonate", choose 1 of your characters in battle, '
            'and that character gets the following ability until end of turn. "【AUTO】 Encore [(2)]".'
        )
        none_stats = AbilityParseStats()
        none_abilities, none_defs, _ = parse_abilities(
            text, "Character", none_stats, approx_profile="strict"
        )
        self.assertEqual(none_abilities, [])
        self.assertEqual(len(none_defs), 1)
        self.assertEqual(none_defs[0]["timing"], "UseAct")
        self.assertEqual(
            none_defs[0]["effects"],
            [{"EncoreStockCost": {"cost": 2, "duration_turn": True}}],
        )
        self.assertEqual(none_defs[0]["conditions"], {})

        rl_stats = AbilityParseStats()
        rl_abilities, rl_defs, _ = parse_abilities(
            text, "Character", rl_stats, approx_profile="approx"
        )
        self.assertEqual(rl_abilities, [])
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "UseAct")
        self.assertEqual(
            rl_defs[0]["effects"],
            [{"EncoreStockCost": {"cost": 2, "duration_turn": True}}],
        )
        self.assertEqual(rl_defs[0]["conditions"], {})

    def test_continuous_stage_level_modifier(self):
        stats = AbilityParseStats()
        text = "【CONT】 This card gets -1 level while on the stage."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddLevel": {"amount": -1, "duration_turn": False}}],
        )

    def test_continuous_marker_level_power_per_marker(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 This card gets +1 level and +1500 power for each marker underneath this card."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"][0]["ConditionalAddLevel"]["amount"],
            1,
        )
        self.assertTrue(ability_defs[0]["effects"][0]["ConditionalAddLevel"]["per_source_marker"])
        self.assertEqual(
            ability_defs[0]["effects"][1]["ConditionalAddPower"]["amount"],
            1500,
        )

    def test_auto_use_this_card_mill_top(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 When you use this card\'s "Backup", put the top 3 cards of your deck into '
            "your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "UseAct")
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(ability_defs[0]["effects"], ["MoveToWaitingRoom"])
        self.assertEqual(ability_defs[0]["target_limit"], 3)

    def test_auto_use_this_card_recycle_waiting_room(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 [(2)] When you use this card\'s "Backup", you may pay the cost. If you do, '
            "return all cards from your waiting room to your deck, and shuffle your deck."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "UseAct")
        self.assertEqual(ability_defs[0]["effects"], ["RecycleWaitingRoomToDeckShuffle"])
        self.assertEqual(ability_defs[0]["cost"]["stock"], 2)

    def test_auto_battle_opponent_reverse_min_level_stock_top(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card's level 2 or higher battle opponent becomes 【REVERSE】, "
            "you may put the top card of your deck into your stock."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BattleOpponentReverse")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"BattleOpponentTopDeckToStockIf": {"min_level": 2}}],
        )

    def test_continuous_hand_level_waiting_room_number_of_climax_wording(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If the number of climax in your waiting room is two or less, "
            "this card gets -1 level while in your hand."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"hand_level_delta": -1, "self_waiting_room_climax_at_most": 2},
        )

    def test_continuous_cannot_play_events_or_backup_from_hand(self):
        stats = AbilityParseStats()
        text = '【CONT】 You cannot play events or "Backup" from your hand.'
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {"CannotPlayEventsFromHand": {"duration_turn": False}},
                {"CannotPlayBackupFromHand": {"duration_turn": False}},
            ],
        )

    def test_auto_on_play_or_auto_effect_heal_top_clock(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand or by the 【AUTO】 effect of "
            '"Tutor of the Quintuplets, Futaro Uesugi", you may put the top card of your clock '
            "into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")
        self.assertEqual(ability_defs[0]["effects"], ["Heal"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfClock"])

    def test_brainstorm_following_look_top_to_hand(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 Brainstorm [(1)] Flip over 4 cards from the top of your deck, and put them into "
            "your waiting room. For each climax revealed among those cards, perform the following action. "
            '"Look at up to 3 cards from the top of your deck, choose up to 1 card from among them, '
            'put it into your hand, and put the rest into your waiting room."'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "Brainstorm": {
                        "reveal_count": 4,
                        "per_climax": 1,
                        "mode": "LookTopToHand",
                    }
                }
            ],
        )

    def test_brainstorm_following_salvage_then_discard(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 Brainstorm [(1)] Flip over 4 cards from the top of your deck, and put them into "
            "your waiting room. For each climax revealed among those cards, perform the following action. "
            '"Choose 1 character in your waiting room, return it to your hand, choose 1 card in your '
            'hand, and put it into your waiting room."'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "Brainstorm": {
                        "reveal_count": 4,
                        "per_climax": 1,
                        "mode": "SalvageCharacterThenDiscard",
                    }
                }
            ],
        )

    def test_auto_paid_stage_to_waiting_room_search_top_level_at_least(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [Put 1 card from your hand into your waiting room] When this card is put into your "
            "waiting room from the stage, you may pay the cost. If you do, look at up to 4 cards from "
            "the top of your deck, choose up to 1 level 1 or higher card from among them, reveal it to "
            "your opponent, put it into your hand, and put the rest into your waiting room. "
            "(Climax are regarded as level 0)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "SearchTopDeckToHandLevelAtLeastMillRest": {
                        "look_count": 4,
                        "choose_count": 1,
                        "min_level": 1,
                    }
                }
            ],
        )

    def test_auto_paid_on_play_reveal_top_and_salvage_by_revealed_level(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [Put 1 card from your hand into your waiting room] When this card is placed on the "
            "stage from your hand, you may pay the cost. If you do, reveal the top card of your deck, "
            "choose 1 level X or lower character in your waiting room, and return it to your hand. X is "
            "equal to the level of the revealed card. (Climax are regarded as level 0. Return the "
            "revealed card to its original place)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"RevealTopAndSalvageByRevealedLevel": {"count": 1, "climax_level": 0}}],
        )

    def test_auto_on_play_power_per_trait_count(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, this card gets +X power "
            "until end of turn. X is equal to the number of 《Music》 characters you have ×500."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_map={"Music": 1},
            trait_to_ids={"Music": [101, 102]},
            source_card_id=101,
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "TimedConditionalAddPower": {
                        "amount": 500,
                        "duration_turn": True,
                        "turn": None,
                        "zone_count": {
                            "side": "SelfSide",
                            "zone": "Stage",
                            "cmp": "AtLeast",
                            "value": 0,
                            "card_ids": [101, 102],
                        },
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": True,
                        "exclude_source": False,
                        "target_ids": [],
                    }
                }
            ],
        )

    def test_auto_battle_opponent_reverse_cxcombo_salvage_clause_prefix(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 【CXCOMBO】 When this card\'s battle opponent becomes 【REVERSE】, if "Combo Name" '
            "is in your climax area, you may choose 1 character in your waiting room, and return it "
            "to your hand."
        )
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Combo Name": [777]},
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BattleOpponentReverse")
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfWaitingRoom"])
        self.assertEqual(ability_defs[0]["target_limit"], 1)
        self.assertEqual(ability_defs[0]["target_card_type"], "Character")
        self.assertEqual(
            ability_defs[0]["conditions"],
            {"climax_area": {"side": "SelfSide", "card_ids": [777]}},
        )
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_auto_when_your_other_trait_character_attacks_self_gets_power(self):
        stats = AbilityParseStats()
        text = "【AUTO】 When your other 《Music》 character attacks, this card gets +1000 power until end of turn."
        abilities, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_map={"Music": 1},
            trait_to_ids={"Music": [101, 102]},
        )
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OtherAttackDeclaration")
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "AddPowerIfOtherAttackerMatches": {
                        "amount": 1000,
                        "duration_turn": True,
                        "attacker_card_ids": [101, 102],
                    }
                }
            ],
        )

    def test_continuous_middle_center_stage_add_soul(self):
        stats = AbilityParseStats()
        text = "【CONT】 If this card is in the middle position of your center stage, this card gets +1 soul."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddSoulIfMiddleCenter": {"amount": 1}}],
        )

    def test_auto_on_reverse_top_deck_to_clock_and_rest_self(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card becomes 【REVERSE】 in battle, put the top card of your deck into "
            "your clock, and 【REST】 this card."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")
        self.assertEqual(ability_defs[0]["effects"], ["MoveToClock", "RestTarget"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfDeckTop", "This"])

    def test_auto_on_reverse_lock_auto_encore_for_self(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 When this card becomes 【REVERSE】 in battle, you cannot use "【AUTO】 Encore" '
            'until end of turn. (You cannot use the "【AUTO】 Encore [(3)]" rule either)'
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnReverse")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"CannotUseAutoEncoreForPlayer": {"target": "SelfSide"}}],
        )

    def test_auto_begin_encore_step_heal_if_played_from_hand_this_turn(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 During the turn that this card is placed on the stage from your hand, at the "
            "beginning of the encore step, you may put the top card of your clock into your waiting room."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BeginEncoreStep")
        self.assertEqual(
            ability_defs[0]["effects"],
            ["HealIfSourcePlayedFromHandThisTurn"],
        )
        self.assertEqual(ability_defs[0]["effect_optional"], [True])

    def test_auto_paid_on_play_resets_opponent_stock(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1)] When this card is placed on the stage from your hand, you may pay the cost. "
            "If you do, your opponent puts all of their stock into their waiting room, and puts the same "
            "number of cards from the top of their deck into their stock."
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"ResetStockFromDeckTop": {"target": "Opponent"}}],
        )
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)

    def test_auto_change_begin_climax_phase_is_approx_approx_only(self):
        text = (
            "【AUTO】 Change [Return this card to your hand] At the beginning of your climax phase, "
            "you may pay the cost. If you do, choose up to 1 《Quintuplets》 character in your hand, "
            "and put it on the stage position that this card was on."
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])
        self.assertEqual(none_stats.parsed_lines, 0)

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "BeginClimaxPhase")
        self.assertEqual(rl_defs[0]["effects"], [{"Draw": {"count": 0}}])
        self.assertEqual(rl_defs[0]["conditions"], {"requires_approx_effects": True})

    def test_auto_change_named_waiting_room_to_same_slot_exact(self):
        text = (
            "【AUTO】 Change [Put this card into your waiting room] At the beginning of your climax "
            'phase, you may pay the cost. If you do, choose a card named "Target" in your waiting '
            "room, and put it on the stage position that this card was on."
        )
        stats = AbilityParseStats()
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Target": [321]},
            approx_profile="strict",
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "BeginClimaxPhase")
        self.assertEqual(
            defs[0]["effects"][0],
            {"MoveWaitingRoomCardToSourceSlot": {"target_ids": [321]}},
        )
        self.assertEqual(defs[0]["targets"], ["SelfWaitingRoom"])
        self.assertEqual(defs[0]["effect_optional"], [True])

    def test_act_look_reorder_is_exact_in_strict_and_approx(self):
        text = (
            "【ACT】 [【REST】 this card] Look at up to 2 cards from the top of your deck, and put "
            "them on the top of your deck in any order."
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(len(none_defs), 1)
        self.assertEqual(
            none_defs[0]["effects"],
            [{"LookTopDeckReorder": {"count": 2}}],
        )
        self.assertEqual(none_defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(none_defs[0]["cost"]["rest_self"], True)

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(
            rl_defs[0]["effects"],
            [{"LookTopDeckReorder": {"count": 2}}],
        )
        self.assertEqual(rl_defs[0]["conditions"], {})

    def test_auto_attack_look_reorder_is_exact_in_strict_and_approx(self):
        text = (
            "【AUTO】 When this card attacks, look at up to 2 cards from the top of your deck, and "
            "put them on the top of your deck in any order."
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(len(none_defs), 1)
        self.assertEqual(none_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(
            none_defs[0]["effects"],
            [{"LookTopDeckReorder": {"count": 2}}],
        )
        self.assertEqual(none_defs[0]["targets"], ["SelfDeckTop"])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(
            rl_defs[0]["effects"],
            [{"LookTopDeckReorder": {"count": 2}}],
        )
        self.assertEqual(rl_defs[0]["conditions"], {})

    def test_auto_resonate_begin_climax_phase_self_power(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 Resonate [Reveal 1 "New Jersey" in your hand] At the beginning of your climax '
            "phase, you may pay the cost. If you do, this card gets +2000 power until end of turn."
        )
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BeginClimaxPhase")
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"AddPower": {"amount": 2000, "duration_turn": True}}],
        )
        self.assertEqual(ability_defs[0]["targets"], ["This"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True])
        self.assertEqual(ability_defs[0]["cost"]["reveal_from_hand"], 1)

    def test_continuous_assist_front_power_scales_with_name_fragment_count(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 Assist All of your characters in front of this card get +X power. X is equal to "
            '500 multiplied by the number of characters you have with "Sakura".'
        )
        _, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Sakura Alpha": [101], "Sakura Beta": [102], "Other": [999]},
        )
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["targets"], ["SelfFrontRow"])
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "ConditionalAddPower": {
                        "amount": 500,
                        "turn": None,
                        "zone_count": {
                            "side": "SelfSide",
                            "zone": "Stage",
                            "cmp": "AtLeast",
                            "value": 0,
                            "card_ids": [101, 102],
                        },
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": True,
                        "exclude_source": False,
                        "target_ids": [],
                    }
                }
            ],
        )

    def test_continuous_other_named_in_center_stage_scales_power(self):
        stats = AbilityParseStats()
        text = '【CONT】 This card gets +500 power for each other "Sakura" in your center stage.'
        _, ability_defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Sakura A": [101], "Sakura B": [102], "Other": [500]},
            source_card_id=101,
        )
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["targets"], ["This"])
        self.assertEqual(len(ability_defs[0]["effects"]), 2)
        self.assertEqual(
            ability_defs[0]["effects"][0],
            {
                "ConditionalAddPower": {
                    "amount": 500,
                    "turn": None,
                    "zone_count": {
                        "side": "SelfSide",
                        "zone": "FrontStage",
                        "cmp": "AtLeast",
                        "value": 0,
                        "card_ids": [101, 102],
                    },
                    "require_source_marker": False,
                    "per_source_marker": False,
                    "per_zone_count": True,
                    "exclude_source": False,
                    "target_ids": [],
                }
            },
        )
        self.assertEqual(
            ability_defs[0]["effects"][1],
            {"AddPower": {"amount": -500, "duration_turn": False}},
        )

    def test_auto_on_play_power_scales_with_opponent_stage_count(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, this card gets +X power "
            "until end of turn. X is equal to the number of characters your opponent has ×500."
        )
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "TimedConditionalAddPower": {
                        "amount": 500,
                        "duration_turn": True,
                        "turn": None,
                        "zone_count": {
                            "side": "Opponent",
                            "zone": "Stage",
                            "cmp": "AtLeast",
                            "value": 0,
                        },
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": True,
                        "exclude_source": False,
                        "target_ids": [],
                    }
                }
            ],
        )

    def test_auto_attack_choose_other_character_gets_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card attacks, choose 1 of your other characters, and that character "
            "gets +1000 power until end of turn."
        )
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(ability_defs[0]["targets"], ["SelfStage"])
        self.assertEqual(ability_defs[0]["target_limit"], 1)
        self.assertEqual(
            ability_defs[0]["effects"][0]["TimedConditionalAddPower"]["exclude_source"], True
        )

    def test_activated_draw_then_discard(self):
        stats = AbilityParseStats()
        text = "【ACT】 [(1)] Draw 1 card, choose 1 card in your hand, and put it into your waiting room."
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [{"Draw": {"count": 1}}, "MoveToWaitingRoom"],
        )
        self.assertEqual(ability_defs[0]["targets"], ["This", "SelfHand"])
        self.assertEqual(ability_defs[0]["cost"]["stock"], 1)
        self.assertEqual(ability_defs[0]["target_limit"], 1)

    def test_activated_salvage_waiting_room(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 [(1) Put 1 climax from your hand into your waiting room] Choose 1 character in "
            "your waiting room, and return it to your hand."
        )
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfWaitingRoom"])
        self.assertEqual(ability_defs[0]["target_card_type"], "Character")
        self.assertEqual(ability_defs[0]["target_limit"], 1)

    def test_continuous_if_you_have_at_most_other_characters(self):
        stats = AbilityParseStats()
        text = "【CONT】 If you have 1 or less other characters, this card gets +1000 power."
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "ConditionalAddPower": {
                        "amount": 1000,
                        "turn": None,
                        "zone_count": {
                            "side": "SelfSide",
                            "zone": "Stage",
                            "cmp": "AtMost",
                            "value": 2,
                        },
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": False,
                        "exclude_source": False,
                        "target_ids": [],
                    }
                }
            ],
        )

    def test_continuous_facing_quoted_is_approx_approx_only(self):
        text = (
            '【CONT】 The character facing this card gets "【CONT】 This card cannot side attack."'
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["effects"], [{"Draw": {"count": 0}}])
        self.assertEqual(rl_defs[0]["conditions"], {"requires_approx_effects": True})

    def test_continuous_same_name_up_to_rule_parsed(self):
        stats = AbilityParseStats()
        text = "【CONT】 You can put up to 6 cards with the same card name as this card into your deck."
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(ability_defs, [])
        self.assertEqual(stats.parsed_lines, 1)

    def test_auto_attack_battle_opponent_level_power_and_soul(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card attacks, if the character facing this card is level 3 or higher, "
            "this card gets +4000 power and +1 soul until end of turn."
        )
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "AddPowerIfBattleOpponentLevelAtLeast": {
                        "amount": 4000,
                        "min_level": 3,
                        "duration_turn": True,
                    }
                },
                {
                    "AddSoulIfBattleOpponentLevelAtLeast": {
                        "amount": 1,
                        "min_level": 3,
                        "duration_turn": True,
                    }
                },
            ],
        )

    def test_auto_on_reverse_cxcombo_put_battle_opponent_to_clock(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 【CXCOMBO】 When this card\'s battle opponent becomes 【REVERSE】, if "War Heroine" '
            "is in your climax area, you may put that character into your opponent's clock."
        )
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "BattleOpponentReverse")
        self.assertEqual(
            ability_defs[0]["effects"],
            [
                {
                    "BattleOpponentMoveToClockIf": {
                        "max_level": None,
                        "max_cost": None,
                        "level_gt_opponent_level": False,
                    }
                }
            ],
        )
        self.assertIn("climax_area", ability_defs[0]["conditions"])

    def test_auto_encore_variant_with_stage_sacrifice_cost(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 Encore [(1) Put 1 character from your stage into your waiting room] "
            "(When this card is put into your waiting room from the stage, you may pay the cost. "
            "If you do, return this card to its previous stage position as 【REST】)"
        )
        abilities, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(ability_defs, [])
        self.assertEqual(len(abilities), 1)
        cost = abilities[0]["EncoreVariant"]["cost"]
        self.assertEqual(cost["stock"], 1)
        self.assertEqual(cost["sacrifice_from_stage"], 1)

    def test_activated_salvage_with_put_self_cost(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 [Put 1 card from your hand into your waiting room & Put this card into your "
            "waiting room] Choose 1 character in your waiting room, and return it to your hand."
        )
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(ability_defs[0]["cost"]["discard_from_hand"], 1)
        self.assertTrue(ability_defs[0]["cost"]["move_self_to_waiting_room"])

    def test_auto_on_play_clock_swap_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, you may choose 1 card "
            "in your clock, and return it to your hand. If you do, choose 1 card in your hand, "
            "and put it into your clock."
        )
        _, ability_defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(ability_defs), 1)
        self.assertEqual(ability_defs[0]["timing"], "OnPlay")
        self.assertEqual(ability_defs[0]["effects"], ["MoveToHand", "MoveToClock"])
        self.assertEqual(ability_defs[0]["targets"], ["SelfClock", "SelfHand"])
        self.assertEqual(ability_defs[0]["effect_optional"], [True, True])

    def test_auto_on_play_look_reorder_then_bounce_rl_partial(self):
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, look at up to 3 cards "
            "from the top of your deck, put them on the top of your deck in any order, choose up "
            "to 1 of your opponent's characters, and return it to their hand."
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(rl_defs[0]["targets"], ["OppStage"])
        self.assertEqual(rl_defs[0]["conditions"], {"requires_approx_effects": True})

    def test_act_following_ability_is_approx_approx_only(self):
        text = (
            "【ACT】 [【REST】 this card] Choose 1 of your characters, and that character gets the "
            "following ability until end of turn. "
            "\"【AUTO】 When this card's battle opponent becomes 【REVERSE】, you may put that "
            "character into your opponent's stock. If you do, put the bottom card of your "
            "opponent's stock into their waiting room.\""
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["effects"], [{"Draw": {"count": 0}}])
        self.assertEqual(
            rl_defs[0]["conditions"],
            {"requires_approx_effects": True},
        )

    def test_auto_damage_dealt_not_canceled_adds_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When damage dealt by this card is not canceled, this card gets +6000 power "
            "until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "DamageDealtNotCanceled")
        self.assertEqual(
            defs[0]["effects"],
            [{"AddPower": {"amount": 6000, "duration_turn": True}}],
        )

    def test_auto_level_up_moves_self_to_waiting_room(self):
        stats = AbilityParseStats()
        text = "【AUTO】 When you level up, put this card into your waiting room."
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "LevelUp")
        self.assertEqual(defs[0]["effects"], ["MoveToWaitingRoom"])
        self.assertEqual(defs[0]["targets"], ["This"])

    def test_auto_change_begin_draw_phase_named_waiting_room_to_slot(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 Change [(1) Put this card into your waiting room] At the beginning of your "
            'draw phase, you may pay the cost. If you do, choose a card named "Target" in your '
            "waiting room, and put it on the stage position that this card was on."
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Target": [42]},
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "BeginDrawPhase")
        self.assertEqual(
            defs[0]["effects"][0],
            {"MoveWaitingRoomCardToSourceSlot": {"target_ids": [42]}},
        )
        self.assertEqual(defs[0]["cost"]["stock"], 1)
        self.assertTrue(defs[0]["cost"]["move_self_to_waiting_room"])

    def test_auto_on_climax_play_team_power_soul(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on your climax area from your hand, choose up to 2 "
            "of your characters, and those characters get +1000 power and +1 soul until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Climax", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[0]["targets"], ["SelfStage", "SelfStage"])
        self.assertEqual(defs[0]["target_limit"], 2)

    def test_auto_on_play_all_opp_center_get_power_down(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, all characters in your "
            "opponent's center stage get -500 power until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[0]["targets"], ["OppFrontRow"])
        self.assertEqual(
            defs[0]["effects"],
            [{"AddPower": {"amount": -500, "duration_turn": True}}],
        )

    def test_continuous_memory_named_power_exact_condition(self):
        stats = AbilityParseStats()
        text = (
            '【CONT】 Memory If "Target Memory Card" is in your memory, this card gets +3000 power.'
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Target Memory Card": [77]},
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["targets"], ["This"])
        self.assertEqual(
            defs[0]["effects"],
            [{"AddPower": {"amount": 3000, "duration_turn": False}}],
        )
        self.assertEqual(defs[0]["conditions"], {"self_memory_card_ids_any": [77]})

    def test_continuous_facing_level_power_exact(self):
        stats = AbilityParseStats()
        text = "【CONT】 If the character facing this card is level 3 or higher, this card gets +4500 power."
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["targets"], ["This"])
        self.assertEqual(
            defs[0]["effects"],
            [
                {
                    "AddPowerIfBattleOpponentLevelAtLeast": {
                        "amount": 4500,
                        "min_level": 3,
                        "duration_turn": False,
                    }
                }
            ],
        )

    def test_continuous_same_name_up_to_rule_in_deck_parsed(self):
        stats = AbilityParseStats()
        text = "【CONT】 You can put up to eight cards with the same card name as this card in your deck."
        abilities, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(abilities, [])
        self.assertEqual(defs, [])
        self.assertEqual(stats.parsed_lines, 1)

    def test_auto_opponent_climax_stock_pronoun_variant(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When your opponent's climax is placed on their climax area, "
            "you may put this card into your stock."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "AfterClimaxPhase")
        self.assertEqual(defs[0]["effects"], ["MoveToStock"])
        self.assertEqual(defs[0]["targets"], ["This"])
        self.assertEqual(defs[0]["effect_optional"], [True])
        self.assertEqual(
            defs[0]["conditions"],
            {"climax_area": {"side": "Opponent", "card_ids": []}},
        )

    def test_auto_damage_received_not_canceled_adds_power(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 During this card's battle, when the damage you received is not canceled, "
            "this card gets +4000 power until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "DamageReceivedNotCanceled")
        self.assertEqual(
            defs[0]["effects"],
            [{"AddPower": {"amount": 4000, "duration_turn": True}}],
        )

    def test_auto_on_play_named_face_down_marker_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, you may choose a card named "
            '"Stored Marker" in your waiting room, and put it face down underneath this card as a marker.'
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Stored Marker": [314]},
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[0]["targets"], ["SelfWaitingRoom"])
        self.assertEqual(defs[0]["target_limit"], 1)
        self.assertEqual(defs[0]["effect_optional"], [True])
        self.assertEqual(
            defs[0]["effects"],
            [{"MoveToMarker": {"target_ids": [314]}}],
        )

    def test_auto_on_play_paid_bounce_opp_stage_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(2)] When this card is placed on the stage from your hand, you may pay the cost. "
            "If you do, choose 1 of your opponent's characters, and return it to their hand."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[0]["targets"], ["OppStage"])
        self.assertEqual(defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(defs[0]["cost"]["stock"], 2)

    def test_auto_on_reverse_paid_move_self_to_memory_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1)] When this card becomes 【REVERSE】 in battle, you may pay the cost. "
            "If you do, put this card into your memory."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnReverse")
        self.assertEqual(defs[0]["targets"], ["This"])
        self.assertEqual(defs[0]["effects"], ["MoveToMemory"])
        self.assertEqual(defs[0]["cost"]["stock"], 1)
        self.assertEqual(defs[0]["effect_optional"], [True])

    def test_activated_put_self_cost_bounce_opp_exact(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 [Put this card into your waiting room] Choose 1 of your opponent's characters, "
            "and return it to their hand."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["targets"], ["OppStage"])
        self.assertEqual(defs[0]["effects"], ["MoveToHand"])
        self.assertTrue(defs[0]["cost"]["move_self_to_waiting_room"])

    def test_auto_use_act_following_is_exact(self):
        text = (
            "【AUTO】 This ability activates up to 1 time per turn. When you use an 【ACT】, "
            "this card gets the following ability until end of turn. "
            "\"【AUTO】 When this card's battle opponent becomes 【REVERSE】, you may put the top card "
            'of your deck into your stock."'
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(len(none_defs), 1)
        self.assertEqual(none_defs[0]["timing"], "UseAct")
        self.assertEqual(
            list(none_defs[0]["effects"][0].keys()),
            ["GrantAbilityDef"],
        )
        granted = none_defs[0]["effects"][0]["GrantAbilityDef"]["ability"]
        self.assertEqual(granted["timing"], "BattleOpponentReverse")
        self.assertEqual(granted["effects"], ["MoveToStock"])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "UseAct")
        self.assertEqual(
            list(rl_defs[0]["effects"][0].keys()),
            ["GrantAbilityDef"],
        )
        self.assertEqual(rl_defs[0]["conditions"], {})

    def test_auto_begin_opponent_draw_mill_gate_return_approx_approx(self):
        text = (
            "【AUTO】 At the beginning of your opponent's draw phase, put the top 2 cards of your deck "
            "into your waiting room. If there is a level 2 or higher card among those cards, you may "
            "return this card to your hand. (Climax are regarded as level 0)"
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "BeginDrawPhase")
        self.assertEqual(
            rl_defs[0]["conditions"],
            {"requires_approx_effects": True, "turn": "OpponentTurn"},
        )

    def test_auto_cxcombo_named_following_approx_approx(self):
        text = (
            '【AUTO】 【CXCOMBO】 When "Birds of a Feather" is placed on your climax area, choose 1 of '
            "your other characters, and that character gets the following ability until end of turn. "
            "\"【AUTO】 When this card's battle opponent becomes 【REVERSE】, look at up to 4 cards from "
            "the top of your deck, choose up to 1 《Tomoeda》 character from among them, reveal it to "
            'your opponent, put it into your hand, and put the rest into your waiting room."'
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "AfterClimaxPhase")
        self.assertEqual(rl_defs[0]["effects"], [{"Draw": {"count": 0}}])
        self.assertEqual(
            rl_defs[0]["conditions"],
            {
                "requires_approx_effects": True,
                "climax_area": {"side": "SelfSide", "card_ids": []},
            },
        )

    def test_parse_cost_nested_gate_icon_in_cost(self):
        text = (
            "【AUTO】 [Put 1 climax with [GATE] in its trigger icon from your hand into your waiting room] "
            "When this card attacks, you may pay the cost. If you do, this card gets +2000 power and the "
            "following ability until end of turn. \"【AUTO】 When this card's battle opponent becomes "
            '【REVERSE】, you may deal 1 damage to your opponent."'
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(len(none_defs), 1)
        self.assertEqual(none_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(
            none_defs[0]["effects"][0], {"AddPower": {"amount": 2000, "duration_turn": True}}
        )
        self.assertEqual(list(none_defs[0]["effects"][1].keys()), ["GrantAbilityDef"])
        self.assertEqual(none_defs[0]["cost"]["discard_from_hand"], 1)

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(rl_defs[0]["conditions"], {})
        self.assertEqual(rl_defs[0]["cost"]["discard_from_hand"], 1)

    def test_continuous_all_other_two_named_power_exact(self):
        stats = AbilityParseStats()
        text = '【CONT】 All of your other "Alpha" and "Beta" get +1000 power.'
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Alpha": [1], "Beta": [2]},
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["targets"], ["SelfStage"])
        self.assertEqual(
            defs[0]["effects"],
            [
                {
                    "ConditionalAddPower": {
                        "amount": 1000,
                        "turn": None,
                        "zone_count": None,
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": False,
                        "exclude_source": True,
                        "target_ids": [1, 2],
                    }
                }
            ],
        )

    def test_continuous_all_other_name_fragment_dual_power_exact(self):
        stats = AbilityParseStats()
        text = '【CONT】 All of your other characters with "Alpha" or "Beta" in its card name get +1500 power.'
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Alpha Card": [10], "Beta Card": [20]},
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["targets"], ["SelfStage"])
        target_ids = defs[0]["effects"][0]["ConditionalAddPower"]["target_ids"]
        self.assertEqual(target_ids, [10, 20])

    def test_continuous_if_has_count_other_dual_trait_power_exact(self):
        stats = AbilityParseStats()
        text = "【CONT】 If you have 2 or more other 《TraitA》 or 《TraitB》 characters, this card gets +1500 power."
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_to_ids={"TraitA": [100], "TraitB": [101]},
            source_card_id=100,
        )
        self.assertEqual(len(defs), 1)
        zone_count = defs[0]["effects"][0]["ConditionalAddPower"]["zone_count"]
        self.assertEqual(zone_count["value"], 3)
        self.assertEqual(zone_count["card_ids"], [100, 101])

    def test_auto_attack_named_climax_team_power_exact(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 When this card attacks, if a card named "Named CX" is in your climax area, '
            "all of your characters get +1000 power until end of turn."
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Named CX": [303]},
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(defs[0]["targets"], ["SelfStage"])
        self.assertEqual(
            defs[0]["conditions"],
            {"climax_area": {"side": "SelfSide", "card_ids": [303]}},
        )

    def test_auto_on_play_draw_then_self_power_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, draw up to 1 card, "
            "and this card gets +1500 power until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[0]["targets"], ["This"])
        self.assertEqual(defs[0]["effect_optional"], [True])
        self.assertEqual(
            defs[0]["effects"],
            [
                {"Draw": {"count": 1}},
                {"AddPower": {"amount": 1500, "duration_turn": True}},
            ],
        )

    def test_auto_on_play_draw_discard_then_bounce_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, draw 1 card, choose 1 card in your hand, "
            "put it into your waiting room, choose up to 1 of your opponent's characters, and return it to their hand."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 2)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[1]["timing"], "OnPlay")
        self.assertEqual(defs[0]["effects"][0], {"Draw": {"count": 1}})
        self.assertEqual(defs[1]["effects"], ["MoveToHand"])
        self.assertEqual(defs[1]["targets"], ["OppStage"])
        self.assertEqual(defs[1]["effect_optional"], [True])

    def test_activated_reduce_power_opponent_center_exact(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 [【REST】 this card] Choose 1 character in your opponent's center stage, "
            "and that character gets -2000 power until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["targets"], ["OppFrontRow"])
        self.assertEqual(
            defs[0]["effects"],
            [{"AddPower": {"amount": -2000, "duration_turn": True}}],
        )

    def test_continuous_same_name_plus_named_deck_rule_counts_as_supported(self):
        stats = AbilityParseStats()
        text = '【CONT】 You can put up to 8 cards with the same card name as this card and cards named "Extra Name" in your deck.'
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(defs, [])
        self.assertEqual(stats.parsed_lines, 1)

    def test_auto_paid_on_climax_add_soul_with_a_climax(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [Return this card to your hand] When a climax is placed on your climax area, you may pay the cost. "
            "If you do, choose 1 of your characters, and that character gets +1 soul until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "AfterClimaxPhase")
        self.assertEqual(defs[0]["cost"]["return_self_to_hand"], True)
        self.assertEqual(defs[0]["effects"], [{"AddSoul": {"amount": 1, "duration_turn": True}}])

    def test_auto_accelerate_begin_climax_paid_self_power_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 Accelerate [Put the top card of your deck into your clock] "
            "At the beginning of your climax phase, you may pay the cost. If you do, this card gets +2000 power until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "BeginClimaxPhase")
        self.assertEqual(defs[0]["cost"]["clock_from_deck_top"], 1)
        self.assertEqual(
            defs[0]["effects"], [{"AddPower": {"amount": 2000, "duration_turn": True}}]
        )

    def test_continuous_assist_front_row_power_level_of_target_wording(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 Assist All of your characters in front of this card get +X power. "
            "X is equal to the level of that character ×500."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(
            defs[0]["effects"],
            [{"AddPowerByLevel": {"multiplier": 500, "duration_turn": False}}],
        )

    def test_counter_trait_requirement_approx_approx(self):
        text = (
            "【COUNTER】 If you do not have a 《Deadly Sin》 character, this card cannot be played from your hand. "
            "Look at up to 4 cards from the top of your deck, choose up to 1 《Deadly Sin》 character from among them, "
            "reveal it to your opponent, put it into your hand, and put the rest into your waiting room."
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Event", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, counter = parse_abilities(text, "Event", rl_stats, approx_profile="approx")
        self.assertTrue(counter)
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "Counter")
        self.assertEqual(rl_defs[0]["conditions"], {"requires_approx_effects": True})

    def test_activated_self_add_soul_exact(self):
        stats = AbilityParseStats()
        text = "【ACT】 [(1)] This card gets +1 soul until end of turn."
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["cost"]["stock"], 1)
        self.assertEqual(defs[0]["targets"], ["This"])
        self.assertEqual(defs[0]["effects"], [{"AddSoul": {"amount": 1, "duration_turn": True}}])

    def test_continuous_assist_front_row_trait_power_by_level_exact(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 Assist All of your 《Music》 characters in front of this card get +X power. "
            "X is equal to that character's level ×500."
        )
        _, defs, _ = parse_abilities(text, "Character", stats, trait_map={"Music": 3})
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["target_trait"], 3)
        self.assertEqual(
            defs[0]["effects"],
            [{"AddPowerByLevel": {"multiplier": 500, "duration_turn": False}}],
        )

    def test_continuous_assist_front_row_trait_level_power_exact(self):
        stats = AbilityParseStats()
        text = "【CONT】 Assist All of your 《Music》 characters in front of this card get +1 level and +500 power."
        _, defs, _ = parse_abilities(text, "Character", stats, trait_map={"Music": 8})
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["target_trait"], 8)
        self.assertEqual(
            defs[0]["effects"],
            [
                {"AddLevel": {"amount": 1, "duration_turn": False}},
                {"AddPower": {"amount": 500, "duration_turn": False}},
            ],
        )

    def test_continuous_power_per_other_named_exact(self):
        stats = AbilityParseStats()
        text = '【CONT】 This card gets +500 power for each of your other cards named "Test Name".'
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Test Name": [101, 202]},
            source_card_id=101,
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["targets"], ["This"])
        self.assertEqual(defs[0]["effects"][0]["ConditionalAddPower"]["amount"], 500)

    def test_auto_on_play_choose_trait_team_power_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, choose 1 of your 《Music》 characters, "
            "and that character gets +1500 power until end of turn."
        )
        _, defs, _ = parse_abilities(text, "Character", stats, trait_map={"Music": 11})
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[0]["target_trait"], 11)
        self.assertEqual(defs[0]["target_limit"], 1)
        self.assertEqual(
            defs[0]["effects"],
            [{"AddPower": {"amount": 1500, "duration_turn": True}}],
        )

    def test_auto_other_attacker_matches_another_of_your_trait_exact(self):
        stats = AbilityParseStats()
        text = "【AUTO】 When another of your 《Music》 character attacks, this card gets +1000 power until end of turn."
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            trait_to_ids={"Music": [201, 202]},
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OtherAttackDeclaration")
        self.assertEqual(
            defs[0]["effects"],
            [
                {
                    "AddPowerIfOtherAttackerMatches": {
                        "amount": 1000,
                        "duration_turn": True,
                        "attacker_card_ids": [201, 202],
                    }
                }
            ],
        )

    def test_auto_direct_attack_choose_other_power_exact(self):
        stats = AbilityParseStats()
        text = "【AUTO】 When this card direct attacks, choose 1 of your other characters, and that character gets +1500 power until end of turn."
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "AttackDeclaration")
        self.assertEqual(defs[0]["targets"], ["SelfStage"])
        self.assertEqual(
            defs[0]["effects"],
            [
                {
                    "TimedConditionalAddPower": {
                        "amount": 1500,
                        "duration_turn": True,
                        "turn": None,
                        "zone_count": None,
                        "require_source_marker": False,
                        "per_source_marker": False,
                        "per_zone_count": False,
                        "exclude_source": True,
                        "target_ids": [],
                    }
                }
            ],
        )

    def test_auto_paid_on_play_search_salvage_generic_approx_approx(self):
        text = (
            "【AUTO】 [Put 1 card from your hand into your clock] When this card is placed on the stage from your hand, "
            "you may pay the cost. If you do, look at up to X cards from the top of your deck, choose up to 1 card from among them, "
            "put it into your hand, and put the rest into your waiting room. X is equal to the number of other 《Roselia》 characters you have."
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertEqual(rl_defs[0]["timing"], "OnPlay")
        self.assertEqual(rl_defs[0]["conditions"], {"requires_approx_effects": True})
        self.assertEqual(rl_defs[0]["cost"]["clock_from_hand"], 1)

    def test_continuous_following_generic_approx_approx(self):
        text = (
            "【CONT】 If you have 2 or more other 《Hello, Happy World!》 characters, this card gets the following ability. "
            '"【AUTO】 Encore [(3)]"'
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertTrue(rl_defs[0]["conditions"]["requires_approx_effects"])

    def test_act_brainstorm_search_to_hand_approx_approx_draw(self):
        text = (
            "【ACT】 Brainstorm [(1) 【REST】 this card] Flip over 4 cards from the top of your deck, and put them into your waiting room. "
            "For each climax revealed among those cards, search your deck for up to 1 《Game》 character, reveal it to your opponent, "
            "put it into your hand, and shuffle your deck."
        )
        none_stats = AbilityParseStats()
        _, none_defs, _ = parse_abilities(text, "Character", none_stats, approx_profile="strict")
        self.assertEqual(none_defs, [])

        rl_stats = AbilityParseStats()
        _, rl_defs, _ = parse_abilities(text, "Character", rl_stats, approx_profile="approx")
        self.assertEqual(len(rl_defs), 1)
        self.assertTrue(rl_defs[0]["conditions"]["requires_approx_effects"])
        self.assertEqual(
            rl_defs[0]["effects"],
            [{"Brainstorm": {"reveal_count": 4, "per_climax": 1, "mode": "Draw"}}],
        )

    def test_parser_v2_fallback_following_ability_grant_marks_rule_and_approx(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If you have 2 or more other 《Hello, Happy World!》 characters, this card gets the following ability. "
            '"【AUTO】 Encore [(3)]"'
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            parser_version="v2",
            approx_profile="approx",
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["kind"], "Continuous")
        self.assertTrue(defs[0]["conditions"]["requires_approx_effects"])
        source_rule_id = defs[0]["conditions"].get("source_rule_id")
        if source_rule_id is not None:
            self.assertTrue(source_rule_id.startswith("parser_v2."))

    def test_parser_v2_fallback_use_this_cards_ability_line(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 When you use this card\'s "Backup", choose 1 of your characters in battle, '
            "and that character gets the following ability until end of turn. "
            '"【AUTO】 At the beginning of your next draw phase, draw 1 card."'
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            parser_version="v2",
            approx_profile="approx",
        )
        self.assertEqual(len(defs), 1)
        self.assertIn(defs[0]["timing"], ["UseAct", None])
        self.assertTrue(defs[0]["conditions"]["requires_approx_effects"])
        source_rule_id = defs[0]["conditions"].get("source_rule_id")
        if source_rule_id is not None:
            self.assertEqual(source_rule_id, "parser_v2.auto.use_this_cards_ability_generic")

    def test_parser_v2_fallback_brainstorm_custom_action(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 Brainstorm [(1) 【REST】 this card] Flip over 4 cards from the top of your deck, and put them into your waiting room. "
            'For each climax revealed among those cards, perform the following action. "Choose 1 of your characters, and that character gets +1000 power until end of turn."'
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            parser_version="v2",
            approx_profile="approx",
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["kind"], "Activated")
        self.assertEqual(
            defs[0]["effects"],
            [{"Brainstorm": {"reveal_count": 4, "per_climax": 1, "mode": "Draw"}}],
        )
        self.assertTrue(defs[0]["conditions"]["requires_approx_effects"])
        source_rule_id = defs[0]["conditions"].get("source_rule_id")
        if source_rule_id is not None:
            self.assertEqual(source_rule_id, "parser_v2.act.brainstorm_custom_action_generic")

    def test_parser_v2_fallback_broad_quoted_grant_line(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 All of your opponent's characters get \"【AUTO】 At the beginning of your encore step, "
            'put the top card of your deck into your waiting room."'
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            parser_version="v2",
            approx_profile="approx",
        )
        self.assertEqual(len(defs), 1)
        self.assertIn(defs[0]["targets"], [[], ["OppStage"]])
        self.assertTrue(defs[0]["conditions"]["requires_approx_effects"])
        source_rule_id = defs[0]["conditions"].get("source_rule_id")
        if source_rule_id is not None:
            self.assertTrue(source_rule_id.startswith("parser_v2.quoted_grant"))

    def test_change_begin_encore_step_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 Change [(2) Put this card into your waiting room] At the beginning of your encore step, "
            'if this card is 【REST】, you may pay the cost. If you do, choose a card named "Future Idol" '
            "in your waiting room, and put it on the stage position that this card was on."
        )
        _, defs, _ = parse_abilities(
            text,
            "Character",
            stats,
            name_to_ids={"Future Idol": [321]},
            approx_profile="strict",
            parser_version="v2",
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "BeginEncoreStep")
        self.assertEqual(
            defs[0]["effects"], [{"MoveWaitingRoomCardToSourceSlot": {"target_ids": [321]}}]
        )
        self.assertEqual(defs[0]["cost"]["stock"], 2)

    def test_paid_draw_on_damage_dealt_canceled_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1)] When damage dealt by this card is canceled, you may pay the cost. "
            "If you do, draw 1 card."
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "DamageDealtCanceled")
        self.assertEqual(defs[0]["effects"], [{"Draw": {"count": 1}}])
        self.assertEqual(defs[0]["cost"]["stock"], 1)
        self.assertEqual(defs[0]["effect_optional"], [True])

    def test_paid_on_play_or_leave_stage_salvage_exact(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1) Put 1 card from your hand into your waiting room] "
            "When this card is placed on the stage from your hand or put into your waiting room from the stage, "
            "you may pay the cost. If you do, choose 1 character in your waiting room, and return it to your hand."
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(defs[0]["targets"], ["SelfWaitingRoom"])
        self.assertEqual(defs[0]["target_card_type"], "Character")
        self.assertEqual(defs[0]["target_limit"], 1)
        self.assertEqual(defs[0]["cost"]["stock"], 1)
        self.assertEqual(defs[0]["cost"]["discard_from_hand"], 1)

    def test_on_play_or_by_act_heal_top_clock_exact(self):
        stats = AbilityParseStats()
        text = (
            '【AUTO】 When this card is placed on the stage from your hand or by the 【ACT】 effect of "Partner", '
            "you may put the top card of your clock into your waiting room."
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[0]["effects"], ["MoveToWaitingRoom"])
        self.assertEqual(defs[0]["targets"], ["SelfClock"])
        self.assertEqual(defs[0]["effect_optional"], [True])
        self.assertEqual(defs[0]["target_limit"], 1)

    def test_cost_steps_ordered_for_converter_cost_parsing(self):
        stats = AbilityParseStats()
        text = (
            "【ACT】 [(1) 【REST】 this card & Put 1 card from your hand into your waiting room & "
            "Put this card into your waiting room] Choose 1 character in your waiting room, and "
            "return it to your hand."
        )
        _, defs, _ = parse_abilities(text, "Character", stats)
        self.assertEqual(len(defs), 1)
        self.assertEqual(
            defs[0]["cost"]["cost_steps"],
            [
                {"PayStock": {"count": 1}},
                {"RestSelf": {}},
                {"DiscardFromHand": {"count": 1}},
                {"MoveSelfToWaitingRoom": {}},
            ],
        )

    def test_converter_cost_step_order_preserves_repeated_steps(self):
        cost, supported, _ = parse_cost_v1(
            "【ACT】 [Reveal 1 card from your hand & Put 1 card from your hand into your waiting room & "
            "Reveal 1 card from your hand] Draw 1 card."
        )
        self.assertTrue(supported)
        self.assertEqual(cost["reveal_from_hand"], 2)
        self.assertEqual(cost["discard_from_hand"], 1)
        self.assertEqual(
            cost["step_order"],
            ["RevealFromHand", "DiscardFromHand", "RevealFromHand"],
        )

    def test_cost_steps_ordered_for_parser_v2_paid_salvage(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1) Put 1 card from your hand into your waiting room] "
            "When this card is placed on the stage from your hand or put into your waiting room from the stage, "
            "you may pay the cost. If you do, choose 1 character in your waiting room, and return it to your hand."
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(
            defs[0]["cost"]["cost_steps"],
            [
                {"PayStock": {"count": 1}},
                {"DiscardFromHand": {"count": 1}},
            ],
        )

    def test_parser_v2_non_default_cost_step_order_sets_step_order(self):
        cost, supported, _ = parse_cost_v2(
            "【ACT】 [Reveal 1 card from your hand & Put 1 card from your hand into your waiting room] Draw 1 card."
        )
        self.assertTrue(supported)
        self.assertEqual(cost["discard_from_hand"], 1)
        self.assertEqual(cost["reveal_from_hand"], 1)
        self.assertEqual(cost["step_order"], ["RevealFromHand", "DiscardFromHand"])

    def test_parser_v2_cost_step_order_preserves_repeated_steps(self):
        cost, supported, _ = parse_cost_v2(
            "【ACT】 [Reveal 1 card from your hand & Put 1 card from your hand into your waiting room & "
            "Reveal 1 card from your hand] Draw 1 card."
        )
        self.assertTrue(supported)
        self.assertEqual(cost["reveal_from_hand"], 2)
        self.assertEqual(cost["discard_from_hand"], 1)
        self.assertEqual(
            cost["step_order"],
            ["RevealFromHand", "DiscardFromHand", "RevealFromHand"],
        )

    def test_parser_v2_parse_cost_single_top_card_clock_supported(self):
        cost, supported, _ = parse_cost_v2(
            "【AUTO】 [Put the top card of your deck into your clock] This card gets +1000 power until end of turn."
        )
        self.assertTrue(supported)
        self.assertEqual(cost["clock_from_deck_top"], 1)

    def test_parser_v2_exact_frontal_attacked_look_top(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is frontal attacked, look at the top card of your deck, and "
            "put it on the top of your deck or into your waiting room."
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["effects"], ["LookTopCardTopOrWaitingRoom"])
        self.assertEqual(defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(
            defs[0]["conditions"].get("source_rule_id"),
            "parser_v2.auto.frontal_attacked_look_top_exact",
        )

    def test_parser_v2_exact_facing_opponent_quoted_restriction(self):
        stats = AbilityParseStats()
        text = (
            '【CONT】 The character facing this card gets "【CONT】 This card cannot move to another '
            'position of the stage."'
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["effects"], ["FacingOpponentCannotMoveStagePosition"])
        self.assertEqual(defs[0]["targets"], ["OppFrontRow"])
        self.assertEqual(
            defs[0]["conditions"].get("source_rule_id"),
            "parser_v2.cont.facing_opponent_quoted_restriction_exact",
        )

    def test_parser_v2_exact_on_play_power_plus_quoted_grant(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 When this card is placed on the stage from your hand, this card gets +4500 power "
            "and the following ability until the end of your opponent's next turn. "
            '"【CONT】 During this card\'s battle, all players cannot play "Backup" from their hands."'
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(list(defs[0]["effects"][0].keys()), ["GrantAbilityDef"])
        grant = defs[0]["effects"][0]["GrantAbilityDef"]
        self.assertEqual(grant["duration"], "UntilEndOfOpponentsNextTurn")
        self.assertEqual(
            grant["ability"]["effects"],
            [
                {"AddPower": {"amount": 4500, "duration_turn": False}},
                {"CannotPlayBackupFromHand": {"duration_turn": False}},
            ],
        )

    def test_parser_v2_exact_marker_power_and_following(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 If there is a marker underneath this card, this card gets +1500 power and the "
            'following ability. "【CONT】 This card cannot side attack."'
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(
            defs[0]["effects"][0]["ConditionalAddPower"]["require_source_marker"],
            True,
        )
        self.assertEqual(
            defs[0]["effects"][1]["ConditionalCannotSideAttack"]["require_source_marker"],
            True,
        )

    def test_parser_v2_exact_marker_power_and_soul(self):
        stats = AbilityParseStats()
        text = "【CONT】 If there is a marker underneath this card, this card gets +1000 power and +1 soul."
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(
            defs[0]["effects"][0]["ConditionalAddPower"]["require_source_marker"],
            True,
        )
        self.assertEqual(
            defs[0]["effects"][1]["ConditionalAddSoul"]["require_source_marker"],
            True,
        )
        self.assertEqual(
            defs[0]["effects"][1]["ConditionalAddSoul"]["amount"],
            1,
        )

    def test_parser_v2_exact_experience_with_following(self):
        stats = AbilityParseStats()
        text = (
            "【CONT】 Experience During your turn, if the total level of the cards in your level is 2 or higher, "
            "this card gets +2000 power and the following ability. "
            '"【CONT】 This card cannot side attack."'
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        cond_power = defs[0]["effects"][0]["ConditionalAddPower"]
        self.assertEqual(cond_power["turn"], "SelfTurn")
        self.assertEqual(cond_power["zone_count"]["cmp"], "AtLeastLevelSum")
        self.assertEqual(cond_power["zone_count"]["value"], 2)
        cannot_side = defs[0]["effects"][1]["ConditionalCannotSideAttack"]
        self.assertEqual(cannot_side["turn"], "SelfTurn")
        self.assertEqual(cannot_side["zone_count"]["value"], 2)

    def test_parser_v2_exact_paid_on_play_salvage_with_trailing_buff(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1) Put 1 card from your hand into your waiting room] When this card is placed on the stage "
            "from your hand, you may pay the cost. If you do, choose 1 《Music》 character in your waiting room, "
            "return it to your hand, choose 1 of your other 《Music》 characters, and that character gets +1000 "
            "power until end of turn."
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(defs[0]["timing"], "OnPlay")
        self.assertEqual(defs[0]["target_limit"], 1)
        self.assertEqual(defs[0]["cost"]["stock"], 1)
        self.assertEqual(defs[0]["cost"]["discard_from_hand"], 1)

    def test_parser_v2_exact_paid_on_play_search_to_hand(self):
        stats = AbilityParseStats()
        text = (
            "【AUTO】 [(1)] When this card is placed on the stage from your hand, you may pay the cost. "
            "If you do, search your deck for up to 1 《Music》 character, reveal it to your opponent, "
            "put it into your hand, and shuffle your deck."
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertEqual(defs[0]["effects"], ["MoveToHand"])
        self.assertEqual(defs[0]["targets"], ["SelfDeckTop"])
        self.assertEqual(defs[0]["target_limit"], 1)
        self.assertEqual(defs[0]["cost"]["stock"], 1)

    def test_parser_v2_exact_climax_placed_following_grant(self):
        text = (
            "【AUTO】 When your climax is placed on your climax area, choose 1 of your characters, and that "
            'character gets the following ability until end of turn. "【CONT】 This card cannot side attack."'
        )
        outcome = parse_line_v2(text, "Character", allow_approx_rules=False, emit_trace=False)
        self.assertTrue(outcome.matched)
        self.assertEqual(
            outcome.rule_match.rule_id,
            "parser_v2.auto.climax_placed_buff_or_following_exact",
        )
        self.assertIsNotNone(outcome.ability_def)
        self.assertEqual(outcome.ability_def["timing"], "AfterClimaxPhase")
        self.assertEqual(outcome.ability_def["target_limit"], 1)
        self.assertEqual(list(outcome.ability_def["effects"][0].keys()), ["GrantAbilityDef"])
        self.assertEqual(
            outcome.ability_def["effects"][0]["GrantAbilityDef"]["ability"]["effects"],
            [{"CannotSideAttack": {"duration_turn": False}}],
        )

    def test_parser_v2_exact_on_reverse_self_move_variants(self):
        stats = AbilityParseStats()
        memory_text = (
            "【AUTO】 When this card becomes 【REVERSE】 in battle, put this card into your memory."
        )
        _, memory_defs, _ = parse_abilities(
            memory_text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(memory_defs), 1)
        self.assertEqual(memory_defs[0]["timing"], "OnReverse")
        self.assertEqual(memory_defs[0]["effects"], ["MoveToMemory"])

        stats = AbilityParseStats()
        bottom_text = "【AUTO】 When this card becomes 【REVERSE】 in battle, put this card at the bottom of your deck."
        _, bottom_defs, _ = parse_abilities(
            bottom_text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(bottom_defs), 1)
        self.assertEqual(bottom_defs[0]["effects"], ["MoveToDeckBottom"])

    def test_parser_v2_exact_terminal_win_lose_effects(self):
        stats = AbilityParseStats()
        win_text = "【AUTO】 You win the game."
        _, win_defs, _ = parse_abilities(
            win_text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(win_defs), 1)
        self.assertEqual(win_defs[0]["effects"], [{"SetTerminalOutcome": {"outcome": "WinSelf"}}])

        stats = AbilityParseStats()
        lose_text = "【AUTO】 If there are no cards in your deck, you lose the game."
        _, lose_defs, _ = parse_abilities(
            lose_text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(lose_defs), 1)
        self.assertEqual(
            lose_defs[0]["effects"], [{"SetTerminalOutcome": {"outcome": "WinOpponent"}}]
        )

    def test_parser_v2_strict_following_fallback_is_exact_not_approx(self):
        stats = AbilityParseStats()
        text = (
            '【CONT】 All of your other "Test Name" get the following ability. '
            '"【AUTO】 Encore [(3)]"'
        )
        _, defs, _ = parse_abilities(
            text, "Character", stats, approx_profile="strict", parser_version="v2"
        )
        self.assertEqual(len(defs), 1)
        self.assertNotIn("requires_approx_effects", defs[0]["conditions"])
        self.assertEqual(defs[0]["effects"], [{"Draw": {"count": 0}}])


if __name__ == "__main__":
    unittest.main()
