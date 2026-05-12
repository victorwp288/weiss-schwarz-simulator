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
_BATCH_OUT_CLASSES = (
    "BatchOutMinimal",
    "BatchOutMinimalI16",
    "BatchOutMinimalI16LegalIds",
    "BatchOutMinimalI16LegalIdsNoMeta",
    "BatchOutMinimalNoMask",
    "BatchOutTrajectory",
    "BatchOutTrajectoryI16",
    "BatchOutTrajectoryI16LegalIds",
    "BatchOutTrajectoryNoMask",
    "BatchOutDebug",
)
_PUBLIC_CONSTANTS = (
    "OBS_LEN",
    "OBS_ENCODING_VERSION",
    "ACTION_ENCODING_VERSION",
    "ACTION_SPACE_SIZE",
    "ACTION_META_WIDTH",
    "ACTION_META_UNUSED",
    "REWARD_COMPONENT_WIDTH",
    "LEGAL_ACTION_CONTEXT_V1_WIDTH",
    "LEGAL_ACTION_CONTEXT_UNUSED",
    "SPEC_HASH",
    "POLICY_VERSION",
    "PASS_ACTION_ID",
    "ACTOR_NONE",
    "DECISION_KIND_NONE",
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
    return _instantiate_batch_out("BatchOutDebug")


def _instantiate_batch_out(class_name: str):
    cls = getattr(weiss_sim, class_name)
    sig = inspect.signature(cls)
    args: list[int] = []
    for param in sig.parameters.values():
        if param.default is inspect._empty:
            args.append(2)
    return cls(*args)


def _stub_class_method_names(class_node: ast.ClassDef) -> list[str]:
    return [
        node.name
        for node in class_node.body
        if isinstance(node, ast.FunctionDef) and not node.name.startswith("__")
    ]


def test_envpool_runtime_method_signatures_match_stub():
    assert _STUB_PATH.exists(), f"missing stub file: {_STUB_PATH}"
    module = _parse_stub_module()
    envpool_stub = _find_class(module, "EnvPool")

    for method_name in _stub_class_method_names(envpool_stub):
        runtime_method = getattr(weiss_sim.EnvPool, method_name)
        runtime_names = _runtime_param_names(runtime_method)
        stub_names = _stub_method_param_names(envpool_stub, method_name)
        assert runtime_names == stub_names or runtime_names == stub_names[1:], method_name


def test_all_batch_out_runtime_fields_match_stub():
    assert _STUB_PATH.exists(), f"missing stub file: {_STUB_PATH}"
    module = _parse_stub_module()

    for class_name in _BATCH_OUT_CLASSES:
        stub_class = _find_class(module, class_name)
        expected_fields = _stub_annotated_fields(stub_class)
        runtime = _instantiate_batch_out(class_name)
        runtime_fields: set[str] = set()
        for name in dir(runtime):
            if name.startswith("_"):
                continue
            try:
                value = getattr(runtime, name)
            except Exception:
                continue
            if isinstance(value, np.ndarray) or isinstance(value, int):
                runtime_fields.add(name)
        assert runtime_fields == expected_fields, class_name


def test_public_stub_classes_are_exported():
    assert _STUB_PATH.exists(), f"missing stub file: {_STUB_PATH}"
    module = _parse_stub_module()
    class_names = [node.name for node in module.body if isinstance(node, ast.ClassDef)]

    missing = [class_name for class_name in class_names if not hasattr(weiss_sim, class_name)]
    assert missing == []


def test_public_constants_are_exported_and_stubbed():
    assert _STUB_PATH.exists(), f"missing stub file: {_STUB_PATH}"
    module = _parse_stub_module()
    stub_constants = {
        node.target.id
        for node in module.body
        if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name)
    }

    for name in _PUBLIC_CONSTANTS:
        assert hasattr(weiss_sim, name), name
        assert name in stub_constants, name


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
