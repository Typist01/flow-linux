#!/usr/bin/env bash
# Fail if the tree looks like it contains live secrets (pre-release hygiene).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail=0

if find . -name '.env' -o -name '.env.*' | grep -v '^\./target' | grep -q .; then
  echo "ERROR: .env files found in tree" >&2
  fail=1
fi

# Live-looking OpenAI keys (allow sk-test-… and sk-…placeholder patterns in docs/tests)
if rg -n --glob '!target/**' --glob '!**/*.lock' \
  -e 'sk-[a-zA-Z0-9]{20,}' \
  . 2>/dev/null | grep -v 'sk-test-' | grep -v 'sk-\.\.\.' | grep -v 'sk-…' | grep -q .
then
  echo "ERROR: possible live OpenAI API key pattern found:" >&2
  rg -n --glob '!target/**' --glob '!**/*.lock' -e 'sk-[a-zA-Z0-9]{20,}' . \
    | grep -v 'sk-test-' || true
  fail=1
fi

if find . \( -name '*.pem' -o -path '*/secrets/*' \) | grep -v '^\./target' | grep -q .; then
  echo "ERROR: pem or secrets/ path found" >&2
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "OK: no obvious secrets in tree"
