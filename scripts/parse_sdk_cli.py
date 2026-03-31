#!/usr/bin/env python3
"""Parse upstream BigQuery Agent Analytics SDK cli.py and the dcx Rust
CLI source, then generate a compatibility contract.

Usage:
    python3 scripts/parse_sdk_cli.py \
        --cli-py tests/fixtures/upstream_sdk_latest/cli.py \
        --cli-rs src/cli.rs \
        --out-json tests/fixtures/analytics_sdk_contract.json \
        --out-md docs/analytics_sdk_contract.md \
        [--upstream-sha <SHA>]
"""

from __future__ import annotations

import argparse
import ast
import json
import re
import sys
from datetime import date
from pathlib import Path


# ── dcx Rust source extraction ───────────────────────────────────────


def parse_cli_rs(source: str) -> dict:
    """Extract the dcx analytics CLI surface from src/cli.rs.

    Parses clap derive macros to extract:
    - Global flags from the Cli struct
    - Analytics subcommands and their per-command flags
    - ViewsCommand subcommands
    - EvaluatorType enum values
    """
    global_flags = _parse_clap_struct_flags(source, "pub struct Cli")
    analytics_commands = _parse_analytics_commands(source, global_flags)
    evaluators = _parse_evaluator_enum(source)
    return {
        "commands": analytics_commands,
        "evaluators": evaluators,
        "global_flags": global_flags,
    }


