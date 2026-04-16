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
    assert_eq!(out.status.code(), Some(1), "expected exit 1 when stale files found");
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

// --- extra frontmatter field preservation (fix #3) ---

#[test]
fn supersede_preserves_extra_fields() {
    let tmp = temp_copy("valid");
    memctl()
        .args([
            "--path",
            tmp.path().to_str().unwrap(),
            "supersede",
            "extra_fields.md",
            "feedback_testing.md",
        ])
        .output()
        .unwrap();

    let content = fs::read_to_string(tmp.path().join("extra_fields.md")).unwrap();
    assert!(content.contains("originSessionId"), "originSessionId must survive supersede round-trip");
    assert!(content.contains("customField"), "customField must survive supersede round-trip");
    assert!(content.contains("superseded_by"), "superseded_by must be added");
}

#[test]
fn migrate_preserves_extra_fields() {
    let tmp = temp_copy("valid");
    memctl()
        .args(["--path", tmp.path().to_str().unwrap(), "migrate"])
        .output()
        .unwrap();

    let content = fs::read_to_string(tmp.path().join("extra_fields.md")).unwrap();
    assert!(content.contains("originSessionId"), "originSessionId must survive migrate round-trip");
    assert!(content.contains("customField"), "customField must survive migrate round-trip");
}

// --- mixed valid/invalid files: commands continue with good files (fix #7) ---

#[test]
fn show_works_with_mixed_files() {
    let out = memctl()
        .args(["--path", fixtures("mixed").to_str().unwrap(), "show", "Good memory"])
        .output()
        .unwrap();
    assert!(out.status.success(), "show should succeed despite bad files");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Good memory"));
    // Should warn about the bad file on stderr
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("warn"), "should warn about bad file: {}", stderr);
}

#[test]
fn stale_works_with_mixed_files() {
    let out = memctl()
        .args(["--path", fixtures("mixed").to_str().unwrap(), "stale"])
        .output()
        .unwrap();
    // Should not exit 2 (error) — should handle the bad file gracefully
    assert_ne!(out.status.code(), Some(2), "should not error-exit on mixed files");
}

#[test]
fn dedupe_works_with_mixed_files() {
    let out = memctl()
        .args(["--path", fixtures("mixed").to_str().unwrap(), "dedupe"])
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(2), "should not error-exit on mixed files");
}

// --- index em-dash separator (fix #6) ---

#[test]
fn index_uses_em_dash_separator() {
    let tmp = temp_copy("valid");
    memctl()
        .args(["--path", tmp.path().to_str().unwrap(), "index"])
        .output()
        .unwrap();

    let content = fs::read_to_string(tmp.path().join("MEMORY.md")).unwrap();
    // Every line should use em-dash, not hyphen
    for line in content.lines() {
        assert!(
            line.contains(" \u{2014} "),
            "line should use em-dash separator, got: {}",
            line
        );
        assert!(
            !line.contains(") - "),
            "line must not use hyphen separator, got: {}",
            line
        );
    }
}

// ============================================================
// Contract / tripwire tests (fix #4)
//
// Pin exact exit codes for all 8 subcommands in both success
// and "found issues" cases. Pin stdout format for commands
// whose output Phase 4 will parse.
// ============================================================

mod contract {
    use super::*;

    // --- exit code contracts ---

