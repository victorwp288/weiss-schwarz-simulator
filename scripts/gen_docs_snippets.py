#!/usr/bin/env python3
"""Generate and validate in-repo markdown snippets derived from code.

This script is intentionally dependency-free and deterministic:

- no timestamps
- stable ordering
- code is the single source of truth

It updates regions wrapped in:

  <!-- GENERATED:<ID>:START -->
  <!-- GENERATED:<ID>:END -->
"""

from __future__ import annotations

import argparse
import ast
import difflib
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

PY_INIT = ROOT / "python" / "weiss_sim" / "__init__.py"
PY_PYI = ROOT / "python" / "weiss_sim" / "weiss_sim.pyi"

DOC_API_GUIDE = ROOT / "docs" / "python_api.md"
DOC_API_REF = ROOT / "docs" / "python_api_reference.md"
DOC_PPO = ROOT / "docs" / "tutorials" / "ppo.md"
DOC_IMPALA = ROOT / "docs" / "tutorials" / "impala_vtrace.md"

EX_PPO = ROOT / "python" / "examples" / "ppo_torch.py"
EX_IMPALA = ROOT / "python" / "examples" / "impala_vtrace.py"


@dataclass(frozen=True)
class ModuleInfo:
    path: Path
    source: str
    tree: ast.Module
    defs: dict[str, ast.AST]
    classes: dict[str, ast.ClassDef]


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise SystemExit(f"Failed to read {path}: {exc}") from exc


def write_text(path: Path, content: str) -> None:
    try:
        path.write_text(content, encoding="utf-8")
    except OSError as exc:
        raise SystemExit(f"Failed to write {path}: {exc}") from exc


def start_marker(marker_id: str) -> str:
    return f"<!-- GENERATED:{marker_id}:START -->"


def end_marker(marker_id: str) -> str:
    return f"<!-- GENERATED:{marker_id}:END -->"


def replace_region(content: str, *, marker_id: str, body: str) -> str:
    start = start_marker(marker_id)
    end = end_marker(marker_id)
    if start not in content or end not in content:
        raise SystemExit(f"Missing markers for {marker_id}")
    before, rest = content.split(start, 1)
    _, after = rest.split(end, 1)
    body_norm = body.rstrip("\n")
    if body_norm:
        replacement = f"{start}\n{body_norm}\n{end}"
    else:
        replacement = f"{start}\n{end}"
    return before + replacement + after


def extract_region(content: str, *, marker_id: str) -> str:
    start = start_marker(marker_id)
    end = end_marker(marker_id)
    if start not in content or end not in content:
        raise SystemExit(f"Missing markers for {marker_id}")
    _, rest = content.split(start, 1)
    body, _ = rest.split(end, 1)
    return body.strip("\n")


def parse_module(path: Path) -> ModuleInfo:
    source = read_text(path)
    tree = ast.parse(source, filename=str(path))
    defs: dict[str, ast.AST] = {}
    classes: dict[str, ast.ClassDef] = {}
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            defs[node.name] = node
            if isinstance(node, ast.ClassDef):
                classes[node.name] = node
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            defs[node.target.id] = node
        elif isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name):
                    defs[target.id] = node
    return ModuleInfo(path=path, source=source, tree=tree, defs=defs, classes=classes)


def parse_all_exports(init_mod: ModuleInfo) -> list[str]:
    for node in init_mod.tree.body:
        if not isinstance(node, ast.Assign):
            continue
        if not any(isinstance(t, ast.Name) and t.id == "__all__" for t in node.targets):
            continue
        if not isinstance(node.value, (ast.List, ast.Tuple)):
            raise SystemExit("__all__ must be a list/tuple literal")
        out: list[str] = []
        for elt in node.value.elts:
            if not isinstance(elt, ast.Constant) or not isinstance(elt.value, str):
                raise SystemExit("__all__ must contain only string literals")
            out.append(elt.value)
        return out
    raise SystemExit("__all__ not found in python/weiss_sim/__init__.py")


def _src(expr: ast.AST | None, *, source: str) -> str | None:
    if expr is None:
        return None
    seg = ast.get_source_segment(source, expr)
    if seg is not None and seg.strip():
        return seg.strip()
    try:
        return ast.unparse(expr).strip()
    except Exception:
        return None


def _format_arg(arg: ast.arg, *, default: str | None, source: str, include_types: bool) -> str:
    name = arg.arg
    ann = _src(arg.annotation, source=source) if include_types else None
    out = name if ann is None else f"{name}: {ann}"
    if default is not None:
        out = f"{out} = {default}"
    return out


