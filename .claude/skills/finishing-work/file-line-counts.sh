#!/bin/bash
# Lists changed files with their line counts, sorted by size descending
{ git diff --name-only origin/main...HEAD 2>/dev/null; git diff --name-only HEAD; } | sort -u | xargs -I{} sh -c 'echo "$(wc -l < "{}" 2>/dev/null || echo 0) {}"' | sort -rn