    #[test]
    fn validate_exit_0_when_all_valid() {
        let out = memctl()
            .args(["--path", fixtures("valid").to_str().unwrap(), "validate"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "validate: exit 0 when all valid");
    }

    #[test]
    fn validate_exit_1_when_invalid() {
        let out = memctl()
            .args(["--path", fixtures("invalid").to_str().unwrap(), "validate"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "validate: exit 1 when invalid files");
    }

    #[test]
    fn index_exit_0_on_success() {
        let tmp = temp_copy("valid");
        let out = memctl()
            .args(["--path", tmp.path().to_str().unwrap(), "index"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "index: exit 0 on success");
    }

    #[test]
    fn dedupe_exit_0_no_dupes() {
        let out = memctl()
            .args(["--path", fixtures("valid").to_str().unwrap(), "dedupe"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "dedupe: exit 0 when no dupes");
    }

    #[test]
    fn dedupe_exit_1_dupes_found() {
        let out = memctl()
            .args(["--path", fixtures("dupes").to_str().unwrap(), "dedupe"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "dedupe: exit 1 when dupes found");
    }

    #[test]
    fn stale_exit_0_no_stale() {
        // Use dupes fixture — no valid_until fields, so nothing stale
        let out = memctl()
            .args(["--path", fixtures("dupes").to_str().unwrap(), "stale"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "stale: exit 0 when nothing stale");
    }

    #[test]
    fn stale_exit_1_stale_found() {
        let out = memctl()
            .args(["--path", fixtures("valid").to_str().unwrap(), "stale"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "stale: exit 1 when stale files found");
    }

    #[test]
    fn gc_exit_0() {
        let tmp = temp_copy("valid");
        let out = memctl()
            .args(["--path", tmp.path().to_str().unwrap(), "gc", "--dry-run"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "gc: exit 0");
    }

    #[test]
    fn supersede_exit_0() {
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
        assert_eq!(out.status.code(), Some(0), "supersede: exit 0 on success");
    }

    #[test]
    fn supersede_exit_2_missing_file() {
        let tmp = temp_copy("valid");
        let out = memctl()
            .args([
                "--path",
                tmp.path().to_str().unwrap(),
                "supersede",
                "nonexistent.md",
                "feedback_testing.md",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "supersede: exit 2 on error");
    }

    #[test]
    fn migrate_exit_0() {
        let out = memctl()
            .args(["--path", fixtures("valid").to_str().unwrap(), "migrate", "--dry-run"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "migrate: exit 0");
    }

    #[test]
    fn show_exit_0_found() {
        let out = memctl()
            .args(["--path", fixtures("valid").to_str().unwrap(), "show", "alice"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "show: exit 0 when found");
    }

    #[test]
    fn show_exit_2_not_found() {
        let out = memctl()
            .args(["--path", fixtures("valid").to_str().unwrap(), "show", "nonexistent"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "show: exit 2 when not found");
    }

    #[test]
    fn error_exit_2_bad_path() {
        let out = memctl()
            .args(["--path", "/nonexistent/path", "validate"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "any command: exit 2 on fatal error");
    }

    // --- stdout format contracts ---

    #[test]
    fn index_output_format() {
        let tmp = temp_copy("valid");
        memctl()
            .args(["--path", tmp.path().to_str().unwrap(), "index"])
            .output()
            .unwrap();

        let content = fs::read_to_string(tmp.path().join("MEMORY.md")).unwrap();
        for line in content.lines() {
            // Format: - [Name](filename.md) \u{2014} description
            assert!(
                line.starts_with("- ["),
                "index line must start with '- [': {}",
                line
            );
            assert!(
                line.contains("]("),
                "index line must contain '](': {}",
                line
            );
            assert!(
                line.contains(".md)"),
                "index line must link to .md file: {}",
                line
            );
            assert!(
                line.contains(" \u{2014} "),
                "index line must use em-dash separator: {}",
                line
            );
        }
    }

    #[test]
    fn dedupe_output_format() {
        let out = memctl()
            .args(["--path", fixtures("dupes").to_str().unwrap(), "dedupe"])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            // Format: 0.XX  file_a.md <-> file_b.md
            assert!(
                line.contains(" <-> "),
                "dedupe line must contain ' <-> ': {}",
                line
            );
        }
    }

    #[test]
    fn stale_output_format() {
        let out = memctl()
            .args(["--path", fixtures("valid").to_str().unwrap(), "stale"])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            // Format: filename.md (expired YYYY-MM-DD)
            assert!(
                line.contains("(expired "),
                "stale line must contain '(expired ': {}",
                line
            );
            assert!(
                line.ends_with(')'),
                "stale line must end with ')': {}",
                line
            );
        }
    }

    #[test]
    fn show_output_format() {
        let out = memctl()
            .args(["--path", fixtures("valid").to_str().unwrap(), "show", "alice"])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Must start with name:, type:, description: header lines
        assert!(lines[0].starts_with("name: "), "first line must be 'name: '");
        assert!(lines[1].starts_with("type: "), "second line must be 'type: '");
        assert!(lines[2].starts_with("description: "), "third line must be 'description: '");

        // Must have --- separator between header and body
        let sep_pos = lines.iter().position(|l| *l == "---");
        assert!(sep_pos.is_some(), "must have --- separator");
    }

    #[test]
    fn gc_dry_run_output_format() {
        let tmp = temp_copy("valid");
        let out = memctl()
            .args(["--path", tmp.path().to_str().unwrap(), "gc", "--dry-run"])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            assert!(
                line.starts_with("would archive: "),
                "gc --dry-run line must start with 'would archive: ': {}",
                line
            );
        }
    }

    #[test]
    fn gc_output_format() {
        let tmp = temp_copy("valid");
        let out = memctl()
            .args(["--path", tmp.path().to_str().unwrap(), "gc"])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            assert!(
                line.starts_with("archived: "),
                "gc line must start with 'archived: ': {}",
                line
            );
        }
    }
}