def _format_signature(
    node: ast.FunctionDef | ast.AsyncFunctionDef,
    *,
    source: str,
    include_types: bool,
    for_markdown: bool,
) -> str:
    args = node.args
    posonly = list(args.posonlyargs)
    normal = list(args.args)
    kwonly = list(args.kwonlyargs)
    defaults = list(args.defaults)
    kw_defaults = list(args.kw_defaults)

    # Align defaults with the last N positional args (posonly+normal).
    pos_args = posonly + normal
    pos_defaults: list[str | None] = [None] * len(pos_args)
    if defaults:
        for i, d in enumerate(defaults, start=len(pos_args) - len(defaults)):
            pos_defaults[i] = _src(d, source=source)

    kw_defaults_src = [_src(d, source=source) if d is not None else None for d in kw_defaults]

    parts: list[str] = []
    for i, arg in enumerate(posonly):
        parts.append(
            _format_arg(arg, default=pos_defaults[i], source=source, include_types=include_types)
        )
    if posonly:
        parts.append("/")
    for i, arg in enumerate(normal, start=len(posonly)):
        parts.append(
            _format_arg(arg, default=pos_defaults[i], source=source, include_types=include_types)
        )

    if args.vararg is not None:
        var_ann = _src(args.vararg.annotation, source=source) if include_types else None
        var_name = args.vararg.arg
        parts.append(f"*{var_name}" if var_ann is None else f"*{var_name}: {var_ann}")
    elif kwonly:
        parts.append("*")

    for i, arg in enumerate(kwonly):
        parts.append(
            _format_arg(arg, default=kw_defaults_src[i], source=source, include_types=include_types)
        )

    if args.kwarg is not None:
        kw_ann = _src(args.kwarg.annotation, source=source) if include_types else None
        kw_name = args.kwarg.arg
        parts.append(f"**{kw_name}" if kw_ann is None else f"**{kw_name}: {kw_ann}")

    returns = _src(node.returns, source=source) if include_types else None
    params = ", ".join(parts)
    header = f"def {node.name}({params})"
    if returns:
        header += f" -> {returns}"
    if for_markdown:
        header += ": ..."
    return header


def format_def_block(
    node: ast.FunctionDef | ast.AsyncFunctionDef,
    *,
    source: str,
    include_types: bool,
) -> str:
    # Prefer a readable multiline format for large signatures.
    args_count = (
        len(node.args.posonlyargs)
        + len(node.args.args)
        + len(node.args.kwonlyargs)
        + (1 if node.args.vararg is not None else 0)
        + (1 if node.args.kwarg is not None else 0)
    )
    one_line = _format_signature(
        node, source=source, include_types=include_types, for_markdown=True
    )
    if args_count <= 6 and len(one_line) <= 100:
        return one_line

    # Multiline pretty signature (still deterministic).
    args = node.args
    posonly = list(args.posonlyargs)
    normal = list(args.args)
    kwonly = list(args.kwonlyargs)
    defaults = list(args.defaults)
    kw_defaults = list(args.kw_defaults)

    pos_args = posonly + normal
    pos_defaults: list[str | None] = [None] * len(pos_args)
    if defaults:
        for i, d in enumerate(defaults, start=len(pos_args) - len(defaults)):
            pos_defaults[i] = _src(d, source=source)
    kw_defaults_src = [_src(d, source=source) if d is not None else None for d in kw_defaults]

    lines: list[str] = [f"def {node.name}("]
    indent = " " * 4
    for i, arg in enumerate(posonly):
        lines.append(
            f"{indent}{_format_arg(arg, default=pos_defaults[i], source=source, include_types=include_types)},"
        )
    if posonly:
        lines.append(f"{indent}/,")
    for i, arg in enumerate(normal, start=len(posonly)):
        lines.append(
            f"{indent}{_format_arg(arg, default=pos_defaults[i], source=source, include_types=include_types)},"
        )

    if args.vararg is not None:
        var_ann = _src(args.vararg.annotation, source=source) if include_types else None
        var_name = args.vararg.arg
        lines.append(
            f"{indent}*{var_name}," if var_ann is None else f"{indent}*{var_name}: {var_ann},"
        )
    elif kwonly:
        lines.append(f"{indent}*,")

    for i, arg in enumerate(kwonly):
        lines.append(
            f"{indent}{_format_arg(arg, default=kw_defaults_src[i], source=source, include_types=include_types)},"
        )
    if args.kwarg is not None:
        kw_ann = _src(args.kwarg.annotation, source=source) if include_types else None
        kw_name = args.kwarg.arg
        lines.append(
            f"{indent}**{kw_name}," if kw_ann is None else f"{indent}**{kw_name}: {kw_ann},"
        )

    returns = _src(node.returns, source=source) if include_types else None
    closing = ")"
    if returns:
        closing += f" -> {returns}"
    closing += ": ..."
    lines.append(closing)
    return "\n".join(lines)


