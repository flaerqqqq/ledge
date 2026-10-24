#!/usr/bin/env bash
# Build ledge from this checkout and install it into the local binary folder, or remove it again with --uninstall.
# Design: docs/architecture.md section 11 (A-21). Wording: requirements.md (Installation and removal) and architecture 8.3 and 11.
set -euo pipefail

fail() {
    echo "Error: $*" >&2
    exit 1
}

mode=install
if [[ $# -eq 1 && "$1" == "--uninstall" ]]; then
    mode=uninstall
elif [[ $# -gt 0 ]]; then
    fail "usage: scripts/install.sh [--uninstall]"
fi

home="${HOME:-}"

bin_dir="${LEDGE_INSTALL_BIN_DIR:-}"
if [[ -z "$bin_dir" ]]; then
    [[ -n "$home" ]] || fail "HOME is not set."
    bin_dir="$home/.local/bin"
fi

state_dir="${LEDGE_INSTALL_STATE_DIR:-}"
if [[ -z "$state_dir" ]]; then
    state_base="${XDG_STATE_HOME:-}"
    if [[ -z "$state_base" ]]; then
        [[ -n "$home" ]] || fail "HOME is not set."
        state_base="$home/.local/state"
    fi
    state_dir="$state_base/ledge"
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$bin_dir/ledge"
record="$state_dir/install-source"

if [[ "$mode" == "uninstall" ]]; then
    if [[ ! -e "$binary" ]]; then
        echo "Nothing to remove: $binary does not exist"
        exit 0
    fi
    # The record names the source folder the binary was installed from. A missing record means
    # the binary is not known to come from this checkout, so it is kept.
    recorded=""
    if [[ -f "$record" ]]; then
        recorded="$(head -n 1 -- "$record")"
    fi
    if [[ "$recorded" != "$root" ]]; then
        echo "Not removed: $binary was not installed from $root" >&2
        exit 1
    fi
    rm -f -- "$binary" "$record"
    echo "Removed $binary"
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    fail "build toolchain is required to build ledge."
fi

cd "$root"
cargo build --release --locked

mkdir -p -- "$bin_dir"
cp -- "$root/target/release/ledge" "$binary"
chmod 0755 -- "$binary"

mkdir -p -- "$state_dir"
printf '%s\n' "$root" > "$record"

echo "Installed $binary"

case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) echo "Warning: $bin_dir is not on PATH; add it to use ledge." >&2 ;;
esac
