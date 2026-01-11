#!/bin/bash
# Lists unique top-level directories containing changes
# (both committed since origin/main and uncommitted)
{ git diff --name-only origin/main...HEAD 2>/dev/null; git diff --name-only HEAD; } | sort -u | cut -d'/' -f1 | sort -u