def render_make_call_signature(api_mod: ModuleInfo) -> str:
    node = api_mod.defs.get("make")
    if not isinstance(node, ast.FunctionDef):
        raise SystemExit("python/weiss_sim/api.py must define make()")

    args = node.args
    if args.args or args.posonlyargs:
        raise SystemExit("make() is expected to be keyword-only (def make(*, ...))")

    if len(args.kwonlyargs) != len(args.kw_defaults):
        raise SystemExit("make() kwonlyargs/kw_defaults length mismatch")

    lines = ["```python", "weiss_sim.make("]
    for arg, default_node in zip(args.kwonlyargs, args.kw_defaults, strict=True):
        default = _src(default_node, source=api_mod.source)
        if default is None:
            raise SystemExit(f"make() missing default for {arg.arg}")
        lines.append(f"    {arg.arg}={default},")
    lines.append(")")
    lines.append("```")
    return "\n".join(lines)


def _first_paragraph(doc: str | None) -> str | None:
    if not doc:
        return None
    text = doc.strip()
    if not text:
        return None
    parts = [p.strip() for p in text.split("\n\n") if p.strip()]
    return parts[0] if parts else None


def _is_overload_decorator(dec: ast.expr) -> bool:
    if isinstance(dec, ast.Name) and dec.id == "overload":
        return True
    if isinstance(dec, ast.Attribute) and dec.attr == "overload":
        return True
    return False


def _overload_defs(mod: ModuleInfo, name: str) -> list[ast.FunctionDef]:
    nodes = [n for n in mod.tree.body if isinstance(n, ast.FunctionDef) and n.name == name]
    overloads = [n for n in nodes if any(_is_overload_decorator(d) for d in n.decorator_list)]
    return overloads


def _class_fields(cls: ast.ClassDef, *, source: str) -> list[str]:
    out: list[str] = []
    for node in cls.body:
        if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            name = node.target.id
            ann = _src(node.annotation, source=source) or "object"
            default = _src(node.value, source=source) if node.value is not None else None
            if default is None:
                out.append(f"{name}: {ann}")
            else:
                out.append(f"{name}: {ann} = {default}")
    return out


def _class_methods(cls: ast.ClassDef) -> list[ast.FunctionDef]:
    methods: list[ast.FunctionDef] = []
    for node in cls.body:
        if isinstance(node, ast.FunctionDef) and not node.name.startswith("_"):
            methods.append(node)
    methods.sort(key=lambda m: m.name)
    return methods


def _find_cards_namespace(catalog_mod: ModuleInfo) -> ast.ClassDef:
    node = catalog_mod.classes.get("_CardsNamespace")
    if node is None:
        raise SystemExit("python/weiss_sim/catalog.py must define _CardsNamespace")
    return node


