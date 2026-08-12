#!/usr/bin/env bash
# Semantic-equivalence verification for the layered mpv-config split.
#
# Usage:
#   tools/verify-equivalence.sh linux [--ref PATH] [--input-ref PATH]
#                                   [--whitelist PATH] [--self-test]
#   tools/verify-equivalence.sh windows [options...]
#
# Merges config/base.conf (with platform conditionals) + the platform layer,
# then compares the active option set (top level + every profile) and the
# input.conf binding set against a reference config. Differences must fall
# inside the whitelist (docs/equivalence-whitelist-<platform>.txt) or the
# script exits non-zero.
#
# References (read-only, never modified):
#   linux   -> ~/.config/mpv/mpv.conf  and  ~/.config/mpv/input.conf
#   windows -> archive/mpv.conf.orig   and  archive/input.conf.orig
#
# Exit codes: 0 = equivalent (or whitelisted diffs only), 1 = real diffs,
# 2 = usage/IO error.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

if ! command -v python3 >/dev/null 2>&1; then
    echo "error: python3 is required" >&2
    exit 2
fi

exec python3 "$SCRIPT_DIR/verify_equivalence.py" "$@"
