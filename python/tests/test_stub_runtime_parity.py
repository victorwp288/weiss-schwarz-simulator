from __future__ import annotations

import ast
import inspect
from pathlib import Path

import numpy as np
import weiss_sim


_STUB_PATH = Path(__file__).resolve().parents[1] / "weiss_sim" / "weiss_sim.pyi"
_ENVPOOL_KEY_METHODS = (
    "step_select_from_logits_into",
    "step_sample_from_logits_into",
    "auto_reset_on_error_codes_into",
    "auto_reset_on_error_codes_into_nomask",
)


def _parse_stub_module() -> ast.Module:
    return ast.parse(_STUB_PATH.read_text(encoding="utf-8"))


def _find_class(module: ast.Module, class_name: str) -> ast.ClassDef:
    for node in module.body:
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            return node
    raise AssertionError(f"class {class_name!r} not found in {_STUB_PATH}")


def _stub_method_param_names(class_node: ast.ClassDef, method_name: str) -> list[str]:
    for node in class_node.body:
        if isinstance(node, ast.FunctionDef) and node.name == method_name:
            args = node.args
            names = [arg.arg for arg in args.posonlyargs]
            names.extend(arg.arg for arg in args.args)
            names.extend(arg.arg for arg in args.kwonlyargs)
            if args.vararg is not None:
                names.append(args.vararg.arg)
            if args.kwarg is not None:
                names.append(args.kwarg.arg)
            return names
    raise AssertionError(f"method {method_name!r} not found in stub class {class_node.name!r}")


def _stub_annotated_fields(class_node: ast.ClassDef) -> set[str]:
    fields: set[str] = set()
    for node in class_node.body:
        if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            fields.add(node.target.id)
    return fields


def _runtime_param_names(callable_obj) -> list[str]:
    return [param.name for param in inspect.signature(callable_obj).parameters.values()]


def _instantiate_batch_out_debug():
    sig = inspect.signature(weiss_sim.BatchOutDebug)
    required_args: list[int] = []
    for param in sig.parameters.values():
        if param.default is inspect._empty:
            required_args.append(2)
    return weiss_sim.BatchOutDebug(*required_args)


def test_envpool_key_method_signatures_match_stub():
    assert _STUB_PATH.exists(), f"missing stub file: {_STUB_PATH}"
    module = _parse_stub_module()
    envpool_stub = _find_class(module, "EnvPool")

    for method_name in _ENVPOOL_KEY_METHODS:
        runtime_names = _runtime_param_names(getattr(weiss_sim.EnvPool, method_name))
        stub_names = _stub_method_param_names(envpool_stub, method_name)
        assert runtime_names == stub_names


def test_batch_out_debug_init_signature_matches_stub():
    assert _STUB_PATH.exists(), f"missing stub file: {_STUB_PATH}"
    module = _parse_stub_module()
    batch_out_stub = _find_class(module, "BatchOutDebug")

    runtime_names = _runtime_param_names(weiss_sim.BatchOutDebug)
    stub_names = _stub_method_param_names(batch_out_stub, "__init__")
    assert runtime_names == stub_names[1:]


def test_batch_out_debug_array_fields_match_stub():
    assert _STUB_PATH.exists(), f"missing stub file: {_STUB_PATH}"
    module = _parse_stub_module()
    batch_out_stub = _find_class(module, "BatchOutDebug")
    expected_fields = _stub_annotated_fields(batch_out_stub)

    runtime = _instantiate_batch_out_debug()
    runtime_fields: set[str] = set()
    for name in dir(runtime):
        if name.startswith("_"):
            continue
        try:
            value = getattr(runtime, name)
        except Exception:
            continue
        if isinstance(value, np.ndarray):
            runtime_fields.add(name)

    assert runtime_fields == expected_fields