def render_api_reference(
    *,
    exports: list[str],
    pyi_mod: ModuleInfo,
    py_modules: list[ModuleInfo],
) -> str:
    py_defs: dict[str, tuple[ModuleInfo, ast.AST]] = {}
    for mod in py_modules:
        for name, node in mod.defs.items():
            py_defs.setdefault(name, (mod, node))

    pyi_defs: dict[str, ast.AST] = dict(pyi_mod.defs)

    def resolve_symbol(name: str) -> tuple[str, str]:
        """Return (kind, markdown) for a public symbol."""
        if name == "cards":
            catalog = next((m for m in py_modules if m.path.name == "catalog.py"), None)
            if catalog is None:
                raise SystemExit("catalog.py module is required to render cards")
            cards_ns = _find_cards_namespace(catalog)
            methods = _class_methods(cards_ns)
            lines = ["`cards` is a namespace object exposed as `weiss_sim.cards`.", "", "Methods:"]
            for m in methods:
                sig = format_def_block(m, source=catalog.source, include_types=True)
                lines.append(f"- `{sig}`")
            return ("cards", "\n".join(lines))

        if name in pyi_defs:
            node = pyi_defs[name]
            if isinstance(node, ast.FunctionDef):
                sig = format_def_block(node, source=pyi_mod.source, include_types=True)
                return ("function", f"```python\n{sig}\n```")
            if isinstance(node, ast.ClassDef):
                doc = _first_paragraph(ast.get_docstring(node))
                fields = _class_fields(node, source=pyi_mod.source)
                methods = _class_methods(node)
                lines: list[str] = []
                if doc:
                    lines.append(doc)
                    lines.append("")
                if fields:
                    lines.append("Fields:")
                    for f in fields:
                        lines.append(f"- `{f}`")
                    lines.append("")
                if methods:
                    lines.append("Methods:")
                    for m in methods:
                        sig = format_def_block(m, source=pyi_mod.source, include_types=True)
                        lines.append(f"- `{sig}`")
                body = "\n".join(lines).strip()
                return ("class", body)
            if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
                ann = _src(node.annotation, source=pyi_mod.source) or "object"
                return ("const", f"`{name}: {ann}`")

        if name in py_defs:
            mod, node = py_defs[name]
            if isinstance(node, ast.FunctionDef):
                overloads = _overload_defs(mod, name)
                if overloads:
                    sigs = "\n\n".join(
                        format_def_block(o, source=mod.source, include_types=True)
                        for o in overloads
                    )
                    sig = f"```python\n{sigs}\n```"
                else:
                    sig = f"```python\n{format_def_block(node, source=mod.source, include_types=True)}\n```"
                doc = _first_paragraph(ast.get_docstring(node))
                if doc:
                    return ("function", f"{doc}\n\n{sig}")
                return ("function", sig)
            if isinstance(node, ast.ClassDef):
                doc = _first_paragraph(ast.get_docstring(node))
                fields = _class_fields(node, source=mod.source)
                methods = _class_methods(node)
                lines: list[str] = []
                if doc:
                    lines.append(doc)
                    lines.append("")
                if fields:
                    lines.append("Fields:")
                    for f in fields:
                        lines.append(f"- `{f}`")
                    lines.append("")
                if methods:
                    lines.append("Methods:")
                    for m in methods:
                        sig = format_def_block(m, source=mod.source, include_types=True)
                        lines.append(f"- `{sig}`")
                return ("class", "\n".join(lines).strip())
            if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
                ann = _src(node.annotation, source=mod.source) or "object"
                value = _src(node.value, source=mod.source)
                if value is None:
                    return ("value", f"`{name}: {ann}`")
                return ("value", f"`{name}: {ann} = {value}`")
            if isinstance(node, ast.Assign):
                value = _src(node.value, source=mod.source) or "..."
                return ("value", f"`{name} = {value}`")

        return ("unknown", "_(not found in scanned modules; update generator inputs)_")

    def details_block(title: str, body: str) -> str:
        body = body.strip()
        if not body:
            body = "_(no details)_"
        return f"<details>\n<summary><code>{title}</code></summary>\n\n{body}\n\n</details>"

    # Section definitions.
    constants = [
        "ACTION_SPACE_SIZE",
        "ACTION_META_WIDTH",
        "ACTION_META_UNUSED",
        "OBS_LEN",
        "SPEC_HASH",
        "POLICY_VERSION",
        "PASS_ACTION_ID",
        "ACTOR_NONE",
        "DECISION_KIND_NONE",
        "__version__",
    ]
    high_level = ["make", "fast", "inspect", "WeissEnv", "ResetBatch", "StepBatch", "LegalActions"]
    low_level = [
        "EnvPool",
        "EnvPoolBuffers",
        "EnvPoolTrajectoryBuffers",
        "BatchOutMinimal",
        "BatchOutMinimalI16",
        "BatchOutMinimalI16LegalIds",
        "BatchOutMinimalNoMask",
        "BatchOutTrajectory",
        "BatchOutTrajectoryI16",
        "BatchOutTrajectoryI16LegalIds",
        "BatchOutTrajectoryNoMask",
        "BatchOutDebug",
        "make_pool",
        "make_batch_out_debug",
        "RlStep",
        "reset_rl",
        "step_rl",
        "step_rl_select_from_logits",
        "step_rl_sample_from_logits",
        "step_rl_sample_from_logits_with_logp",
        "pass_action_id_for_decision_kind",
    ]
    specs = [
        "observation_spec_json",
        "action_spec_json",
        "decode_action_id",
        "decode_factorized_action_id",
        "encode_factorized_action",
        "build_info",
        "spec_bundle",
        "export_spec_bundle",
        "export_card_table",
        "db_info",
    ]
    cards = [
        "cards",
        "DeckInput",
        "DeckBuilder",
        "CurriculumOverrides",
        "EndConditionOverrides",
        "CardRef",
        "DeckValidationIssue",
        "DeckValidationReport",
    ]
    league = [
        "MatchRecord",
        "AgentSummary",
        "FirstPlayerBiasSummary",
        "ClockGreedSummary",
        "round_robin_schedule",
        "sample_population_schedule",
        "records_from_step",
        "summarize_records",
        "summarize_first_player_bias",
        "summarize_clock_greed_from_replay",
        "rank_agents",
    ]
    errors = [
        "WeissSimError",
        "DeckSpecError",
        "CardLookupError",
        "DeckValidationError",
        "ConfigConflictError",
        "DbMismatchError",
    ]

    # Validate that we cover all exports deterministically.
    covered = set(constants + high_level + low_level + specs + cards + league + errors)
    missing = [n for n in exports if n not in covered]
    if missing:
        raise SystemExit(f"Uncategorized exports in API reference generator: {missing}")

    lines: list[str] = []
    lines.append("## Constants & versions")
    lines.append("")
    lines.append(
        "These values are compatibility boundaries; see [RL Contract](rl_contract.md) for the checksum table."
    )
    lines.append("")
    for name in constants:
        _, body = resolve_symbol(name)
        lines.append(f"- {body}")

    def emit_section(title: str, names: list[str]) -> None:
        lines.append("")
        lines.append(f"## {title}")
        for name in names:
            kind, body = resolve_symbol(name)
            lines.append("")
            lines.append(f"### `{name}`")
            if kind == "class":
                lines.append(details_block(name, body))
            else:
                lines.append(body)

    emit_section("Specs & metadata", specs)
    emit_section("High-level API", high_level)
    emit_section("Low-level API", low_level)
    emit_section("Cards & decks", cards)
    emit_section("League utilities", league)
    emit_section("Errors", errors)
    return "\n".join(lines).rstrip() + "\n"


