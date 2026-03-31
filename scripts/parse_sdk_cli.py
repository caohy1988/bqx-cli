#!/usr/bin/env python3
"""Parse upstream BigQuery Agent Analytics SDK cli.py and generate a
compatibility contract against the dcx analytics surface.

Usage:
    python3 scripts/parse_sdk_cli.py \
        --cli-py tests/fixtures/upstream_sdk_latest/cli.py \
        --out-json tests/fixtures/analytics_sdk_contract.json \
        --out-md docs/analytics_sdk_contract.md
"""

from __future__ import annotations

import argparse
import ast
import json
import sys
from dataclasses import asdict, dataclass, field
from datetime import date, timezone
from pathlib import Path

# ── dcx analytics surface (ground truth from src/cli.rs) ─────────────

DCX_COMMANDS: dict[str, dict] = {
    "doctor": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
        },
    },
    "evaluate": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
            "--evaluator": {"type": "enum", "required": True, "values": ["latency", "error-rate"]},
            "--threshold": {"type": "float", "required": True},
            "--last": {"type": "string", "required": True},
            "--agent-id": {"type": "string", "required": False},
            "--exit-code": {"type": "bool", "required": False, "default": False},
        },
    },
    "get-trace": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
            "--session-id": {"type": "string", "required": True},
        },
    },
    "list-traces": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
            "--last": {"type": "string", "required": True},
            "--agent-id": {"type": "string", "required": False},
            "--limit": {"type": "int", "required": False, "default": 20},
        },
    },
    "insights": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
            "--last": {"type": "string", "required": True},
            "--agent-id": {"type": "string", "required": False},
        },
    },
    "drift": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
            "--golden-dataset": {"type": "string", "required": True},
            "--last": {"type": "string", "required": False, "default": "7d"},
            "--agent-id": {"type": "string", "required": False},
            "--min-coverage": {"type": "float", "required": False, "default": 0.8},
            "--exit-code": {"type": "bool", "required": False, "default": False},
        },
    },
    "distribution": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
            "--last": {"type": "string", "required": True},
            "--agent-id": {"type": "string", "required": False},
        },
    },
    "hitl-metrics": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
            "--last": {"type": "string", "required": True},
            "--agent-id": {"type": "string", "required": False},
            "--limit": {"type": "int", "required": False, "default": 20},
        },
    },
    "views create-all": {
        "flags": {
            "--project-id": {"type": "string", "required": True, "env": "DCX_PROJECT"},
            "--dataset-id": {"type": "string", "required": True, "env": "DCX_DATASET"},
            "--location": {"type": "string", "required": False, "default": "US", "env": "DCX_LOCATION"},
            "--table": {"type": "string", "required": False, "default": "agent_events"},
            "--format": {"type": "string", "required": False, "default": "json"},
            "--prefix": {"type": "string", "required": False, "default": ""},
        },
    },
}

DCX_EVALUATORS = ["latency", "error-rate"]

# Known intentional divergences seeded from plan review
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
        elif kw.arg == "help" and isinstance(kw.value, ast.Constant):
            info["help"] = kw.value.value
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
    # Find the command name from decorator
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

    for param in func.args.args:
        if param.arg == "self":
            continue
        # Skip return type, etc.
        ann = param.annotation
        if ann is None:
            continue

        cli_flag = _python_name_to_cli(param.arg)
        type_str = _resolve_type_annotation(ann)

        # Look for default value — a typer.Option or typer.Argument call
        flag_info: dict = {"type": type_str}
        default = param.default if hasattr(param, "default") else None
        if default is None:
            # Check in func.args.defaults by position
            pass

    # Use a different approach: iterate defaults aligned with args
    non_self_args = [a for a in func.args.args if a.arg != "self"]
    defaults = func.args.defaults
    # defaults align to the last N args
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
            # --fmt is really --format
            if param.arg == "fmt":
                cli_flag = "--format"
            flags[cli_flag] = flag_info

    return {"name": cmd_name, "flags": flags, "args": args}


