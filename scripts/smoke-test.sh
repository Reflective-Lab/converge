#!/usr/bin/env bash
set -euo pipefail

base_url="${1:-http://127.0.0.1:8080}"
base_url="${base_url%/}"

curl -fsS "${base_url}/health" >/dev/null
curl -fsS "${base_url}/ready" >/dev/null
curl -fsS "${base_url}/api-docs/openapi.json" >/dev/null

echo "smoke ok: ${base_url}"
