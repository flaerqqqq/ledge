#!/usr/bin/env bash
# Gate (docs/testing.md level 6): fmt, clippy, build, test, the installer tests, the ignored tmux integration tests when
# tmux is on PATH, shellcheck when it is on PATH, and the layer check. Exits non-zero on the first failure.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets --locked -- -D warnings

echo "==> cargo build --locked"
cargo build --locked

echo "==> cargo test --locked"
cargo test --locked

echo "==> tmux integration tests (ignored)"
if command -v tmux >/dev/null 2>&1; then
    # Isolated tmux server: never the user's default server (testing.md). Killed on exit.
    LEDGE_TMUX_SERVER="ledge-gate-$$"
    export LEDGE_TMUX_SERVER
    kill_gate_server() {
        tmux -L "$LEDGE_TMUX_SERVER" kill-server >/dev/null 2>&1 || true
    }
    trap kill_gate_server EXIT
    # The test process and the spawned binary must not see a surrounding tmux session.
    unset TMUX
    cargo test --locked -- --ignored
else
    echo "SKIP: tmux is not on PATH; ignored tmux integration tests not run"
fi

echo "==> scripts/test_install.sh"
if [[ -f "$root/scripts/test_install.sh" ]]; then
    bash "$root/scripts/test_install.sh"
else
    echo "SKIP: scripts/test_install.sh does not exist"
fi

echo "==> shellcheck scripts/*.sh"
if command -v shellcheck >/dev/null 2>&1; then
    shellcheck "$root"/scripts/*.sh
else
    echo "SKIP: shellcheck is not on PATH"
fi

echo "==> scripts/check_layers.sh"
bash "$root/scripts/check_layers.sh"

echo "gate: all checks passed"