def _extract_dict_keys(node: ast.Dict) -> list[str]:
    """Extract string keys from a dict literal."""
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

    # First pass: find top-level assignments for evaluators/judges
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name):
                    if target.id == "_CODE_EVALUATORS" and isinstance(node.value, ast.Dict):
                        code_evaluators = _extract_dict_keys(node.value)
                    elif target.id == "_LLM_JUDGES" and isinstance(node.value, ast.Dict):
                        llm_judges = _extract_dict_keys(node.value)

    # Second pass: find decorated command functions
    views_commands: list[dict] = []

    for node in ast.iter_child_nodes(tree):
        if isinstance(node, ast.FunctionDef):
            is_command = False
            for dec in node.decorator_list:
                if isinstance(dec, ast.Call) and isinstance(dec.func, ast.Attribute):
                    if dec.func.attr == "command":
                        if isinstance(dec.func.value, ast.Name):
                            if dec.func.value.id == "app":
                                is_command = True
                                cmd = _extract_command(node)
                                commands.append(cmd)
                            elif dec.func.value.id == "views_app":
                                is_command = True
                                cmd = _extract_command(node, parent_name="views")
                                views_commands.append(cmd)
                elif isinstance(dec, ast.Attribute):
                    if isinstance(dec.value, ast.Name):
                        if dec.value.id == "app" and dec.attr == "command":
                            is_command = True
                            cmd = _extract_command(node)
                            commands.append(cmd)

    commands.extend(views_commands)

    # Collect env vars from all flags
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


# ── Contract generation ──────────────────────────────────────────────


def _classify_command(sdk_name: str) -> dict:
    """Classify a single SDK command against dcx."""
    dcx_name = sdk_name  # same name convention
    if dcx_name in DCX_COMMANDS:
        return {"status": "present", "dcx_command": f"dcx analytics {dcx_name}"}
    return {"status": "missing", "dcx_command": None}


def _classify_flag(sdk_flag: str, sdk_info: dict, dcx_flags: dict) -> dict:
    """Classify a single SDK flag against dcx flags."""
    # Direct match
    if sdk_flag in dcx_flags:
        return {"status": "exact"}

    # Known renames
    if sdk_flag == "--table-id" and "--table" in dcx_flags:
        return {"status": "intentional_divergence", "note": "--table-id -> --table"}

    return {"status": "missing"}


