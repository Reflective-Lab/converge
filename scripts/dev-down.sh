#!/usr/bin/env bash
set -euo pipefail

mode="${1:-auto}"

case "${mode}" in
    auto|native)
        pkill -f "cargo run -p converge-runtime" 2>/dev/null || true
        pkill -f "target/.*/converge-runtime" 2>/dev/null || true
        echo "requested converge-runtime stop"
        ;;
    help|-h|--help)
        echo "usage: scripts/dev-down.sh [auto|native]"
        ;;
    *)
        echo "unsupported dev mode: ${mode}" >&2
        exit 64
        ;;
esac