def render_embedded_script(path: Path) -> str:
    text = read_text(path).rstrip("\n")
    try:
        label = path.relative_to(ROOT).as_posix()
    except ValueError:
        label = path.as_posix()
    return "\n".join(
        [
            "```python",
            f"# {label}",
            text,
            "```",
        ]
    )


def unified_diff(a: str, b: str, *, fromfile: str, tofile: str) -> str:
    return "".join(
        difflib.unified_diff(
            a.splitlines(keepends=True),
            b.splitlines(keepends=True),
            fromfile=fromfile,
            tofile=tofile,
        )
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate/validate code-derived markdown snippets."
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="Rewrite generated regions in-place.")
    mode.add_argument("--check", action="store_true", help="Fail if generated regions are stale.")
    args = parser.parse_args()

    init_mod = parse_module(PY_INIT)
    pyi_mod = parse_module(PY_PYI)
    exports = parse_all_exports(init_mod)

    py_modules = [
        init_mod,
        parse_module(ROOT / "python" / "weiss_sim" / "api.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "runner.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "types.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "_buffers.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "rl.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "catalog.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "deck_builder.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "league.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "errors.py"),
        parse_module(ROOT / "python" / "weiss_sim" / "config_types.py"),
    ]

    expected = {
        (DOC_API_GUIDE, "MAKE_SIGNATURE"): render_make_call_signature(py_modules[1]),
        (DOC_API_REF, "PYTHON_API_REFERENCE"): render_api_reference(
            exports=exports, pyi_mod=pyi_mod, py_modules=py_modules
        ),
        (DOC_PPO, "PPO_TORCH_SCRIPT"): render_embedded_script(EX_PPO),
        (DOC_IMPALA, "IMPALA_VTRACE_SCRIPT"): render_embedded_script(EX_IMPALA),
    }

    failures: list[str] = []
    for (path, marker_id), body in expected.items():
        content = read_text(path)
        updated = replace_region(content, marker_id=marker_id, body=body)
        if args.write:
            if updated != content:
                write_text(path, updated)
        else:
            current_region = extract_region(content, marker_id=marker_id)
            expected_region = extract_region(updated, marker_id=marker_id)
            if current_region != expected_region:
                diff = unified_diff(
                    current_region + "\n",
                    expected_region + "\n",
                    fromfile=f"{path}:{marker_id} (current)",
                    tofile=f"{path}:{marker_id} (expected)",
                )
                failures.append(diff or f"{path}: {marker_id} differs")

    if failures:
        for item in failures:
            sys.stdout.write(item)
            if not item.endswith("\n"):
                sys.stdout.write("\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
