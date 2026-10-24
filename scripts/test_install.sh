#!/usr/bin/env bash
# Installer tests (architecture 11, A-21; testing.md section 6). Each case runs a copy of install.sh placed in its own
# source folder under a temp folder, with a fake cargo first on PATH and HOME, the bin folder and the state folder all
# under that temp folder. Nothing here touches the real ~/.local, ~/.config or target/ of this checkout.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bash_bin="$(command -v bash)"
tmp="$(mktemp -d)"
trap 'rm -rf -- "$tmp"' EXIT

pass=0
failures=0
ok() {
    pass=$((pass + 1))
}
bad() {
    failures=$((failures + 1))
    echo "  FAIL: $*" >&2
}
expect_eq() { # expect_eq <label> <expected> <actual>
    if [[ "$2" == "$3" ]]; then ok; else bad "$1: expected [$2], got [$3]"; fi
}
expect_has() { # expect_has <label> <needle> <haystack>
    if [[ "$3" == *"$2"* ]]; then ok; else bad "$1: [$2] not found in [$3]"; fi
}
expect_lacks() { # expect_lacks <label> <needle> <haystack>
    if [[ "$3" != *"$2"* ]]; then ok; else bad "$1: [$2] unexpectedly found in [$3]"; fi
}
expect_true() { # expect_true <label> <command...>
    local label="$1"
    shift
    if "$@"; then ok; else bad "$label"; fi
}

# Fake cargo: logs its arguments and writes a stub release binary under the current folder's target/.
fake_bin="$tmp/fake-bin"
mkdir -p -- "$fake_bin"
cat > "$fake_bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$FAKE_CARGO_LOG"
mkdir -p -- target/release
printf '#!/usr/bin/env bash\necho ledge stub\n' > target/release/ledge
chmod 0755 target/release/ledge
EOF
chmod 0755 "$fake_bin/cargo"
std_path="$fake_bin:/usr/bin:/bin"

# new_case <name>: sets the case folders and a fresh copy of install.sh as the source checkout.
new_case() {
    echo "== $1"
    case_dir="$tmp/$1"
    case_src="$case_dir/src"
    case_home="$case_dir/home"
    bin="$case_dir/bin"
    state="$case_dir/state"
    case_log="$case_dir/cargo.log"
    mkdir -p -- "$case_src/scripts" "$case_home"
    cp -- "$root/scripts/install.sh" "$case_src/scripts/install.sh"
}

# run_install <PATH> [args...]: runs the installer copy of the current case with a clean environment.
# Sets rc, out and err.
run_install() {
    local path_value="$1"
    shift
    rc=0
    env -i HOME="$case_home" PATH="$path_value" \
        LEDGE_INSTALL_BIN_DIR="$bin" LEDGE_INSTALL_STATE_DIR="$state" FAKE_CARGO_LOG="$case_log" \
        "$bash_bin" "$case_src/scripts/install.sh" "$@" >"$case_dir/out" 2>"$case_dir/err" || rc=$?
    out="$(cat -- "$case_dir/out")"
    err="$(cat -- "$case_dir/err")"
}

# 1. install builds, copies, records the source and reports it
new_case install_copies_and_records
run_install "$std_path"
expect_eq "install exit code" 0 "$rc"
expect_true "binary copied and executable" test -x "$bin/ledge"
expect_has "binary is the built stub" "ledge stub" "$(cat -- "$bin/ledge")"
expect_eq "source record names this checkout" "$case_src" "$(cat -- "$state/install-source")"
expect_eq "cargo build argument" "build --release --locked" "$(cat -- "$case_log")"
expect_eq "install message" "Installed $bin/ledge" "$out"

# 2. PATH warning when the bin folder is not on PATH
new_case path_warning_when_missing
run_install "$std_path"
expect_eq "warning exit code" 0 "$rc"
expect_eq "PATH warning text" "Warning: $bin is not on PATH; add it to use ledge." "$err"

