#!/usr/bin/env bash
# Layer check (architecture 3): grep-based dependency rules over src/.
# Runs from any root that holds a src/ folder (the gate runs it from the repo root).
# Comments are ignored: a `//` comment (whole line or trailing, also inside a brace group) is
# removed before matching. Brace groups (`use std::{fs, env};`, also over several lines) are joined
# into one line before matching, so they cannot bypass a rule.
# Exit 1 on any violation, 0 otherwise.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src="$root/src"

if [[ ! -d "$src" ]]; then
    echo "check_layers: no src/ folder under $root" >&2
    exit 1
fi

violations=0

# scan_lines <files...>: prints "file:line:text" for every line with its `//` comment removed.
# Lines that are empty after that are dropped. A line opening a brace group (`::{`) is joined with
# the lines up to the matching close brace, and the joined text is printed once, at the line where
# the group opens. Comment-only and blank lines inside a group add nothing to the joined text.
scan_lines() {
    awk '
        FNR == 1 { open = 0 }
        { sub(/\/\/.*$/, "") }
        open || /::\{/ {
            if (!open) { open = 1; start = FNR; buf = ""; depth = 0 }
            if ($0 ~ /^[[:space:]]*$/) next
            buf = buf " " $0
            line = $0
            depth += gsub(/\{/, "{", line) - gsub(/\}/, "}", line)
            if (depth <= 0) { print FILENAME ":" start ":" buf; open = 0 }
            next
        }
        /^[[:space:]]*$/ { next }
        { print FILENAME ":" FNR ":" $0 }
    ' "$@"
}

# rs_files <folder under src>: the .rs files below it, one per line (none if the folder is missing).
rs_files() {
    [[ -d "$src/$1" ]] || return 0
    find "$src/$1" -name '*.rs' -type f
}

# report <rule> <hits>: prints each hit line as a violation and counts the rule once.
report() {
    local rule="$1" hits="$2" line
    [[ -n "$hits" ]] || return 0
    while IFS= read -r line; do
        [[ -n "$line" ]] || continue
        echo "layer violation ($rule): ${line#"$root/"}" >&2
    done <<< "$hits"
    violations=$((violations + 1))
}

# check <folder under src> <extended regex> <rule text>
check() {
    local dir="$1" pattern="$2" rule="$3" matched
    local -a files=()
    mapfile -t files < <(rs_files "$dir")
    [[ ${#files[@]} -gt 0 ]] || return 0
    matched="$(scan_lines "${files[@]}" | grep -E -e "$pattern" || true)"
    report "$rule" "$matched"
}

# tui_ports_only_signals <text>: succeeds when every `;`-separated statement of the text that names
# `ports` imports only the signals module: `ports::signals` as a plain path, or `signals` alone
# (optionally with a trailing comma) in a brace group, as `crate::ports::{signals}` or
# `crate::{ports::signals}`. A mixed group such as `crate::{ports::signals, tui::picker}` fails.
tui_ports_only_signals() {
    local text="$1" part re ok
    local use='^[[:space:]]*(pub[[:space:]]+)?use[[:space:]]+'
    local -a parts exempt
    exempt=(
        "${use}crate::ports::signals([^[:alnum:]_]|\$)"
        "${use}crate::ports::\\{[[:space:]]*signals[[:space:]]*,?[[:space:]]*\\}[[:space:]]*\$"
        "${use}crate::\\{[[:space:]]*ports::signals[[:space:]]*,?[[:space:]]*\\}[[:space:]]*\$"
        "${use}crate::\\{[[:space:]]*ports::\\{[[:space:]]*signals[[:space:]]*,?[[:space:]]*\\}[[:space:]]*,?[[:space:]]*\\}[[:space:]]*\$"
    )
    IFS=';' read -ra parts <<< "$text"
    for part in "${parts[@]}"; do
        re='(^|[^[:alnum:]_])ports([^[:alnum:]_]|$)'
        [[ $part =~ $re ]] || continue
        ok=0
        for re in "${exempt[@]}"; do
            if [[ $part =~ $re ]]; then
                ok=1
                break
            fi
        done
        [[ $ok -eq 1 ]] || return 1
    done
    return 0
}

# Each pattern has a plain path form and a brace form (`std::{...}`, `crate::{...}`), where a
# forbidden name anywhere inside the group matches.

# domain: pure logic, no I/O, no other layers, no ratatui (architecture 3)
check domain 'std::(fs|env|process|io|path)\b|std::\{.*\b(fs|env|process|io|path)\b|crate::(ports|app|tui|adapters|cli)\b|crate::\{.*\b(ports|app|tui|adapters|cli)\b|ratatui|crossterm' \
    "domain must not use std::fs/env/process/io/path or other layers"

# ports: traits only, no adapters, app or tui
check ports 'crate::(app|tui|adapters|cli)\b|crate::\{.*\b(app|tui|adapters|cli)\b' \
    "ports must not use app, tui, adapters or cli"

# tui: no std fs/process/env, no app, adapters or cli (section 3)
check tui 'std::(fs|process|env)\b|std::\{.*\b(fs|process|env)\b|crate::(app|adapters|cli)\b|crate::\{.*\b(app|adapters|cli)\b' \
    "tui must not use std::fs/process/env, app, adapters or cli"

# tui: ports only the signals module. Each use statement that names ports is judged on its own
# (see tui_ports_only_signals), so a mixed group or a second statement on the line cannot hide one.
tui_ports() {
    local matched line text hits=""
    local -a files=()
    mapfile -t files < <(rs_files tui)
    [[ ${#files[@]} -gt 0 ]] || return 0
    matched="$(scan_lines "${files[@]}" | grep -E -e 'crate::(ports\b|\{.*\bports\b)' || true)"
    while IFS= read -r line; do
        [[ -n "$line" ]] || continue
        text="${line#*:*:}"
        if ! tui_ports_only_signals "$text"; then
            hits+="$line"$'\n'
        fi
    done <<< "$matched"
    report "tui must not use ports other than signals" "$hits"
}
tui_ports

# app: goes through ports and tui, never names an adapter, ratatui or the cli
check app 'crate::(adapters|cli)\b|crate::\{.*\b(adapters|cli)\b|ratatui|crossterm' \
    "app must not use adapters, cli or ratatui directly"

# cli: parses and routes to app, never names an adapter
check cli 'crate::adapters\b|crate::\{.*\badapters\b' \
    "cli must not use adapters"

# adapters: implement ports, never call app or cli
check adapters 'crate::(app|cli)\b|crate::\{.*\b(app|cli)\b' \
    "adapters must not use app or cli"

if [[ "$violations" -gt 0 ]]; then
    echo "check_layers: $violations rule(s) violated" >&2
    exit 1
fi
echo "check_layers: ok"
