#!/usr/bin/env python3
"""
Extract specification tests from Lonala documentation.

Parses markdown files from docs/lonala/*.md and extracts test blocks
containing '; =>' assertions into Rust code for the spec test runner.

Usage: extract_spec_tests.py [--rust-output PATH]

Output: crates/lona-vm/src/e2e/spec_data.rs (or specified path)
"""

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class Source:
    """Source location information for a test block."""
    file: str
    line_start: int
    line_end: int
    section: list[str]


@dataclass
class SetupLine:
    """A setup line (no assertion) in a test block."""
    line_offset: int
    expression: str


@dataclass
class Expected:
    """Expected result of an assertion."""
    type: str  # "value" or "error"
    value: Optional[str] = None  # For value type or error reason


@dataclass
class Assertion:
    """A single assertion in a test block."""
    line_offset: int
    expression: str
    expected: Expected
    tags: list[str] = field(default_factory=list)


@dataclass
class SpecBlock:
    """A test block extracted from documentation."""
    id: str
    source: Source
    block_tags: list[str]
    setup: list[SetupLine]
    assertions: list[Assertion]


def parse_expected(expected_str: str) -> Expected:
    """Parse the expected value from an assertion.

    Examples:
        "3" -> Expected(type="value", value="3")
        "ERROR" -> Expected(type="error", value=None)
        "ERROR :out-of-bounds" -> Expected(type="error", value=":out-of-bounds")
    """
    expected_str = expected_str.strip()

    if expected_str.upper().startswith("ERROR"):
        remainder = expected_str[5:].strip()
        if remainder:
            return Expected(type="error", value=remainder)
        return Expected(type="error", value=None)

    return Expected(type="value", value=expected_str)


def parse_line_tags(text: str) -> list[str]:
    """Extract @tags from the end of a line.

    Example: "42  @todo @x86_64" -> ["todo", "x86_64"]
    """
    tags = []
    for match in re.finditer(r'@(\w+)', text):
        tags.append(match.group(1))
    return tags


def split_assertion_line(line: str) -> tuple[str, str, list[str]]:
    """Split a line containing '; =>' into (expression, expected, tags).

    Example:
        "(+ 1 2)  ; => 3  @todo" -> ("(+ 1 2)", "3", ["todo"])
    """
    # Find the assertion marker
    idx = line.find('; =>')
    if idx == -1:
        raise ValueError(f"No '; =>' found in line: {line}")

    expression = line[:idx].strip()
    remainder = line[idx + 4:].strip()  # Skip "; =>"

    # Find where tags start (first @)
    tag_idx = remainder.find('@')
    if tag_idx == -1:
        expected_str = remainder
        tags = []
    else:
        expected_str = remainder[:tag_idx].strip()
        tags = parse_line_tags(remainder[tag_idx:])

    return expression, expected_str, tags


def count_parens(text: str) -> int:
    """Count net open parentheses/brackets in text.

    Returns positive if more opens than closes, negative if more closes.
    Ignores parens inside strings.
    """
    in_string = False
    escape_next = False
    depth = 0

    for char in text:
        if escape_next:
            escape_next = False
            continue

        if char == '\\':
            escape_next = True
            continue

        if char == '"':
            in_string = not in_string
            continue

        if in_string:
            continue

        if char in '([{':
            depth += 1
        elif char in ')]}':
            depth -= 1

    return depth


def parse_code_block(lines: list[str], start_line: int) -> tuple[list[SetupLine], list[Assertion], list[str]]:
    """Parse a code block into setup lines, assertions, and block tags.

    Returns (setup, assertions, block_tags).
    """
    setup: list[SetupLine] = []
    assertions: list[Assertion] = []
    block_tags: list[str] = []

    # Check for block-level directive at start
    if lines and lines[0].strip().startswith(';; @'):
        directive_line = lines[0].strip()
        block_tags = parse_line_tags(directive_line)
        lines = lines[1:]

    # Accumulator for multi-line expressions
    expr_lines: list[str] = []
    expr_start_offset = 0
    line_offset = 0

    for line in lines:
        stripped = line.strip()

        # Skip empty lines and pure comments (but not assertion comments)
        if not stripped:
            line_offset += 1
            continue
        if stripped.startswith(';;') and '; =>' not in stripped:
            line_offset += 1
            continue
        if stripped.startswith(';') and not stripped.startswith(';;') and '; =>' not in stripped:
            line_offset += 1
            continue

        # Check if this line contains an assertion
        has_assertion = '; =>' in line

        if has_assertion:
            # Complete the expression with this line
            expr_lines.append(line)
            full_line = '\n'.join(expr_lines)

            # Parse the assertion
            expression, expected_str, tags = split_assertion_line(full_line)
            expected = parse_expected(expected_str)

            assertions.append(Assertion(
                line_offset=expr_start_offset,
                expression=expression,
                expected=expected,
                tags=tags,
            ))

            # Reset accumulator
            expr_lines = []
            expr_start_offset = line_offset + 1
        else:
            # No assertion - could be setup or part of multi-line expression
            if not expr_lines:
                expr_start_offset = line_offset

            expr_lines.append(stripped)

            # Check if expression is complete (balanced parens)
            full_expr = '\n'.join(expr_lines)
            if count_parens(full_expr) == 0:
                # Complete setup line
                setup.append(SetupLine(
                    line_offset=expr_start_offset,
                    expression=full_expr,
                ))
                expr_lines = []
                expr_start_offset = line_offset + 1

        line_offset += 1

    return setup, assertions, block_tags


