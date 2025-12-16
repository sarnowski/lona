#!/usr/bin/env python3
"""
Hook to prevent NEW #[allow(...)] and #[expect(...)] directives
without explicit user approval. Ignores existing directives.

Directives containing "[approved]" in their reason are allowed,
indicating the user has explicitly approved the suppression.

Runs on PreToolUse for Edit and Write tools targeting .rs files.
"""

import json
import re
import sys
from pathlib import Path

# Marker that indicates user has approved the suppression
APPROVAL_MARKER = "[approved]"


def find_directive_lines(content: str) -> list[str]:
    """Extract full directive lines for reporting."""
    # Use [\s\S]+? to match any character including newlines (non-greedy)
    # Use non-capturing group (?:...) so findall returns full match
    pattern = r"#!?\[(?:allow|expect)\([\s\S]+?\)\]"
    return re.findall(pattern, content or "")


def has_approval_marker(directive: str) -> bool:
    """Check if directive contains the approval marker in its reason."""
    return APPROVAL_MARKER.lower() in directive.lower()


def check_for_new_directives(old_content: str, new_content: str) -> list[str]:
    """Return list of unapproved directives in new_content that aren't in old_content."""
    new_directive_lines = find_directive_lines(new_content)

    # Find directives that are genuinely new and not approved
    new_directives = []
    for directive in new_directive_lines:
        # Check if this specific directive pattern existed before
        if directive not in old_content:
            # Allow if it has the approval marker
            if not has_approval_marker(directive):
                new_directives.append(directive)

    return new_directives


def main() -> None:
    try:
        input_data = json.load(sys.stdin)
    except json.JSONDecodeError:
        sys.exit(1)

    tool_name = input_data.get("tool_name", "")
    tool_input = input_data.get("tool_input", {})
    file_path = tool_input.get("file_path", "")

    # Only check Rust files
    if not file_path.endswith(".rs"):
        sys.exit(0)

    new_directives = []

    if tool_name == "Edit":
        # Edit provides old_string and new_string directly
        old_string = tool_input.get("old_string", "")
        new_string = tool_input.get("new_string", "")
        new_directives = check_for_new_directives(old_string, new_string)

    elif tool_name == "Write":
        # Write overwrites file - read existing content from disk
        new_content = tool_input.get("content", "")
        old_content = ""

        try:
            path = Path(file_path)
            if path.exists():
                old_content = path.read_text()
        except (OSError, IOError):
            pass  # File doesn't exist or can't read - treat as empty

        new_directives = check_for_new_directives(old_content, new_content)

    if new_directives:
        directive_list = ", ".join(new_directives[:3])  # Show first 3
        if len(new_directives) > 3:
            directive_list += f" (and {len(new_directives) - 3} more)"

        output = {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": (
                    f"BLOCKED: Attempting to add new suppression directive(s): "
                    f"{directive_list}\n\n"
                    "Per CLAUDE.md policy, you must:\n"
                    "1. Attempt to fix the underlying issue correctly\n"
                    "2. If unfixable, explain the issue in detail to the user\n"
                    "3. Wait for EXPLICIT user approval before adding any "
                    "suppression\n\n"
                    "Resubmit with either a proper fix or request user approval "
                    "first."
                ),
            }
        }

        print(json.dumps(output))
        sys.exit(0)

    # No new directives - allow the operation
    sys.exit(0)


if __name__ == "__main__":
    main()
