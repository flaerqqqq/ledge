#!/usr/bin/env bash
# Layer check (architecture 3): grep-based dependency rules over src/.
# Runs from any root that holds a src/ folder (the gate runs it from the repo root).
# Lines that are comments are ignored. Exit 1 on any violation, 0 otherwise.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src="$root/src"

if [[ ! -d "$src" ]]; then
    echo "check_layers: no src/ folder under $root" >&2
    exit 1
fi

violations=0

# check <folder under src> <extended regex> <rule text> [exclude regex]
check() {
    local dir="$1" pattern="$2" rule="$3" exclude="${4:-^\$}" hits line
    [[ -d "$src/$dir" ]] || return 0
    hits="$(grep -rnE --include='*.rs' -e "$pattern" "$src/$dir" \
        | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//' \
        | grep -vE "$exclude" || true)"
    [[ -n "$hits" ]] || return 0
    while IFS= read -r line; do
        echo "layer violation ($rule): ${line#"$root/"}" >&2
    done <<< "$hits"
    violations=$((violations + 1))
}

# domain: pure logic, no I/O, no other layers, no ratatui (architecture 3)
check domain 'std::(fs|env|process|io|path)\b|crate::(ports|app|tui|adapters|cli)\b|ratatui|crossterm' \
    "domain must not use std::fs/env/process/io/path or other layers"

# ports: traits only, no adapters, app or tui
check ports 'crate::(app|tui|adapters|cli)\b' \
    "ports must not use app, tui, adapters or cli"

# tui: no std fs/process/env, no app, adapters or cli; ports only the signals module (section 3)
check tui 'std::(fs|process|env)\b|crate::(app|adapters|cli)\b|crate::ports\b' \
    "tui must not use std::fs/process/env, app, adapters, cli or ports other than signals" \
    'crate::ports::signals\b'

# app: goes through ports and tui, never names an adapter, ratatui or the cli
check app 'crate::(adapters|cli)\b|ratatui|crossterm' \
    "app must not use adapters, cli or ratatui directly"

# cli: parses and routes to app, never names an adapter
check cli 'crate::adapters\b' \
    "cli must not use adapters"

# adapters: implement ports, never call app or cli
check adapters 'crate::(app|cli)\b' \
    "adapters must not use app or cli"

if [[ "$violations" -gt 0 ]]; then
    echo "check_layers: $violations rule(s) violated" >&2
    exit 1
fi
echo "check_layers: ok"
