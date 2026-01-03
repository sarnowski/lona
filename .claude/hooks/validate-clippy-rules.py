#!/usr/bin/env python3
"""
Hook to prevent suppressing clippy/rustc warnings via #[allow], #[expect],
or clippy configuration files.

Rules:
1. #[allow(...)] and #[expect(...)] are ONLY permitted in _test.rs files
2. Clippy configuration in Cargo.toml ([lints] section) is forbidden
3. clippy.toml and .clippy.toml files are forbidden

If suppression is genuinely needed, the agent must ask the user directly.
"""

import json
import re
import sys
from pathlib import Path


def get_content(tool_input: dict, tool_name: str) -> str | None:
    """Extract content from tool input based on tool type."""
    if tool_name == "Write":
        return tool_input.get("content", "")
    elif tool_name == "Edit":
        return tool_input.get("new_string", "")
    return None


def is_test_module(file_path: str) -> bool:
    """Check if file is a test module (ends with _test.rs)."""
    return file_path.endswith("_test.rs")


def find_suppression_patterns(content: str) -> list[tuple[str, str]]:
    """Find #[allow(...)] and #[expect(...)] patterns in content.

    Returns list of (pattern_type, lint_name) tuples.
    """
    # Match #[allow(...)] and #[expect(...)] with their contents
    # Also matches #![allow(...)] and #![expect(...)] (crate-level)
    pattern = r"#!?\[(allow|expect)\s*\(([^)]+)\)\]"
    matches = re.findall(pattern, content)
    return matches


def find_clippy_config_in_cargo(content: str) -> list[str]:
    """Find clippy/lint configuration in Cargo.toml content."""
    issues = []

    # Check for [lints] section
    if re.search(r"^\s*\[lints\]", content, re.MULTILINE):
        issues.append("[lints] section")

    # Check for [lints.rust] section
    if re.search(r"^\s*\[lints\.rust\]", content, re.MULTILINE):
        issues.append("[lints.rust] section")

    # Check for [lints.clippy] section
    if re.search(r"^\s*\[lints\.clippy\]", content, re.MULTILINE):
        issues.append("[lints.clippy] section")

    # Check for workspace.lints
    if re.search(r"^\s*\[workspace\.lints", content, re.MULTILINE):
        issues.append("[workspace.lints] section")

    return issues


def create_block_message(reason: str, guidance: str) -> str:
    """Create a standardized block message."""
    return f"""BLOCKED: {reason}

{guidance}

If you believe suppression is genuinely necessary (not possible to fix properly without
sacrificing code reliability or security), you MUST ask the user directly and explain:

1. The specific warning/error being suppressed
2. Why it cannot be fixed properly
3. The exact suppression needed (#[allow(lint_name)] or #[expect(lint_name)])
4. Any impact on code quality or security

The user will decide whether to allow the suppression and how to implement it.

DO NOT attempt to work around this hook. Fix the underlying issue instead."""


def main():
    # Check if suppressions are explicitly allowed via marker file
    # The file should be in the project root (same directory as .claude/)
    hook_dir = Path(__file__).resolve().parent  # .claude/hooks/
    project_root = hook_dir.parent.parent  # project root
    allow_file = project_root / "allow-suppressions"

    if allow_file.exists():
        sys.exit(0)  # Suppressions allowed, skip all checks

    # Read input from stdin
    try:
        input_data = json.load(sys.stdin)
    except json.JSONDecodeError:
        sys.exit(0)  # If we can't parse input, let it through

    tool_name = input_data.get("tool_name", "")
    tool_input = input_data.get("tool_input", {})

    # Only check Write and Edit operations
    if tool_name not in ("Write", "Edit"):
        sys.exit(0)

    file_path = tool_input.get("file_path", "")
    if not file_path:
        sys.exit(0)

    path = Path(file_path)
    filename = path.name

    # Block clippy.toml and .clippy.toml entirely
    if filename in ("clippy.toml", ".clippy.toml"):
        message = create_block_message(
            "Clippy configuration files are not allowed",
            "Clippy warnings must be fixed, not configured away globally."
        )
        print(message, file=sys.stderr)
        sys.exit(2)

    content = get_content(tool_input, tool_name)
    if content is None:
        sys.exit(0)

    # Check Cargo.toml for lint configuration
    if filename == "Cargo.toml":
        issues = find_clippy_config_in_cargo(content)
        if issues:
            message = create_block_message(
                f"Cargo.toml contains lint configuration: {', '.join(issues)}",
                "Lint configuration in Cargo.toml suppresses warnings globally.\n"
                "Fix the underlying issues in the code instead."
            )
            print(message, file=sys.stderr)
            sys.exit(2)

    # Check Rust files for #[allow] and #[expect]
    if file_path.endswith(".rs"):
        # Allow in test modules
        if is_test_module(file_path):
            sys.exit(0)

        suppressions = find_suppression_patterns(content)
        if suppressions:
            # Format the found suppressions for the message
            found_items = [f"#[{typ}({lint})]" for typ, lint in suppressions]
            unique_items = list(set(found_items))

            message = create_block_message(
                f"Code contains warning suppressions: {', '.join(unique_items)}",
                "Warning suppressions (#[allow], #[expect]) are only permitted in _test.rs files.\n"
                "In production code, fix the underlying issue that triggers the warning."
            )
            print(message, file=sys.stderr)
            sys.exit(2)

    # All checks passed
    sys.exit(0)


if __name__ == "__main__":
    main()
