#!/usr/bin/env bash
set -euo pipefail

mode="${1:-auto}"

case "${mode}" in
    auto|native)
        exec cargo run -p converge-runtime
        ;;
    help|-h|--help)
        echo "usage: scripts/dev-up.sh [auto|native]"
        ;;
    *)
        echo "unsupported dev mode: ${mode}" >&2
        exit 64
        ;;
esac