def generate_block_id(file_path: str, section: list[str], block_num: int) -> str:
    """Generate a unique block ID.

    Format: "filename::Section::Subsection::n"
    """
    filename = Path(file_path).name
    parts = [filename] + section + [str(block_num)]
    return '::'.join(parts)


def extract_from_markdown(file_path: Path, base_path: Path) -> list[SpecBlock]:
    """Extract test blocks from a markdown file."""
    blocks: list[SpecBlock] = []

    content = file_path.read_text()
    lines = content.split('\n')

    # Track current section hierarchy
    section: list[str] = []
    section_block_counts: dict[str, int] = {}

    # State machine for parsing
    in_code_block = False
    code_block_lang = ''
    code_block_lines: list[str] = []
    code_block_start = 0

    for i, line in enumerate(lines, 1):
        # Track section headers
        header_match = re.match(r'^(#{1,6})\s+`?([^`]+)`?', line)
        if header_match and not in_code_block:
            level = len(header_match.group(1))
            title = header_match.group(2).strip()

            # Truncate section to current level
            section = section[:level - 1]
            section.append(title)
            continue

        # Check for code block start
        fence_match = re.match(r'^```(\w*)', line)
        if fence_match and not in_code_block:
            in_code_block = True
            code_block_lang = fence_match.group(1).lower()
            code_block_lines = []
            code_block_start = i + 1  # Next line is first content line
            continue

        # Check for code block end
        if line.startswith('```') and in_code_block:
            in_code_block = False

            # Only process clojure/lonala blocks
            if code_block_lang in ('clojure', 'lonala', ''):
                # Parse the block
                setup, assertions, block_tags = parse_code_block(
                    code_block_lines, code_block_start
                )

                # Only include blocks with assertions (test blocks)
                if assertions:
                    # Generate unique ID
                    section_key = '::'.join(section)
                    block_num = section_block_counts.get(section_key, 0) + 1
                    section_block_counts[section_key] = block_num

                    rel_path = str(file_path.relative_to(base_path))
                    block_id = generate_block_id(rel_path, section, block_num)

                    blocks.append(SpecBlock(
                        id=block_id,
                        source=Source(
                            file=rel_path,
                            line_start=code_block_start,
                            line_end=i - 1,
                            section=list(section),
                        ),
                        block_tags=block_tags,
                        setup=setup,
                        assertions=assertions,
                    ))

            code_block_lines = []
            continue

        # Collect code block content
        if in_code_block:
            code_block_lines.append(line)

    return blocks


