#!/bin/bash
# Lists changed Rust source files (excluding test files)
"$(dirname "$0")/changed-files.sh" | grep -E '\.rs$' | grep -v '_test\.rs$'
