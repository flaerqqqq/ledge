//! Component tests for the bookmark edit window state and listing (docs/testing.md level 2): the `bm-edit-open`
//! file through `RealFs` on temp folders, and the folder listing of a project on the real file system.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ledge::adapters::fs::RealFs;
use ledge::app::bookmark_cmd::edit;
use ledge::app::build::ensure_session;
use ledge::app::store::{load_expanded, save_expanded};
use ledge::cli::run;
use ledge::domain::bm_edit::{visible_rows, visible_rows_with_hidden, ExpandedFolders};
use ledge::domain::bookmarks::Bookmarks;
use ledge::domain::location::Dirs;
use ledge::domain::naming::session_name;
use ledge::domain::path::AbsPath;
use ledge::domain::profile::Profile;
use ledge::error::{LedgeError, Outcome, Result};
use ledge::ports::fs::FileSystem;
use ledge::tui::terminal::Input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::{Path, PathBuf};
use support::ctx::Harness;
use support::temp::TempTree;

mod support;

/// Dirs with the XDG cache base set to `base`, so the cache folder is `base/ledge` (architecture 9.1).
fn dirs_with_cache(base: &Path) -> Dirs {
    Dirs::resolve(
        Some(base.join("config").to_str().expect("utf-8 temp path")),
        Some(base.to_str().expect("utf-8 temp path")),
        None,
    )
    .expect("dirs resolve")
}

fn cache_folder(base: &Path) -> PathBuf {
    base.join("ledge")
}

fn abs(text: &str) -> AbsPath {
    AbsPath::clean(text, &AbsPath::root())
}

/// Writes a file in the cache folder as a hand-edited file would be.
fn seed_cache(base: &Path, name: &str, text: &str) {
    fs::create_dir_all(cache_folder(base)).expect("create cache folder");
    fs::write(cache_folder(base).join(name), text).expect("seed cache file");
}

fn expanded_file_text(base: &Path) -> String {
    fs::read_to_string(cache_folder(base).join("bm-edit-open")).expect("read bm-edit-open")
}

fn folder(root: &Path, rel: &str) {
    fs::create_dir_all(root.join(rel)).expect("create folder");
}

/// The child folder names of `dir` on the real file system (what the edit window lists).
fn real_children(dir: &AbsPath) -> Vec<String> {
    RealFs
        .list_dirs(Path::new(dir.as_str()))
        .unwrap_or_default()
}

// FR-EDIT-05: a missing bm-edit-open file loads as no folder expanded
#[test]
fn missing_expanded_file_loads_as_nothing_expanded() {
    let tree = TempTree::new("component_edit-missing");
    let dirs = dirs_with_cache(tree.path());
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    assert!(expanded.entries().is_empty());
}

// FR-EDIT-05: the expanded folders saved by one run are the ones the next run loads
#[test]
fn expanded_folders_persist_between_runs() {
    let tree = TempTree::new("component_edit-persist");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app/src");
    let app = format!("{}/app", tree.path().display());
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(&app));
    expanded.expand(abs(&format!("{app}/src")));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    assert_eq!(
        expanded_file_text(tree.path()),
        format!("{app}\n{app}/src\n")
    );
    assert_eq!(load_expanded(&RealFs, &dirs).expect("load"), expanded);
}

// FR-EDIT-05, D-09: when the file is written, folders that no longer exist are dropped and the others kept
#[test]
fn save_prunes_folders_that_no_longer_exist() {
    let tree = TempTree::new("component_edit-prune");
    let dirs = dirs_with_cache(tree.path());
    let project = tree.path().join("app");
    folder(tree.path(), "app/src");
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(project.to_str().expect("utf-8")));
    expanded.expand(abs(&format!("{}/src", project.display())));
    expanded.expand(abs(&format!("{}/gone", project.display())));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    assert_eq!(
        expanded_file_text(tree.path()),
        format!("{0}\n{0}/src\n", project.display())
    );
}

// D-09: collapsing a folder removes only its entry; the descendants stay expanded for the next time
#[test]
fn collapsed_parent_keeps_expanded_descendants_after_save() {
    let tree = TempTree::new("component_edit-collapse");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app/src/main");
    let app = format!("{}/app", tree.path().display());
    let src = format!("{app}/src");
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(&app));
    expanded.expand(abs(&src));
    expanded.collapse(&abs(&app));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    assert_eq!(expanded_file_text(tree.path()), format!("{src}\n"));
    assert!(load_expanded(&RealFs, &dirs)
        .expect("load")
        .is_expanded(&abs(&src)));
}