def generate_contract(upstream: dict) -> dict:
    """Generate the full compatibility contract."""
    command_map = []

    for sdk_cmd in upstream["commands"]:
        sdk_name = sdk_cmd["name"]
        classification = _classify_command(sdk_name)

        flag_map = []
        dcx_flags = DCX_COMMANDS.get(sdk_name, {}).get("flags", {})

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

        # Also check for dcx-only flags (extensions)
        dcx_only_flags = []
        if sdk_name in DCX_COMMANDS:
            sdk_flag_names = set(sdk_cmd.get("flags", {}).keys())
            # Normalize --table-id -> --table for comparison
            sdk_flag_normalized = set()
            for f in sdk_flag_names:
                if f == "--table-id":
                    sdk_flag_normalized.add("--table")
                else:
                    sdk_flag_normalized.add(f)
            # Also normalize --fmt -> --format
            for f in list(sdk_flag_normalized):
                if f == "--fmt":
                    sdk_flag_normalized.discard(f)
                    sdk_flag_normalized.add("--format")

            for dcx_flag in dcx_flags:
                if dcx_flag not in sdk_flag_normalized:
                    dcx_only_flags.append({
                        "flag": dcx_flag,
                        "status": "dcx_extension",
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
        if dcx_name in DCX_EVALUATORS or ev in DCX_EVALUATORS:
            evaluator_map.append({"sdk_name": ev, "dcx_name": dcx_name, "status": "present"})
        else:
            evaluator_map.append({"sdk_name": ev, "dcx_name": None, "status": "missing"})

    judge_map = []
    for j in upstream["llm_judges"]:
        judge_map.append({"sdk_name": j, "status": "missing", "note": "llm-judge not yet in dcx"})

    return {
        "generated": str(date.today()),
        "upstream_repo": "haiyuan-eng-google/BigQuery-Agent-Analytics-SDK",
        "upstream_branch": "main",
        "commands": command_map,
        "code_evaluators": evaluator_map,
        "llm_judges": judge_map,
        "env_vars": {
            "sdk": upstream["env_vars"],
            "dcx": ["DCX_PROJECT", "DCX_DATASET", "DCX_LOCATION"],
        },
        "exit_codes": {
            "sdk": {"success": 0, "eval_failure": 1, "infra_error": 2},
            "dcx": {"success": 0, "eval_failure": 1, "infra_error": "not distinguished"},
        },
        "intentional_divergences": KNOWN_DIVERGENCES,
    }


# ── Markdown rendering ───────────────────────────────────────────────


def render_markdown(contract: dict) -> str:
    """Render the contract as a Markdown document."""
    lines = [
        "# Analytics SDK Compatibility Contract",
        "",
        f"Generated: {contract['generated']}",
        f"Upstream: `{contract['upstream_repo']}` (`{contract['upstream_branch']}`)",
        "",
        "This file is generated by `scripts/parse_sdk_cli.py`. Do not edit",
        "the command inventory manually — edit intentional divergence notes only.",
        "",
    ]

    # ── Summary
    total = len(contract["commands"])
    present = sum(1 for c in contract["commands"] if c["status"] == "present")
    missing = total - present
    lines += [
        "## Summary",
        "",
        f"| Metric | Count |",
        f"|--------|-------|",
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
        status = cmd["status"]
        lines.append(f"| `{cmd['sdk_command']}` | `{dcx}` | {status} |")
    lines.append("")

    # ── Flag parity per command
    lines += ["## Flag Parity", ""]
    for cmd in contract["commands"]:
        if cmd["status"] == "missing":
            lines += [f"### `{cmd['sdk_command']}` — missing from dcx", ""]
            continue

        total_flags = len(cmd["flags"])
        exact = sum(1 for f in cmd["flags"] if f["status"] == "exact")
        divergent = sum(1 for f in cmd["flags"] if f["status"] == "intentional_divergence")
        missing_f = sum(1 for f in cmd["flags"] if f["status"] == "missing")
        ext = len(cmd["dcx_extensions"])

        lines += [f"### `{cmd['sdk_command']}` ({exact}/{total_flags} exact, {missing_f} missing, {ext} dcx-only)", ""]
        lines += [
            "| SDK Flag | Type | Required | Status | Note |",
            "|----------|------|----------|--------|------|",
        ]
        for f in cmd["flags"]:
            req = "yes" if f.get("sdk_required") else "no"
            note = f.get("note", "")
            lines.append(
                f"| `{f['sdk_flag']}` | {f['sdk_type']} | {req} | {f['status']} | {note} |"
            )
        for ext_f in cmd["dcx_extensions"]:
            lines.append(
                f"| `{ext_f['flag']}` | — | — | dcx_extension | |"
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
    sdk_vars = contract["env_vars"]["sdk"]
    dcx_vars = contract["env_vars"]["dcx"]
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
    parser = argparse.ArgumentParser(description="Generate analytics SDK compatibility contract")
    parser.add_argument("--cli-py", required=True, help="Path to upstream cli.py")
    parser.add_argument("--out-json", required=True, help="Output JSON contract path")
    parser.add_argument("--out-md", required=True, help="Output Markdown contract path")
    args = parser.parse_args()

    source = Path(args.cli_py).read_text()
    upstream = parse_cli_py(source)

    print(f"Parsed {len(upstream['commands'])} SDK commands")
    print(f"  Code evaluators: {upstream['code_evaluators']}")
    print(f"  LLM judges: {upstream['llm_judges']}")
    print(f"  Env vars: {upstream['env_vars']}")

    contract = generate_contract(upstream)

    # Write JSON
    json_path = Path(args.out_json)
    json_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json.dumps(contract, indent=2) + "\n")
    print(f"Wrote {json_path}")

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
