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