// FR-EDIT-05, D-05, architecture 9.3: saving over an older file leaves exactly the new text and no temp file
#[test]
fn save_replaces_old_text_and_leaves_no_temp_file() {
    let tree = TempTree::new("component_edit-atomic");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app");
    let app = format!("{}/app", tree.path().display());
    seed_cache(
        tree.path(),
        "bm-edit-open",
        "/old/one\n/old/two\n/old/three\n",
    );
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(&app));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    assert_eq!(expanded_file_text(tree.path()), format!("{app}\n"));
    let names = RealFs.list_files(&cache_folder(tree.path())).expect("list");
    assert_eq!(names, vec!["bm-edit-open".to_string()]);
}

// A-09, D-06: a hand-edited file with relative lines, a trailing slash and a repeat is cleaned on read
#[test]
fn hand_edited_expanded_file_is_cleaned_on_read() {
    let tree = TempTree::new("component_edit-clean");
    let dirs = dirs_with_cache(tree.path());
    seed_cache(tree.path(), "bm-edit-open", "app/src\n/p/app/\n/p/app\n");
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    let texts: Vec<&str> = expanded.entries().iter().map(AbsPath::as_str).collect();
    assert_eq!(texts, vec!["/p/app"]);
}

// FR-EDIT-01, FR-EDIT-04: the listing of a real project shows its subfolders in name order, skips hidden
// folders, lists the children of expanded folders only, and marks them "+" or "-" (a leaf has no marker)
#[test]
fn real_listing_shows_visible_subfolders_with_markers() {
    let tree = TempTree::new("component_edit-listing");
    let dirs = dirs_with_cache(tree.path());
    let base = tree.path();
    folder(base, "app/src/main");
    folder(base, "app/docs");
    folder(base, "app/.git");
    fs::write(base.join("app/README.md"), "text").expect("write file");
    let app = abs(&format!("{}/app", base.display()));
    let mut expanded = ExpandedFolders::default();
    expanded.expand(abs(&format!("{}/src", app.as_str())));
    save_expanded(&RealFs, &dirs, &expanded).expect("save");
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    let bookmarks = Bookmarks::parse(&format!("{}/app/docs\n", base.display()));

    let rows = visible_rows(&app, &expanded, &bookmarks, &real_children);
    let shown: Vec<(&str, &str, bool)> = rows
        .iter()
        .map(|row| (row.name.as_str(), row.marker.text(), row.bookmarked))
        .collect();
    assert_eq!(
        shown,
        vec![
            ("app", "-", false),
            ("docs", "", true),
            ("src", "-", false),
            ("main", "", false),
        ]
    );
}

// FR-EDIT-01, S-44: each row counts its direct visible subfolders, whether it is expanded or not; hidden folders and
// files are not counted, and the grandchildren are not counted in the parent's count
#[test]
fn child_count_is_the_visible_direct_subfolders() {
    let tree = TempTree::new("component_edit-child-count");
    let base = tree.path();
    folder(base, "app/src/main");
    folder(base, "app/docs");
    folder(base, "app/.git");
    fs::write(base.join("app/README.md"), "text").expect("write file");
    let app = abs(&format!("{}/app", base.display()));
    let rows = visible_rows(
        &app,
        &ExpandedFolders::default(),
        &Bookmarks::default(),
        &real_children,
    );
    let counts: Vec<(&str, usize)> = rows
        .iter()
        .map(|row| (row.name.as_str(), row.subfolders))
        .collect();
    assert_eq!(counts, vec![("app", 2), ("docs", 0), ("src", 1)]);
}

// FR-EDIT-04: a folder with subfolders that is not expanded shows "+"
#[test]
fn collapsed_folder_with_subfolders_shows_plus() {
    let tree = TempTree::new("component_edit-plus");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app/src/main");
    let app = abs(&format!("{}/app", tree.path().display()));
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    let rows = visible_rows(&app, &expanded, &Bookmarks::default(), &real_children);
    let shown: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| (row.name.as_str(), row.marker.text()))
        .collect();
    assert_eq!(shown, vec![("app", "-"), ("src", "+")]);
}