def generate_rust_code(blocks: list[SpecBlock]) -> str:
    """Generate Rust code with static arrays for spec tests."""
    lines = [
        "// SPDX-License-Identifier: GPL-3.0-or-later",
        "// Copyright 2026 Tobias Sarnowski",
        "",
        "//! Auto-generated specification test data.",
        "//!",
        "//! Generated by scripts/extract_spec_tests.py - DO NOT EDIT",
        "",
        "#![allow(dead_code)]",
        "",
    ]

    # Generate block data
    lines.append(f"pub const SPEC_BLOCK_COUNT: usize = {len(blocks)};")
    lines.append("")

    # Generate each block as a const struct
    lines.append("pub struct SpecBlock {")
    lines.append("    pub id: &'static str,")
    lines.append("    pub source_file: &'static str,")
    lines.append("    pub line_start: u32,")
    lines.append("    pub block_tags: &'static [&'static str],")
    lines.append("    pub setup: &'static [SetupLine],")
    lines.append("    pub assertions: &'static [Assertion],")
    lines.append("}")
    lines.append("")

    lines.append("pub struct SetupLine {")
    lines.append("    pub line_offset: u32,")
    lines.append("    pub expression: &'static str,")
    lines.append("}")
    lines.append("")

    lines.append("pub struct Assertion {")
    lines.append("    pub line_offset: u32,")
    lines.append("    pub expression: &'static str,")
    lines.append("    pub expected_type: ExpectedType,")
    lines.append("    pub expected_value: Option<&'static str>,")
    lines.append("    pub tags: &'static [&'static str],")
    lines.append("}")
    lines.append("")

    lines.append("#[derive(Clone, Copy, PartialEq, Eq)]")
    lines.append("pub enum ExpectedType {")
    lines.append("    Value,")
    lines.append("    Error,")
    lines.append("}")
    lines.append("")

    # Generate static data for each block
    for i, block in enumerate(blocks):
        block_name = f"BLOCK_{i}"

        # Block tags
        if block.block_tags:
            tags_str = ", ".join(f'"{t}"' for t in block.block_tags)
            lines.append(f"const {block_name}_TAGS: &[&str] = &[{tags_str}];")
        else:
            lines.append(f"const {block_name}_TAGS: &[&str] = &[];")

        # Setup lines
        if block.setup:
            lines.append(f"const {block_name}_SETUP: &[SetupLine] = &[")
            for setup in block.setup:
                expr_escaped = setup.expression.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n')
                lines.append(f'    SetupLine {{ line_offset: {setup.line_offset}, expression: "{expr_escaped}" }},')
            lines.append("];")
        else:
            lines.append(f"const {block_name}_SETUP: &[SetupLine] = &[];")

        # Assertions
        if block.assertions:
            lines.append(f"const {block_name}_ASSERTIONS: &[Assertion] = &[")
            for assertion in block.assertions:
                expr_escaped = assertion.expression.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n')
                exp_type = "ExpectedType::Error" if assertion.expected.type == "error" else "ExpectedType::Value"
                if assertion.expected.value:
                    val_escaped = assertion.expected.value.replace('\\', '\\\\').replace('"', '\\"')
                    exp_val = f'Some("{val_escaped}")'
                else:
                    exp_val = "None"
                if assertion.tags:
                    tags_str = "&[" + ", ".join(f'"{t}"' for t in assertion.tags) + "]"
                else:
                    tags_str = "&[]"
                lines.append(f'    Assertion {{ line_offset: {assertion.line_offset}, expression: "{expr_escaped}", expected_type: {exp_type}, expected_value: {exp_val}, tags: {tags_str} }},')
            lines.append("];")
        else:
            lines.append(f"const {block_name}_ASSERTIONS: &[Assertion] = &[];")

        lines.append("")

    # Generate the main array of blocks
    lines.append("pub const SPEC_BLOCKS: &[SpecBlock] = &[")
    for i, block in enumerate(blocks):
        block_name = f"BLOCK_{i}"
        id_escaped = block.id.replace('\\', '\\\\').replace('"', '\\"')
        file_escaped = block.source.file.replace('\\', '\\\\').replace('"', '\\"')
        lines.append(f'    SpecBlock {{ id: "{id_escaped}", source_file: "{file_escaped}", line_start: {block.source.line_start}, block_tags: {block_name}_TAGS, setup: {block_name}_SETUP, assertions: {block_name}_ASSERTIONS }},')
    lines.append("];")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description='Extract specification tests from documentation')
    parser.add_argument('--rust-output', '-r', default='crates/lona-vm/src/e2e/spec_data.rs',
                        help='Output Rust file path')
    parser.add_argument('--docs', '-d', default='docs/lonala',
                        help='Documentation directory')
    args = parser.parse_args()

    # Find project root (directory containing docs/)
    script_dir = Path(__file__).parent
    project_root = script_dir.parent

    docs_dir = project_root / args.docs
    rust_output_path = project_root / args.rust_output

    if not docs_dir.exists():
        print(f"Error: Documentation directory not found: {docs_dir}", file=sys.stderr)
        sys.exit(1)

    # Find all markdown files
    md_files = sorted(docs_dir.glob('*.md'))
    if not md_files:
        print(f"Error: No markdown files found in {docs_dir}", file=sys.stderr)
        sys.exit(1)

    # Extract from all files
    all_blocks: list[SpecBlock] = []

    for md_file in md_files:
        rel_path = str(md_file.relative_to(project_root))
        blocks = extract_from_markdown(md_file, project_root)
        all_blocks.extend(blocks)

        if blocks:
            print(f"  {rel_path}: {len(blocks)} test blocks", file=sys.stderr)

    # Ensure output directory exists
    rust_output_path.parent.mkdir(parents=True, exist_ok=True)

    # Write Rust code
    rust_code = generate_rust_code(all_blocks)
    with open(rust_output_path, 'w') as f:
        f.write(rust_code)

    # Summary
    total_assertions = sum(len(b.assertions) for b in all_blocks)
    print(f"\nExtracted {len(all_blocks)} test blocks with {total_assertions} assertions", file=sys.stderr)
    print(f"Rust output: {rust_output_path}", file=sys.stderr)


if __name__ == '__main__':
    main()
