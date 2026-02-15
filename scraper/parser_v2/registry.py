from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional

from .models import RULE_MODES, RULE_MODE_APPROX

try:
    import yaml
except Exception:  # pragma: no cover - exercised in environments without PyYAML
    yaml = None


DEFAULT_RULES_DIR = Path(__file__).resolve().parents[1] / "rules_v2"
DEFAULT_COMPILED_PATH = Path(__file__).resolve().parents[1] / "rules_v2_compiled.json"


@dataclass(frozen=True)
class CompiledRule:
    id: str
    description: str
    pattern: str
    priority: int
    mode: str
    match_on: str = "body"
    effect_mode: str = "noop_draw"
    timing: Optional[str] = None
    kind: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)
    regex: re.Pattern[str] = field(init=False, repr=False, compare=False)

    def __post_init__(self) -> None:
        object.__setattr__(self, "regex", re.compile(self.pattern, re.I))


def _load_yaml_like(path: Path) -> Any:
    text = path.read_text(encoding="utf-8")
    if yaml is not None:
        return yaml.safe_load(text)
    return json.loads(text)


def _iter_rule_docs(rules_dir: Path) -> Iterable[tuple[Path, Any]]:
    for path in sorted(
        [*rules_dir.glob("*.yaml"), *rules_dir.glob("*.yml")],
        key=lambda p: p.name.casefold(),
    ):
        yield path, _load_yaml_like(path)


def _normalize_rule(raw: Dict[str, Any], source: Path) -> Dict[str, Any]:
    if not isinstance(raw, dict):
        raise ValueError(f"rule in {source} must be a mapping")
    for key in ("id", "pattern", "priority"):
        if key not in raw:
            raise ValueError(f"rule in {source} missing required key '{key}'")

    mode = str(raw.get("mode", RULE_MODE_APPROX)).strip().lower()
    if mode not in RULE_MODES:
        raise ValueError(f"rule {raw.get('id')} in {source} has unsupported mode '{mode}'")

    priority = raw.get("priority")
    if not isinstance(priority, int):
        raise ValueError(f"rule {raw.get('id')} in {source} priority must be an integer")

    rule: Dict[str, Any] = {
        "id": str(raw["id"]).strip(),
        "description": str(raw.get("description", "")).strip(),
        "pattern": str(raw["pattern"]).strip(),
        "priority": priority,
        "mode": mode,
        "match_on": str(raw.get("match_on", "body")).strip().lower(),
        "effect_mode": str(raw.get("effect_mode", "noop_draw")).strip(),
        "timing": raw.get("timing"),
        "kind": raw.get("kind"),
        "metadata": dict(raw.get("metadata") or {}),
        "_source_file": source.name,
    }
    if not rule["id"]:
        raise ValueError(f"rule id in {source} must not be empty")
    if not rule["pattern"]:
        raise ValueError(f"rule {rule['id']} in {source} has empty pattern")
    return rule


def load_rules_from_yaml(rules_dir: Optional[Path] = None) -> List[Dict[str, Any]]:
    base = rules_dir or DEFAULT_RULES_DIR
    if not base.exists():
        return []

    raw_rules: List[Dict[str, Any]] = []
    for path, doc in _iter_rule_docs(base):
        if doc is None:
            continue
        if isinstance(doc, dict):
            items = doc.get("rules", [])
        elif isinstance(doc, list):
            items = doc
        else:
            raise ValueError(f"unsupported rules document in {path}")
        if not isinstance(items, list):
            raise ValueError(f"'rules' in {path} must be a list")
        for item in items:
            raw_rules.append(_normalize_rule(item, path))

    seen_ids: Dict[str, str] = {}
    for rule in raw_rules:
        existing = seen_ids.get(rule["id"])
        if existing is not None:
            raise ValueError(
                f"duplicate rule id '{rule['id']}' in {rule['_source_file']} and {existing}"
            )
        seen_ids[rule["id"]] = str(rule["_source_file"])

    raw_rules.sort(key=lambda r: (-int(r["priority"]), str(r["id"]), str(r["_source_file"])))
    return raw_rules


def build_compiled_payload(rules_dir: Optional[Path] = None) -> Dict[str, Any]:
    rules = load_rules_from_yaml(rules_dir=rules_dir)
    for rule in rules:
        rule.pop("_source_file", None)
    return {
        "version": 1,
        "rules": rules,
    }


def write_compiled_rulepack(
    output_path: Optional[Path] = None,
    rules_dir: Optional[Path] = None,
) -> Path:
    payload = build_compiled_payload(rules_dir=rules_dir)
    out_path = output_path or DEFAULT_COMPILED_PATH
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")
    return out_path


def _to_compiled_rule(raw: Dict[str, Any]) -> CompiledRule:
    return CompiledRule(
        id=str(raw["id"]),
        description=str(raw.get("description", "")),
        pattern=str(raw["pattern"]),
        priority=int(raw["priority"]),
        mode=str(raw["mode"]).lower(),
        match_on=str(raw.get("match_on", "body")).lower(),
        effect_mode=str(raw.get("effect_mode", "noop_draw")),
        timing=raw.get("timing"),
        kind=raw.get("kind"),
        metadata=dict(raw.get("metadata") or {}),
    )


def load_compiled_rules(
    compiled_path: Optional[Path] = None,
    rules_dir: Optional[Path] = None,
) -> List[CompiledRule]:
    path = compiled_path or DEFAULT_COMPILED_PATH
    if path.exists():
        data = json.loads(path.read_text(encoding="utf-8"))
        raw_rules = data.get("rules", [])
    else:
        payload = build_compiled_payload(rules_dir=rules_dir)
        raw_rules = payload.get("rules", [])
    return [_to_compiled_rule(rule) for rule in raw_rules]
