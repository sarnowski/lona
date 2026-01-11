#!/bin/bash
# Lists all changed files: committed since origin/main, uncommitted, and untracked
{ git diff --name-only origin/main...HEAD 2>/dev/null; git diff --name-only HEAD; git status --porcelain | awk '{print $2}'; } | sort -u
