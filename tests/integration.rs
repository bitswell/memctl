use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn memctl() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_memctl"));
    cmd.env("RUSTFLAGS", "");
    cmd
}

fn fixtures(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn temp_copy(fixture: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let src = fixtures(fixture);
    for entry in fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        let dest = tmp.path().join(entry.file_name());
        fs::copy(entry.path(), dest).unwrap();
    }
    tmp
}

// --- validate ---

#[test]
fn validate_valid_files() {
    let out = memctl()
        .args(["--path", fixtures("valid").to_str().unwrap(), "validate"])
        .output()
        .unwrap();
    assert!(out.status.success(), "expected exit 0");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("validated"), "stderr: {}", stderr);
}

#[test]
fn validate_invalid_files_exits_1() {
    let out = memctl()
        .args(["--path", fixtures("invalid").to_str().unwrap(), "validate"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "expected exit 1 for invalid files");
}

// --- index ---

#[test]
fn index_generates_memory_md() {
    let tmp = temp_copy("valid");
    let out = memctl()
        .args(["--path", tmp.path().to_str().unwrap(), "index"])
        .output()
        .unwrap();
    assert!(out.status.success(), "exit: {:?}\nstderr: {}", out.status, String::from_utf8_lossy(&out.stderr));

    let content = fs::read_to_string(tmp.path().join("MEMORY.md")).unwrap();

    // Superseded and expired entries should be excluded
    assert!(!content.contains("Old auth setup"), "superseded entry should be excluded");
    assert!(!content.contains("Sprint deadline"), "expired entry should be excluded");

    // Active entries should be present
    assert!(content.contains("Alice"));
    assert!(content.contains("Testing preferences"));
    assert!(content.contains("API documentation site"));

    // Tags should appear
    assert!(content.contains("#testing"));
}

#[test]
fn index_sorted_by_type_then_name() {
    let tmp = temp_copy("valid");
    memctl()
        .args(["--path", tmp.path().to_str().unwrap(), "index"])
        .output()
        .unwrap();

    let content = fs::read_to_string(tmp.path().join("MEMORY.md")).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // feedback < project < reference < user (alphabetical by type)
    let feedback_pos = lines.iter().position(|l| l.contains("Testing preferences"));
    let project_pos = lines.iter().position(|l| l.contains("Product launch"));
    let reference_pos = lines.iter().position(|l| l.contains("API documentation"));
    let user_pos = lines.iter().position(|l| l.contains("Alice"));

    assert!(feedback_pos < project_pos, "feedback should come before project");
    assert!(project_pos < reference_pos, "project should come before reference");
    assert!(reference_pos < user_pos, "reference should come before user");
}

// --- show ---

#[test]
fn show_by_name() {
    let out = memctl()
        .args(["--path", fixtures("valid").to_str().unwrap(), "show", "alice"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Alice"));
    assert!(stdout.contains("user"));
}

#[test]
fn show_not_found_exits_2() {
    let out = memctl()
        .args(["--path", fixtures("valid").to_str().unwrap(), "show", "nonexistent"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

// --- dedupe ---

#[test]
fn dedupe_finds_similar() {
    let out = memctl()
        .args(["--path", fixtures("dupes").to_str().unwrap(), "dedupe"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "expected exit 1 when dupes found");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dupe_a.md"), "stdout: {}", stdout);
    assert!(stdout.contains("dupe_b.md"), "stdout: {}", stdout);
}

#[test]
fn dedupe_no_dupes() {
    let out = memctl()
        .args(["--path", fixtures("valid").to_str().unwrap(), "dedupe"])
        .output()
        .unwrap();
    assert!(out.status.success(), "expected exit 0 when no dupes");
}

// --- stale ---

#[test]
fn stale_finds_expired() {
    let out = memctl()
        .args(["--path", fixtures("valid").to_str().unwrap(), "stale"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("expired_memory.md"), "stdout: {}", stdout);
}

// --- gc ---

#[test]
fn gc_dry_run_does_not_delete() {
    let tmp = temp_copy("valid");
    let out = memctl()
        .args(["--path", tmp.path().to_str().unwrap(), "gc", "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("would archive"), "stdout: {}", stdout);

    // Files should still exist
    assert!(tmp.path().join("expired_memory.md").exists());
    assert!(tmp.path().join("superseded_old.md").exists());
}

#[test]
fn gc_archives_expired_and_superseded() {
    let tmp = temp_copy("valid");
    let out = memctl()
        .args(["--path", tmp.path().to_str().unwrap(), "gc"])
        .output()
        .unwrap();
    assert!(out.status.success());

    // Original files should be gone
    assert!(!tmp.path().join("expired_memory.md").exists());
    assert!(!tmp.path().join("superseded_old.md").exists());

    // Should be in archive/
    assert!(tmp.path().join("archive").join("expired_memory.md").exists());
    assert!(tmp.path().join("archive").join("superseded_old.md").exists());
}

// --- supersede ---

#[test]
fn supersede_updates_frontmatter() {
    let tmp = temp_copy("valid");
    let out = memctl()
        .args([
            "--path",
            tmp.path().to_str().unwrap(),
            "supersede",
            "user_alice.md",
            "feedback_testing.md",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let content = fs::read_to_string(tmp.path().join("user_alice.md")).unwrap();
    assert!(content.contains("superseded_by"), "file should have superseded_by field");
    assert!(content.contains("feedback_testing.md"));
}

// --- migrate ---

#[test]
fn migrate_dry_run() {
    let out = memctl()
        .args(["--path", fixtures("valid").to_str().unwrap(), "migrate", "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success());
}