def _parse_clap_struct_flags(source: str, struct_header: str) -> dict[str, dict]:
    """Extract flags from a clap derive struct (e.g. Cli)."""
    # Find the struct body
    idx = source.find(struct_header)
    if idx < 0:
        return {}
    brace_start = source.index("{", idx)
    depth = 0
    brace_end = brace_start
    for i in range(brace_start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                brace_end = i
                break
    body = source[brace_start + 1 : brace_end]
    return _extract_flags_from_body(body)


def _extract_flags_from_body(body: str) -> dict[str, dict]:
    """Extract flags from a block of clap field definitions."""
    flags: dict[str, dict] = {}
    lines = body.split("\n")
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        # Collect #[arg(...)] attributes — may span multiple lines
        arg_attrs: list[str] = []
        doc_lines: list[str] = []
        while i < len(lines):
            line = lines[i].strip()
            if line.startswith("///"):
                doc_lines.append(line[3:].strip())
                i += 1
            elif line.startswith("#[arg("):
                # Collect potentially multi-line attribute
                attr_text = line
                while not attr_text.rstrip().endswith(")]"):
                    i += 1
                    if i < len(lines):
                        attr_text += " " + lines[i].strip()
                arg_attrs.append(attr_text)
                i += 1
            elif line.startswith("#[command("):
                # Skip command attributes (subcommand markers)
                i += 1
            elif re.match(r"(?:pub\s+)?\w+\s*:", line):
                break
            else:
                i += 1

        if i >= len(lines):
            break
        line = lines[i].strip()
        i += 1

        # Match both struct fields (pub name: Type) and enum variant fields (name: Type)
        m = re.match(r"(?:pub\s+)?(\w+)\s*:\s*(.+?),?\s*$", line)
        if not m:
            doc_lines = []
            arg_attrs = []
            continue

        # Skip subcommand fields
        if m.group(1) == "command":
            doc_lines = []
            arg_attrs = []
            continue

        # Parse: [pub] field_name: Type,
        m = re.match(r"(?:pub\s+)?(\w+)\s*:\s*(.+?),?\s*$", line)
        if not m:
            doc_lines = []
            arg_attrs = []
            continue

        field_name = m.group(1)
        field_type = m.group(2).strip().rstrip(",")

        # Skip the subcommand field
        if field_name == "command":
            doc_lines = []
            arg_attrs = []
            continue

        cli_flag = "--" + field_name.replace("_", "-")

        flag_info: dict = {
            "type": _rust_type_to_str(field_type),
            "help": " ".join(doc_lines) if doc_lines else None,
        }

        # Parse all arg attributes
        combined_attr = " ".join(arg_attrs)
        flag_info["required"] = _is_required(field_type, combined_attr)
        explicit_default = _extract_attr_value(combined_attr, "default_value")
        if explicit_default is not None:
            flag_info["default"] = explicit_default
        elif field_type.strip() == "bool":
            flag_info["default"] = False  # clap bool flags default to false
        else:
            flag_info["default"] = None
        env = _extract_attr_value(combined_attr, "env")
        if env:
            flag_info["env"] = env
        if "global = true" in combined_attr:
            flag_info["global"] = True
        if "hide = true" in combined_attr:
            flag_info["hidden"] = True

        flags[cli_flag] = flag_info
        doc_lines = []
        arg_attrs = []

    return flags


def _parse_analytics_commands(source: str, global_flags: dict) -> dict[str, dict]:
    """Extract analytics subcommands from AnalyticsCommand and ViewsCommand enums."""
    commands: dict[str, dict] = {}

    # Parse AnalyticsCommand enum
    analytics_variants = _parse_enum_variants(source, "pub enum AnalyticsCommand")
    for variant_name, variant_body in analytics_variants.items():
        if variant_name == "Views":
            # Views is a nested subcommand, handled separately
            continue
        cmd_name = _variant_to_cli_name(variant_name)
        per_cmd_flags = _extract_flags_from_body(variant_body) if variant_body else {}
        # Merge global + per-command
        merged = {}
        merged.update(global_flags)
        merged.update(per_cmd_flags)
        commands[cmd_name] = {"flags": merged}

    # Parse ViewsCommand enum
    views_variants = _parse_enum_variants(source, "pub enum ViewsCommand")
    for variant_name, variant_body in views_variants.items():
        cmd_name = "views " + _variant_to_cli_name(variant_name)
        per_cmd_flags = _extract_flags_from_body(variant_body) if variant_body else {}
        merged = {}
        merged.update(global_flags)
        merged.update(per_cmd_flags)
        commands[cmd_name] = {"flags": merged}

    return commands


def _parse_enum_variants(source: str, enum_header: str) -> dict[str, str | None]:
    """Parse a clap Subcommand enum into {VariantName: body_or_None}."""
    idx = source.find(enum_header)
    if idx < 0:
        return {}
    brace_start = source.index("{", idx)
    depth = 0
    brace_end = brace_start
    for i in range(brace_start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                brace_end = i
                break
    body = source[brace_start + 1 : brace_end]

    variants: dict[str, str | None] = {}
    # Match variants: doc comments, then Name { ... } or Name,
    # Use a state machine approach
    lines = body.split("\n")
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        # Skip doc comments and attributes
        if line.startswith("///") or line.startswith("#["):
            i += 1
            continue
        # Look for variant name
        m = re.match(r"(\w+)\s*\{", line)
        if m:
            variant_name = m.group(1)
            # Collect the brace-delimited body
            start_i = i
            depth = 0
            variant_lines: list[str] = []
            for j in range(i, len(lines)):
                for ch in lines[j]:
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                variant_lines.append(lines[j])
                if depth == 0:
                    i = j + 1
                    break
            # Strip outer braces
            variant_body = "\n".join(variant_lines)
            first_brace = variant_body.index("{")
            last_brace = variant_body.rindex("}")
            variant_body = variant_body[first_brace + 1 : last_brace]
            variants[variant_name] = variant_body
            continue
        # Unit variant: Name,
        m2 = re.match(r"(\w+)\s*,", line)
        if m2:
            variants[m2.group(1)] = None
            i += 1
            continue
        i += 1

    return variants


def _parse_evaluator_enum(source: str) -> list[str]:
    """Extract EvaluatorType enum values."""
    idx = source.find("pub enum EvaluatorType")
    if idx < 0:
        return []
    brace_start = source.index("{", idx)
    brace_end = source.index("}", brace_start)
    body = source[brace_start + 1 : brace_end]
    values = []
    for line in body.split("\n"):
        line = line.strip().rstrip(",")
        if line and not line.startswith("//") and not line.startswith("#"):
            # Convert PascalCase to kebab-case
            kebab = re.sub(r"(?<=[a-z])(?=[A-Z])", "-", line).lower()
            values.append(kebab)
    return values


def _variant_to_cli_name(name: str) -> str:
    """Convert PascalCase enum variant to kebab-case CLI command name."""
    return re.sub(r"(?<=[a-z])(?=[A-Z])", "-", name).lower()


def _rust_type_to_str(t: str) -> str:
    t = t.strip()
    if t.startswith("Option<"):
        inner = t[7:-1].strip()
        return f"optional<{_rust_type_to_str(inner)}>"
    if t in ("String", "&str"):
        return "string"
    if t in ("f64", "f32"):
        return "float"
    if t in ("u32", "u64", "i32", "i64", "usize"):
        return "int"
    if t == "bool":
        return "bool"
    if t == "OutputFormat":
        return "enum"
    if t == "EvaluatorType":
        return "enum"
    return t.lower()


def _is_required(field_type: str, attr_text: str) -> bool:
    """A clap field is required if it's not Option<> and has no default_value."""
    if field_type.strip().startswith("Option<"):
        return False
    if "default_value" in attr_text:
        return False
    if field_type.strip() == "bool":
        # bool flags default to false in clap
        return False
    return True


def _extract_attr_value(attr_text: str, key: str) -> str | None:
    """Extract a quoted value from a clap #[arg(...)] attribute."""
    # Match: key = "value"
    m = re.search(rf'{key}\s*=\s*"([^"]*)"', attr_text)
    if m:
        return m.group(1)
    return None


# ── AST extraction from upstream cli.py ──────────────────────────────


def _python_name_to_cli(name: str) -> str:
    """Convert Python identifier to CLI flag name: foo_bar -> --foo-bar."""
    return "--" + name.replace("_", "-")


def _get_string(node: ast.expr) -> str | None:
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    return None


def _extract_typer_option(call: ast.Call) -> dict:
    """Extract metadata from a typer.Option(...) or typer.Argument(...) call."""
    info: dict = {}
    is_argument = False

    if isinstance(call.func, ast.Attribute):
        if call.func.attr == "Argument":
            is_argument = True
    info["positional"] = is_argument

    # Check if first positional arg is ... (Ellipsis = required)
    if call.args:
        first = call.args[0]
        if isinstance(first, ast.Constant) and first.value is ...:
            info["required"] = True
        elif isinstance(first, ast.Constant):
            info["default"] = first.value
            info["required"] = False

    for kw in call.keywords:
        if kw.arg == "envvar" and isinstance(kw.value, ast.Constant):
            info["env"] = kw.value.value
        elif kw.arg == "help":
            if isinstance(kw.value, ast.Constant):
                info["help"] = kw.value.value
            elif isinstance(kw.value, ast.JoinedStr):
                info["help"] = "(f-string)"
        elif kw.arg == "default" and isinstance(kw.value, ast.Constant):
            info["default"] = kw.value.value
            info["required"] = False

    return info


def _resolve_type_annotation(ann: ast.expr) -> str:
    """Best-effort type string from annotation."""
    if isinstance(ann, ast.Name):
        return ann.id.lower()
    if isinstance(ann, ast.Subscript):
        if isinstance(ann.value, ast.Name) and ann.value.id == "Optional":
            inner = _resolve_type_annotation(ann.slice)
            return f"optional<{inner}>"
    if isinstance(ann, ast.Attribute):
        return ann.attr.lower()
    return "unknown"


def _extract_command(func: ast.FunctionDef, parent_name: str = "") -> dict:
    """Extract command metadata from a decorated function."""
    cmd_name = func.name.replace("_", "-")
    for dec in func.decorator_list:
        if isinstance(dec, ast.Call):
            if dec.args:
                s = _get_string(dec.args[0])
                if s:
                    cmd_name = s

    if parent_name:
        cmd_name = f"{parent_name} {cmd_name}"

    flags: dict[str, dict] = {}
    args: dict[str, dict] = {}

    non_self_args = [a for a in func.args.args if a.arg != "self"]
    defaults = func.args.defaults
    pad = len(non_self_args) - len(defaults)
    for i, param in enumerate(non_self_args):
        if param.annotation is None:
            continue
        cli_flag = _python_name_to_cli(param.arg)
        type_str = _resolve_type_annotation(param.annotation)

        flag_info: dict = {"type": type_str}
        default_idx = i - pad
        if default_idx >= 0 and default_idx < len(defaults):
            default_node = defaults[default_idx]
            if isinstance(default_node, ast.Call):
                option_info = _extract_typer_option(default_node)
                flag_info.update(option_info)
            elif isinstance(default_node, ast.Constant):
                flag_info["default"] = default_node.value
                flag_info["required"] = False

        if flag_info.get("positional"):
            args[param.arg] = flag_info
        else:
            if param.arg == "fmt":
                cli_flag = "--format"
            flags[cli_flag] = flag_info

    return {"name": cmd_name, "flags": flags, "args": args}


def _extract_dict_keys(node: ast.Dict) -> list[str]:
    keys = []
    for k in node.keys:
        if isinstance(k, ast.Constant) and isinstance(k.value, str):
            keys.append(k.value)
    return keys


def parse_cli_py(source: str) -> dict:
    """Parse cli.py and return structured upstream surface."""
    tree = ast.parse(source)

    commands: list[dict] = []
    code_evaluators: list[str] = []
    llm_judges: list[str] = []
    env_vars: list[str] = []

    for node in ast.walk(tree):
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name):
                    if target.id == "_CODE_EVALUATORS" and isinstance(node.value, ast.Dict):
                        code_evaluators = _extract_dict_keys(node.value)
                    elif target.id == "_LLM_JUDGES" and isinstance(node.value, ast.Dict):
                        llm_judges = _extract_dict_keys(node.value)

    views_commands: list[dict] = []
    for node in ast.iter_child_nodes(tree):
        if isinstance(node, ast.FunctionDef):
            for dec in node.decorator_list:
                if isinstance(dec, ast.Call) and isinstance(dec.func, ast.Attribute):
                    if dec.func.attr == "command":
                        if isinstance(dec.func.value, ast.Name):
                            if dec.func.value.id == "app":
                                commands.append(_extract_command(node))
                            elif dec.func.value.id == "views_app":
                                views_commands.append(
                                    _extract_command(node, parent_name="views")
                                )
                elif isinstance(dec, ast.Attribute):
                    if isinstance(dec.value, ast.Name):
                        if dec.value.id == "app" and dec.attr == "command":
                            commands.append(_extract_command(node))

    commands.extend(views_commands)

    for cmd in commands:
        for flag_info in cmd.get("flags", {}).values():
            if "env" in flag_info:
                env_vars.append(flag_info["env"])
    env_vars = sorted(set(env_vars))

    return {
        "commands": commands,
        "code_evaluators": code_evaluators,
        "llm_judges": llm_judges,
        "env_vars": env_vars,
    }


# ── Known intentional divergences ────────────────────────────────────

KNOWN_DIVERGENCES: list[dict[str, str]] = [
    {
        "item": "--table vs --table-id",
        "dcx": "--table",
        "sdk": "--table-id",
        "reason": "dcx uses shorter flag name; both default to agent_events",
    },
    {
        "item": "--location default",
        "dcx": "US",
        "sdk": "None (optional)",
        "reason": "dcx defaults to US for BigQuery; SDK leaves it to the client library",
    },
    {
        "item": "drift --min-coverage",
        "dcx": "present (default 0.8)",
        "sdk": "absent",
        "reason": "dcx extension for CI gate workflows",
    },
    {
        "item": "drift --exit-code",
        "dcx": "present",
        "sdk": "absent",
        "reason": "dcx extension for CI gate workflows",
    },
    {
        "item": "infrastructure error exit code",
        "dcx": "not used",
        "sdk": "exit 2",
        "reason": "dcx uses generic error handling; SDK distinguishes eval-fail (1) from infra-error (2)",
    },
    {
        "item": "env var prefix",
        "dcx": "DCX_PROJECT / DCX_DATASET",
        "sdk": "BQ_AGENT_PROJECT / BQ_AGENT_DATASET",
        "reason": "dcx uses its own namespace; same semantics",
    },
    {
        "item": "--sanitize flag",
        "dcx": "present (Model Armor)",
        "sdk": "absent",
        "reason": "dcx-only feature for response sanitization",
    },
    {
        "item": "evaluator name spelling",
        "dcx": "error-rate (kebab-case)",
        "sdk": "error_rate (snake_case)",
        "reason": "dcx follows CLI kebab-case convention; may add aliases for SDK names",
    },
]


# ── Contract generation ──────────────────────────────────────────────


# Canonical flag name mapping: SDK name -> dcx name (where they differ)
FLAG_RENAMES = {
    "--table-id": "--table",
}


def _resolve_dcx_flag(sdk_flag: str, dcx_flags: dict) -> tuple[str | None, str]:
    """Find the matching dcx flag for an SDK flag.

    Returns (dcx_flag_name, match_type) where match_type is:
    - "exact_name": same flag name
    - "renamed": known rename (e.g. --table-id -> --table)
    - None: not found
    """
    if sdk_flag in dcx_flags:
        return sdk_flag, "exact_name"
    renamed = FLAG_RENAMES.get(sdk_flag)
    if renamed and renamed in dcx_flags:
        return renamed, "renamed"
    return None, "not_found"


def _classify_flag(sdk_flag: str, sdk_info: dict, dcx_flags: dict) -> dict:
    """Classify a single SDK flag against dcx, comparing semantics."""
    dcx_match, match_type = _resolve_dcx_flag(sdk_flag, dcx_flags)

    if match_type == "not_found":
        return {"status": "missing"}

    if match_type == "renamed":
        return {
            "status": "intentional_divergence",
            "note": f"{sdk_flag} -> {dcx_match}",
        }

    # Name matches — now compare semantics
    dcx_info = dcx_flags[dcx_match]
    mismatches: list[str] = []

    sdk_required = sdk_info.get("required", False)
    dcx_required = dcx_info.get("required", False)
    if sdk_required != dcx_required:
        sdk_r = "required" if sdk_required else "optional"
        dcx_r = "required" if dcx_required else "optional"
        mismatches.append(f"SDK {sdk_r}, dcx {dcx_r}")

    sdk_default = sdk_info.get("default")
    dcx_default = dcx_info.get("default")
    if sdk_default != dcx_default and not (sdk_default is None and dcx_default is None):
        mismatches.append(f"SDK default={sdk_default!r}, dcx default={dcx_default!r}")

    if mismatches:
        return {
            "status": "semantic_mismatch",
            "note": "; ".join(mismatches),
        }

    return {"status": "exact"}


def generate_contract(upstream: dict, dcx: dict, upstream_sha: str | None) -> dict:
    """Generate the full compatibility contract."""
    dcx_commands = dcx["commands"]
    dcx_evaluators = dcx["evaluators"]

    command_map = []

    for sdk_cmd in upstream["commands"]:
        sdk_name = sdk_cmd["name"]

        if sdk_name in dcx_commands:
            classification = {"status": "present", "dcx_command": f"dcx analytics {sdk_name}"}
        else:
            classification = {"status": "missing", "dcx_command": None}

        dcx_flags = dcx_commands.get(sdk_name, {}).get("flags", {})

        flag_map = []
        for flag_name, flag_info in sdk_cmd.get("flags", {}).items():
            fc = _classify_flag(flag_name, flag_info, dcx_flags)
            flag_map.append({
                "sdk_flag": flag_name,
                "sdk_type": flag_info.get("type", "unknown"),
                "sdk_required": flag_info.get("required", False),
                "sdk_default": flag_info.get("default"),
                "sdk_env": flag_info.get("env"),
                **fc,
            })

        # dcx-only flags (extensions)
        dcx_only_flags = []
        if sdk_name in dcx_commands:
            sdk_flag_names = set(sdk_cmd.get("flags", {}).keys())
            # Build normalized SDK flag set (accounting for renames)
            sdk_normalized: set[str] = set()
            for f in sdk_flag_names:
                renamed = FLAG_RENAMES.get(f)
                sdk_normalized.add(renamed if renamed else f)

            for dcx_flag, dcx_info in dcx_flags.items():
                if dcx_flag not in sdk_normalized:
                    dcx_only_flags.append({
                        "flag": dcx_flag,
                        "status": "dcx_extension",
                        "type": dcx_info.get("type", "unknown"),
                        "global": dcx_info.get("global", False),
                    })

        command_map.append({
            "sdk_command": sdk_name,
            "dcx_command": classification.get("dcx_command"),
            "status": classification["status"],
            "flags": flag_map,
            "dcx_extensions": dcx_only_flags,
            "args": [
                {"name": k, **v}
                for k, v in sdk_cmd.get("args", {}).items()
            ],
        })

    # Evaluator comparison
    evaluator_map = []
    for ev in upstream["code_evaluators"]:
        dcx_name = ev.replace("_", "-")
        if dcx_name in dcx_evaluators or ev in dcx_evaluators:
            evaluator_map.append({"sdk_name": ev, "dcx_name": dcx_name, "status": "present"})
        else:
            evaluator_map.append({"sdk_name": ev, "dcx_name": None, "status": "missing"})

    judge_map = []
    for j in upstream["llm_judges"]:
        judge_map.append({"sdk_name": j, "status": "missing", "note": "llm-judge not yet in dcx"})

    contract: dict = {
        "generated": str(date.today()),
        "upstream_repo": "haiyuan-eng-google/BigQuery-Agent-Analytics-SDK",
        "upstream_branch": "main",
    }
    if upstream_sha:
        contract["upstream_sha"] = upstream_sha

    contract.update({
        "commands": command_map,
        "code_evaluators": evaluator_map,
        "llm_judges": judge_map,
        "env_vars": {
            "sdk": upstream["env_vars"],
            "dcx": sorted(
                {
                    info.get("env")
                    for cmd in dcx_commands.values()
                    for info in cmd.get("flags", {}).values()
                    if info.get("env")
                }
            ),
        },
        "exit_codes": {
            "sdk": {"success": 0, "eval_failure": 1, "infra_error": 2},
            "dcx": {"success": 0, "eval_failure": 1, "infra_error": "not distinguished"},
        },
        "intentional_divergences": KNOWN_DIVERGENCES,
    })
    return contract


# ── Markdown rendering ───────────────────────────────────────────────


def render_markdown(contract: dict) -> str:
    """Render the contract as a Markdown document."""
    sha = contract.get("upstream_sha", "unknown")
    lines = [
        "# Analytics SDK Compatibility Contract",
        "",
        f"Generated: {contract['generated']}",
        f"Upstream: `{contract['upstream_repo']}` (`{contract['upstream_branch']}`"
        + (f" @ `{sha[:12]}`)" if sha and sha != "unknown" else ")"),
        "",
        "This file is generated by `scripts/parse_sdk_cli.py` from the upstream",
        "SDK `cli.py` and `src/cli.rs`. Do not edit the command inventory manually —",
        "edit intentional divergence notes only.",
        "",
    ]

    # ── Summary
    total = len(contract["commands"])
    present = sum(1 for c in contract["commands"] if c["status"] == "present")
    missing = total - present
    lines += [
        "## Summary",
        "",
        "| Metric | Count |",
        "|--------|-------|",
        f"| SDK commands | {total} |",
        f"| Matched in dcx | {present} |",
        f"| Missing from dcx | {missing} |",
        f"| Code evaluators (SDK) | {len(contract['code_evaluators'])} |",
        f"| Code evaluators in dcx | {sum(1 for e in contract['code_evaluators'] if e['status'] == 'present')} |",
        f"| LLM judges (SDK) | {len(contract['llm_judges'])} |",
        f"| LLM judges in dcx | 0 |",
        "",
    ]

    # ── Command parity
    lines += ["## Command Parity", ""]
    lines += [
        "| SDK Command | dcx Command | Status |",
        "|-------------|-------------|--------|",
    ]
    for cmd in contract["commands"]:
        dcx = cmd["dcx_command"] or "—"
        lines.append(f"| `{cmd['sdk_command']}` | `{dcx}` | {cmd['status']} |")
    lines.append("")

    # ── Flag parity per command
    lines += ["## Flag Parity", ""]
    for cmd in contract["commands"]:
        if cmd["status"] == "missing":
            lines += [f"### `{cmd['sdk_command']}` — missing from dcx", ""]
            continue

        flags = cmd["flags"]
        exact = sum(1 for f in flags if f["status"] == "exact")
        mismatch = sum(1 for f in flags if f["status"] == "semantic_mismatch")
        divergent = sum(1 for f in flags if f["status"] == "intentional_divergence")
        missing_f = sum(1 for f in flags if f["status"] == "missing")
        ext = len(cmd["dcx_extensions"])

        parts = []
        if exact:
            parts.append(f"{exact} exact")
        if mismatch:
            parts.append(f"{mismatch} semantic mismatch")
        if divergent:
            parts.append(f"{divergent} divergent")
        if missing_f:
            parts.append(f"{missing_f} missing")
        if ext:
            parts.append(f"{ext} dcx-only")
        summary = ", ".join(parts)

        lines += [f"### `{cmd['sdk_command']}` ({summary})", ""]
        lines += [
            "| SDK Flag | Type | Required | Status | Note |",
            "|----------|------|----------|--------|------|",
        ]
        for f in flags:
            req = "yes" if f.get("sdk_required") else "no"
            note = f.get("note", "")
            lines.append(
                f"| `{f['sdk_flag']}` | {f['sdk_type']} | {req} | {f['status']} | {note} |"
            )
        for ext_f in cmd["dcx_extensions"]:
            scope = "global" if ext_f.get("global") else "local"
            lines.append(
                f"| `{ext_f['flag']}` | {ext_f.get('type', '—')} | — | dcx_extension ({scope}) | |"
            )
        lines.append("")

    # ── Evaluators
    lines += ["## Evaluator Parity", ""]
    lines += [
        "### Code Evaluators",
        "",
        "| SDK Name | dcx Name | Status |",
        "|----------|----------|--------|",
    ]
    for ev in contract["code_evaluators"]:
        dcx = ev["dcx_name"] or "—"
        lines.append(f"| `{ev['sdk_name']}` | `{dcx}` | {ev['status']} |")
    lines.append("")

    lines += [
        "### LLM Judges",
        "",
        "| SDK Criterion | Status | Note |",
        "|---------------|--------|------|",
    ]
    for j in contract["llm_judges"]:
        lines.append(f"| `{j['sdk_name']}` | {j['status']} | {j.get('note', '')} |")
    lines.append("")

    # ── Exit codes
    lines += ["## Exit Codes", ""]
    ec = contract["exit_codes"]
    lines += [
        "| Meaning | SDK | dcx |",
        "|---------|-----|-----|",
        f"| Success | {ec['sdk']['success']} | {ec['dcx']['success']} |",
        f"| Eval failure | {ec['sdk']['eval_failure']} | {ec['dcx']['eval_failure']} |",
        f"| Infra error | {ec['sdk']['infra_error']} | {ec['dcx']['infra_error']} |",
        "",
    ]

    # ── Env vars
    lines += ["## Environment Variables", ""]
    lines += [
        "| SDK | dcx | Notes |",
        "|-----|-----|-------|",
    ]
    pairs = [
        ("BQ_AGENT_PROJECT", "DCX_PROJECT", "same semantics"),
        ("BQ_AGENT_DATASET", "DCX_DATASET", "same semantics"),
    ]
    for sv, dv, note in pairs:
        lines.append(f"| `{sv}` | `{dv}` | {note} |")
    lines.append("")

    # ── Intentional divergences
    lines += ["## Intentional Divergences", ""]
    lines += [
        "| Item | dcx | SDK | Reason |",
        "|------|-----|-----|--------|",
    ]
    for d in contract["intentional_divergences"]:
        lines.append(f"| {d['item']} | {d['dcx']} | {d['sdk']} | {d['reason']} |")
    lines.append("")

    return "\n".join(lines)


# ── Main ─────────────────────────────────────────────────────────────


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Generate analytics SDK compatibility contract"
    )
    parser.add_argument("--cli-py", required=True, help="Path to upstream cli.py")
    parser.add_argument("--cli-rs", required=True, help="Path to dcx src/cli.rs")
    parser.add_argument("--out-json", required=True, help="Output JSON contract path")
    parser.add_argument("--out-md", required=True, help="Output Markdown contract path")
    parser.add_argument("--upstream-sha", default=None, help="Upstream commit SHA")
    args = parser.parse_args()

    # Parse upstream SDK
    source = Path(args.cli_py).read_text()
    upstream = parse_cli_py(source)

    print(f"Parsed {len(upstream['commands'])} SDK commands")
    print(f"  Code evaluators: {upstream['code_evaluators']}")
    print(f"  LLM judges: {upstream['llm_judges']}")
    print(f"  Env vars: {upstream['env_vars']}")

    # Parse dcx CLI
    rs_source = Path(args.cli_rs).read_text()
    dcx = parse_cli_rs(rs_source)

    print(f"Parsed {len(dcx['commands'])} dcx analytics commands")
    print(f"  Evaluators: {dcx['evaluators']}")
    for cmd_name, cmd_info in dcx["commands"].items():
        print(f"  {cmd_name}: {len(cmd_info['flags'])} flags")

    contract = generate_contract(upstream, dcx, args.upstream_sha)

    # Write JSON
    json_path = Path(args.out_json)
    json_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json.dumps(contract, indent=2) + "\n")
    print(f"\nWrote {json_path}")

    # Write Markdown
    md_path = Path(args.out_md)
    md_path.parent.mkdir(parents=True, exist_ok=True)
    md_path.write_text(render_markdown(contract))
    print(f"Wrote {md_path}")

    # Summary
    total_cmds = len(contract["commands"])
    present_cmds = sum(1 for c in contract["commands"] if c["status"] == "present")
    missing_cmds = total_cmds - present_cmds
    total_evals = len(contract["code_evaluators"])
    present_evals = sum(1 for e in contract["code_evaluators"] if e["status"] == "present")

    print(f"\nContract summary:")
    print(f"  Commands: {present_cmds}/{total_cmds} present, {missing_cmds} missing")
    print(f"  Evaluators: {present_evals}/{total_evals} present")
    print(f"  LLM judges: 0/{len(contract['llm_judges'])} present")
    print(f"  Intentional divergences: {len(contract['intentional_divergences'])}")

    if missing_cmds > 0 or present_evals < total_evals:
        print(f"\n⚠ Gaps remain — see {md_path} for details")


if __name__ == "__main__":
    main()