# 3. no PATH warning when the bin folder is on PATH
new_case no_warning_when_on_path
run_install "$fake_bin:$bin:/usr/bin:/bin"
expect_eq "on-PATH exit code" 0 "$rc"
expect_lacks "no PATH warning" "not on PATH" "$err"

# 4. defaults resolve under HOME (no overrides; XDG_STATE_HOME unset)
new_case default_folders_under_home
rc=0
env -i HOME="$case_home" PATH="$std_path" FAKE_CARGO_LOG="$case_log" \
    "$bash_bin" "$case_src/scripts/install.sh" >/dev/null 2>&1 || rc=$?
expect_eq "default install exit code" 0 "$rc"
expect_true "default bin folder under HOME" test -x "$case_home/.local/bin/ledge"
expect_eq "default state record under HOME" "$case_src" "$(cat -- "$case_home/.local/state/ledge/install-source")"

# 5. uninstall removes its own binary and record, keeps config, profiles, history and bookmarks
new_case uninstall_removes_own_binary
run_install "$std_path"
mkdir -p -- "$case_home/.config/ledge"
printf 'search_paths = []\n' > "$case_home/.config/ledge/config.toml"
run_install "$std_path" --uninstall
expect_eq "uninstall exit code" 0 "$rc"
expect_eq "removed message" "Removed $bin/ledge" "$out"
expect_true "binary removed" test ! -e "$bin/ledge"
expect_true "record removed" test ! -e "$state/install-source"
expect_eq "config untouched" "search_paths = []" "$(cat -- "$case_home/.config/ledge/config.toml")"

# 6. uninstall refuses a binary whose record names another source, or has no record
new_case uninstall_refuses_other_source
run_install "$std_path"
printf '%s\n' "/elsewhere/ledge-checkout" > "$state/install-source"
run_install "$std_path" --uninstall
expect_eq "refusal exit code" 1 "$rc"
expect_eq "refusal message" "Not removed: $bin/ledge was not installed from $case_src" "$err"
expect_true "binary kept after refusal" test -x "$bin/ledge"
expect_true "record kept after refusal" test -f "$state/install-source"

new_case uninstall_refuses_missing_record
run_install "$std_path"
rm -f -- "$state/install-source"
run_install "$std_path" --uninstall
expect_eq "missing record exit code" 1 "$rc"
expect_eq "missing record message" "Not removed: $bin/ledge was not installed from $case_src" "$err"
expect_true "binary kept without record" test -x "$bin/ledge"

# 7. nothing to remove when no binary is present
new_case nothing_to_remove
run_install "$std_path" --uninstall
expect_eq "nothing-to-remove exit code" 0 "$rc"
expect_eq "nothing-to-remove message" "Nothing to remove: $bin/ledge does not exist" "$out"

# 8. missing build toolchain: only the tools the script needs, no cargo
new_case missing_toolchain
no_cargo="$case_dir/no-cargo-bin"
mkdir -p -- "$no_cargo"
for tool in dirname mkdir cp chmod rm head; do
    ln -s -- "$(command -v "$tool")" "$no_cargo/$tool"
done
run_install "$no_cargo"
expect_eq "missing toolchain exit code" 1 "$rc"
expect_eq "missing toolchain message" "Error: build toolchain is required to build ledge." "$err"
expect_true "no binary without toolchain" test ! -e "$bin/ledge"
expect_true "cargo never ran without toolchain" test ! -e "$case_log"

# 9. unknown arguments are refused
new_case unknown_argument
run_install "$std_path" --bogus
expect_eq "unknown argument exit code" 1 "$rc"
expect_eq "unknown argument message" "Error: usage: scripts/install.sh [--uninstall]" "$err"

echo
echo "installer tests: $pass passed, $failures failed"
if [[ "$failures" -ne 0 ]]; then
    exit 1
fi
