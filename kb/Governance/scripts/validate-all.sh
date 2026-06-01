#!/bin/bash
# validate-all.sh - Complete governance validation
# Single entry point per decision D3
#
# Runs all four validation layers in sequence:
# 1. Structure (JSON Schema) - validate-structure.sh
# 2. Policy Logic (governance rules) - validate-policy.sh
# 3. Eval Frontmatter (gray-box) - validate-evals.sh
# 4. Documentation Hygiene (markdownlint, links) - validate-docs.sh

set -euo pipefail

# Get script directory for relative paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "=== Converge Personas Governance Validation ==="
echo ""
echo "Running all validation layers..."
echo ""

# Track overall status
FAILED_LAYERS=()

# Layer 1: Structure (JSON Schema)
echo "--- Layer 1: Structure Validation ---"
if ./scripts/validate-structure.sh; then
  echo ""
else
  echo "Structure validation failed"
  FAILED_LAYERS+=("Structure")
  echo ""
fi

# Layer 2: Policy Logic
echo "--- Layer 2: Policy Validation ---"
if ./scripts/validate-policy.sh; then
  echo ""
else
  echo "Policy validation failed"
  FAILED_LAYERS+=("Policy")
  echo ""
fi

# Layer 3: Existing evals validation
echo "--- Layer 3: Eval Frontmatter Validation ---"
if ./scripts/validate-evals.sh; then
  echo ""
else
  echo "Eval validation failed"
  FAILED_LAYERS+=("Evals")
  echo ""
fi

# Layer 4: Documentation Hygiene
echo "--- Layer 4: Documentation Validation ---"
if ./scripts/validate-docs.sh; then
  echo ""
else
  echo "Documentation validation failed"
  FAILED_LAYERS+=("Documentation")
  echo ""
fi

echo "=== Governance Validation Summary ==="
echo ""

if [ ${#FAILED_LAYERS[@]} -eq 0 ]; then
  echo "All validations passed"
  echo ""
  echo "Layers validated:"
  echo "  1. Structure (JSON Schema)"
  echo "  2. Policy (governance rules)"
  echo "  3. Evals (frontmatter)"
  echo "  4. Documentation (markdown, links)"
  exit 0
else
  echo "Failed layers: ${FAILED_LAYERS[*]}"
  echo ""
  echo "Fix issues in failed layers and re-run."
  exit 1
fi
