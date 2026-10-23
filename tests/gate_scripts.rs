//! S-00 acceptance for the gate scripts (no FR-ID: the backlog states these as slice acceptance).
//! `scripts/check_layers.sh` is run from a scratch tree that holds a copy of the script and a
//! `src/` folder, so the planted-violation cases do not depend on the real sources.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Fresh scratch tree under CARGO_TARGET_TMPDIR with a copy of check_layers.sh and the given files.
fn scratch_tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("scripts")).expect("create scripts dir");
    let script = project_root().join("scripts/check_layers.sh");
    fs::copy(&script, root.join("scripts/check_layers.sh")).unwrap_or_else(|e| {
        panic!(
            "cannot copy {} (S-00 deliverable missing): {e}",
            script.display()
        )
    });
    for (rel, body) in files {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("file has a parent")).expect("create src dir");
        fs::write(&path, body).expect("write source file");
    }
    root
}

fn run_check_layers(root: &Path) -> ExitStatus {
    Command::new("bash")
        .arg(root.join("scripts/check_layers.sh"))
        .current_dir(root)
        .env_remove("TMUX")
        .status()
        .expect("run check_layers.sh")
}

// S-00 acceptance: gate.sh exists and is executable
#[test]
fn gate_script_exists_and_is_executable() {
    let gate = project_root().join("scripts/gate.sh");
    assert!(gate.is_file(), "missing {}", gate.display());
    assert!(is_executable(&gate), "not executable: {}", gate.display());
}

// S-00 acceptance: check_layers.sh exists and is executable
#[test]
fn check_layers_script_exists_and_is_executable() {
    let script = project_root().join("scripts/check_layers.sh");
    assert!(script.is_file(), "missing {}", script.display());
    assert!(
        is_executable(&script),
        "not executable: {}",
        script.display()
    );
}

// S-00 acceptance: check_layers.sh passes on a clean domain module (no false positives)
#[test]
fn check_layers_passes_on_clean_domain_module() {
    let root = scratch_tree(
        "layers-clean",
        &[("src/domain/clean.rs", "pub fn value() -> u8 {\n    1\n}\n")],
    );
    assert!(run_check_layers(&root).success());
}