// FR-EDIT-11: a project without subfolders gives no rows (the window does not open); hidden folders do not count
#[test]
fn project_without_visible_subfolders_gives_no_rows() {
    let tree = TempTree::new("component_edit-empty");
    let dirs = dirs_with_cache(tree.path());
    folder(tree.path(), "app/.git");
    fs::write(tree.path().join("app/main.rs"), "fn main() {}").expect("write file");
    let app = abs(&format!("{}/app", tree.path().display()));
    let expanded = load_expanded(&RealFs, &dirs).expect("load");
    assert!(visible_rows(&app, &expanded, &Bookmarks::default(), &real_children).is_empty());
}

/// The text of a command that must fail.
fn fatal(result: Result<Outcome>) -> String {
    match result {
        Err(LedgeError::Fatal(text)) => text,
        other => panic!("expected a fatal error, got {other:?}"),
    }
}

fn esc() -> Input {
    Input::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
}

/// The label of the window the terminal drew last, if a window opened.
fn window_label(h: &Harness) -> Option<String> {
    let record = h.terminal.last_record()?;
    let lines = record.borrow().lines.clone();
    lines
        .into_iter()
        .find(|line| line.contains("Bookmarks in "))
}

/// A search path with one project `app` (`src/main`, `docs`, hidden `.git`), and `h`'s working directory in it.
fn project_app(h: &Harness) -> String {
    let app = h.project("app");
    fs::create_dir_all(format!("{app}/src/main")).expect("create src/main");
    fs::create_dir_all(format!("{app}/docs")).expect("create docs");
    fs::create_dir_all(format!("{app}/.git")).expect("create .git");
    let projects = h.root().join("projects").display().to_string();
    h.write_config(&format!("{{\"search_paths\": [\"{projects}\"]}}"));
    h.terminal.set_size(80, 24);
    app
}

// FR-EDIT-08: with no directory the project is the one containing the working directory
#[test]
fn no_directory_chooses_the_project_of_the_working_directory() {
    let mut h = Harness::new("component_edit-cwd-project");
    let app = project_app(&h);
    h.cwd = ledge::domain::path::AbsPath::clean(
        &format!("{app}/src/main"),
        &ledge::domain::path::AbsPath::root(),
    );
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), None, false), Ok(Outcome::Done));
    assert!(window_label(&h).is_some_and(|line| line.contains("Bookmarks in app")));
}

// FR-EDIT-08: a given directory chooses the project that contains it, and a project contains itself
#[test]
fn given_directory_chooses_its_project_and_a_project_contains_itself() {
    let h = Harness::new("component_edit-given-project");
    let app = project_app(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(
        edit(&h.ctx(), Some(&format!("{app}/docs")), false),
        Ok(Outcome::Done)
    );
    assert!(window_label(&h).is_some_and(|line| line.contains("Bookmarks in app")));

    let h = Harness::new("component_edit-self-project");
    let app = project_app(&h);
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    assert!(window_label(&h).is_some_and(|line| line.contains("Bookmarks in app")));
}

// FR-EDIT-10: a directory outside every project fails with the message, and no window opens
#[test]
fn directory_outside_every_project_is_refused() {
    let h = Harness::new("component_edit-outside");
    let _app = project_app(&h);
    let outside = format!("{}/elsewhere", h.root().display());
    fs::create_dir_all(&outside).expect("create outside folder");
    assert_eq!(
        fatal(edit(&h.ctx(), Some(&outside), false)),
        format!("{outside} is not inside a project from search_paths.")
    );
    assert!(h.terminal.last_record().is_none());
}

// FR-EDIT-11: a project with no visible subfolders exits with no output and opens no window
#[test]
fn project_without_subfolders_opens_no_window() {
    let h = Harness::new("component_edit-no-subfolders");
    let projects = h.root().join("projects").display().to_string();
    h.write_config(&format!("{{\"search_paths\": [\"{projects}\"]}}"));
    let empty = h.project("empty");
    fs::write(format!("{empty}/notes.txt"), "x").expect("write a file");
    fs::create_dir_all(format!("{empty}/.hidden")).expect("create hidden folder");
    assert_eq!(edit(&h.ctx(), Some(&empty), false), Ok(Outcome::Done));
    assert!(h.terminal.last_record().is_none());
}

// FR-EDIT-09, D-16: --session-scope outside a tmux session is refused with the message
#[test]
fn session_scope_outside_a_session_is_refused() {
    let h = Harness::new("component_edit-scope-outside");
    let _app = project_app(&h);
    assert_eq!(
        fatal(edit(&h.ctx(), None, true)),
        "--session-scope requires running inside a session."
    );
    assert!(h.terminal.last_record().is_none());
}

// FR-EDIT-09: --session-scope uses the directory of the session the client is in
#[test]
fn session_scope_uses_the_directory_of_the_current_session() {
    let mut h = Harness::new("component_edit-scope-inside");
    let app = project_app(&h);
    let dir = ledge::domain::path::AbsPath::clean(
        &format!("{app}/docs"),
        &ledge::domain::path::AbsPath::root(),
    );
    ensure_session(&h.backend, &dir, &Profile::basic()).expect("create session");
    h.backend.set_current(&session_name(&dir));
    h.env = h.env.clone().with_var("TMUX", "/tmp/tmux-1000/default,1,0");
    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), None, true), Ok(Outcome::Done));
    assert!(window_label(&h).is_some_and(|line| line.contains("Bookmarks in app")));
}