// architecture 3: domain must not use std::fs. S-00 acceptance: fails on a planted violation.
#[test]
fn check_layers_fails_on_planted_domain_std_fs_import() {
    let root = scratch_tree(
        "layers-domain-fs",
        &[(
            "src/domain/planted.rs",
            "use std::fs;\npub fn read() {\n    let _ = fs::read_to_string(\"x\");\n}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// architecture 3: app must not name adapters. S-00 acceptance: fails on a planted violation.
#[test]
fn check_layers_fails_on_planted_app_adapter_import() {
    let root = scratch_tree(
        "layers-app-adapter",
        &[(
            "src/app/planted.rs",
            "use crate::adapters::tmux;\npub fn go() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// architecture 3: a brace group must not hide std::fs from the domain rule.
#[test]
fn check_layers_fails_on_planted_domain_brace_std_import() {
    let root = scratch_tree(
        "layers-domain-brace-fs",
        &[(
            "src/domain/planted.rs",
            "use std::{fs, env};\npub fn read() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// architecture 3: a brace group spread over several lines is still joined and checked.
#[test]
fn check_layers_fails_on_planted_multiline_domain_brace_std_import() {
    let root = scratch_tree(
        "layers-domain-brace-multiline",
        &[(
            "src/domain/planted.rs",
            "use std::{\n    collections::HashMap,\n    fs,\n};\npub fn read() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// architecture 3: a brace group must not hide crate::adapters from the app rule.
#[test]
fn check_layers_fails_on_planted_app_brace_adapter_import() {
    let root = scratch_tree(
        "layers-app-brace-adapter",
        &[(
            "src/app/planted.rs",
            "use crate::{adapters::tmux, domain};\npub fn go() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// section 3: tui may use ports only for the signals module, also inside a brace group.
#[test]
fn check_layers_fails_on_planted_tui_brace_ports_import() {
    let root = scratch_tree(
        "layers-tui-brace-ports",
        &[(
            "src/tui/planted.rs",
            "use crate::{ports::fs};\npub fn draw() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// no false positives: harmless brace groups and the signals module pass.
#[test]
fn check_layers_passes_on_clean_brace_imports() {
    let root = scratch_tree(
        "layers-brace-clean",
        &[
            (
                "src/domain/clean.rs",
                "use std::{collections::HashMap, fmt};\npub fn f() {}\n",
            ),
            (
                "src/tui/clean.rs",
                "use crate::{ports::signals};\npub fn draw() {}\n",
            ),
        ],
    );
    assert!(run_check_layers(&root).success());
}

// section 3: the signals module passes as a plain path too.
#[test]
fn check_layers_passes_on_plain_tui_signals_import() {
    let root = scratch_tree(
        "layers-tui-signals-plain",
        &[(
            "src/tui/clean.rs",
            "use crate::ports::signals;\npub fn draw() {}\n",
        )],
    );
    assert!(run_check_layers(&root).success());
}

// section 3: a group that mixes signals with another item is still checked (fails).
#[test]
fn check_layers_fails_on_planted_tui_mixed_group_ports_import() {
    let root = scratch_tree(
        "layers-tui-mixed-group",
        &[(
            "src/tui/planted.rs",
            "use crate::{ports::signals, tui::picker};\npub fn draw() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// section 3: the reverse order of a mixed group is checked too (fails).
#[test]
fn check_layers_fails_on_planted_tui_mixed_group_reverse_order_ports_import() {
    let root = scratch_tree(
        "layers-tui-mixed-group-reverse",
        &[(
            "src/tui/planted.rs",
            "use crate::{tui::picker, ports::signals};\npub fn draw() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// section 3: a multi-line mixed group is joined and checked (fails).
#[test]
fn check_layers_fails_on_planted_tui_multiline_mixed_group_ports_import() {
    let root = scratch_tree(
        "layers-tui-mixed-group-multiline",
        &[(
            "src/tui/planted.rs",
            "use crate::{\n    ports::signals,\n    tui::picker,\n};\npub fn draw() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// section 3: a second statement on the same line is checked on its own (fails).
#[test]
fn check_layers_fails_on_planted_tui_second_statement_ports_import() {
    let root = scratch_tree(
        "layers-tui-second-statement",
        &[(
            "src/tui/planted.rs",
            "use crate::ports::signals; use crate::ports::fs;\npub fn draw() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// comments: a brace group opened with a trailing comment, comment lines inside a group, and a
// trailing comment after a use statement are all ignored (clean tree passes).
#[test]
fn check_layers_passes_on_comments_inside_and_after_imports() {
    let root = scratch_tree(
        "layers-comments-clean",
        &[(
            "src/domain/clean.rs",
            "use std::{ // reads fs later\n    collections::HashMap,\n    // fs is not used here\n    fmt,\n};\nuse crate::domain::x; // ports\npub fn f() {}\n",
        )],
    );
    assert!(run_check_layers(&root).success());
}

// comments: a trailing comment does not hide a real violation (fails).
#[test]
fn check_layers_fails_on_planted_violation_with_trailing_comment() {
    let root = scratch_tree(
        "layers-trailing-comment-violation",
        &[(
            "src/domain/planted.rs",
            "use crate::ports::Thing; // pure logic\npub fn f() {}\n",
        )],
    );
    assert!(!run_check_layers(&root).success());
}

// S-00 acceptance: gate.sh exits non-zero on any failure. The first step (`cargo fmt --check`)
// is made to fail by a stub `cargo` placed first on PATH. The stub never runs the real cargo, so
// the test cannot recurse into the real gate.
#[test]
fn gate_exits_non_zero_when_a_step_fails() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("gate-failing-step");
    let _ = fs::remove_dir_all(&root);
    let stub_dir = root.join("stubs");
    fs::create_dir_all(&stub_dir).expect("create stub dir");
    fs::create_dir_all(root.join("scripts")).expect("create scripts dir");
    for name in ["gate.sh", "check_layers.sh"] {
        let script = project_root().join("scripts").join(name);
        fs::copy(&script, root.join("scripts").join(name)).unwrap_or_else(|e| {
            panic!(
                "cannot copy {} (S-00 deliverable missing): {e}",
                script.display()
            )
        });
    }
    let stub = stub_dir.join("cargo");
    fs::write(
        &stub,
        "#!/usr/bin/env bash\nif [ \"$1\" = fmt ]; then exit 1; fi\nexit 0\n",
    )
    .expect("write cargo stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod cargo stub");

    let path = format!(
        "{}:{}",
        stub_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new("bash")
        .arg(root.join("scripts/gate.sh"))
        .current_dir(&root)
        .env("PATH", path)
        .env_remove("TMUX")
        .output()
        .expect("run gate.sh");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(1),
        "gate.sh must exit 1 when fmt fails; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("==> cargo fmt --check"),
        "gate.sh must reach the fmt step; stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("==> cargo clippy"),
        "gate.sh must stop at the failing fmt step; stdout:\n{stdout}"
    );
}