// FR-EDIT-08, FR-EDIT-09: the command reaches the window through the CLI (dispatch of `bm edit`)
#[test]
fn bm_edit_command_opens_the_window_for_the_given_directory() {
    let h = Harness::new("component_edit-cli");
    let app = project_app(&h);
    h.terminal.script(vec![esc()]);
    let args = vec!["bm".to_string(), "edit".to_string(), app.clone()];
    assert_eq!(run(&args, &h.ctx()), Ok(Outcome::Done));
    assert!(window_label(&h).is_some_and(|line| line.contains("Bookmarks in app")));
}

// FR-EDIT-01, P-68: the listing skips hidden folders by default; with the toggle on it lists them in name order too
#[test]
fn hidden_folders_are_listed_only_after_the_toggle() {
    let tree = TempTree::new("component_edit-hidden-listing");
    let base = tree.path();
    folder(base, "app/src/main");
    folder(base, "app/docs");
    folder(base, "app/.git");
    folder(base, "app/.config");
    let app = abs(&format!("{}/app", base.display()));
    let names = |rows: &[ledge::domain::bm_edit::EditRow]| -> Vec<String> {
        rows.iter().map(|row| row.name.clone()).collect()
    };

    let hidden = visible_rows(
        &app,
        &ExpandedFolders::default(),
        &Bookmarks::default(),
        &real_children,
    );
    assert_eq!(names(&hidden), vec!["app", "docs", "src"]);

    let shown = visible_rows_with_hidden(
        &app,
        &ExpandedFolders::default(),
        &Bookmarks::default(),
        &real_children,
        true,
    );
    assert_eq!(names(&shown), vec!["app", ".config", ".git", "docs", "src"]);
    assert_eq!(
        shown[0].subfolders, 4,
        "the root counts the shown subfolders"
    );
}

// P-68: the toggle is not saved: the bm-edit-open file holds only the expanded folders, and a fresh open starts hidden
#[test]
fn hidden_toggle_is_not_saved_and_a_fresh_open_starts_hidden() {
    let h = Harness::new("component_edit-hidden-not-saved");
    h.terminal.set_size(80, 24);
    let app = h.project("app");
    fs::create_dir_all(format!("{app}/docs")).expect("create docs");
    fs::create_dir_all(format!("{app}/.config")).expect("create .config");
    let projects = h.root().join("projects").display().to_string();
    h.write_config(&format!("{{\"search_paths\": [\"{projects}\"]}}"));

    h.terminal.script(vec![
        Input::Key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE)),
        esc(),
    ]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    // No file is written by the toggle; a cache folder that does not exist holds nothing either.
    let cache = PathBuf::from(h.dirs.cache.as_str());
    if let Ok(entries) = fs::read_dir(&cache) {
        for entry in entries.flatten() {
            let text = fs::read_to_string(entry.path()).unwrap_or_default();
            assert!(
                !text.contains(".config"),
                "{} keeps the hidden toggle",
                entry.path().display()
            );
        }
    }

    h.terminal.script(vec![esc()]);
    assert_eq!(edit(&h.ctx(), Some(&app), false), Ok(Outcome::Done));
    let lines = h
        .terminal
        .last_record()
        .expect("second window opened")
        .borrow()
        .lines
        .clone();
    assert!(
        !lines.iter().any(|line| line.contains(".config")),
        "a fresh open is hidden: {lines:#?}"
    );
}
