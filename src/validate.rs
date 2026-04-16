use anyhow::Result;
use chrono::Local;
use std::path::Path;

use crate::schema;

/// Validate all memory files in the directory. Prints errors to stderr.
/// Returns Ok(true) if all valid, Ok(false) if any invalid.
///
/// Checks:
/// - Schema validity (required fields, correct types)
/// - Temporal consistency:
///   - superseded_by must not reference self
///   - superseded_by target file must exist
///   - valid_until in the past emits a warning (not an error)
pub fn run(dir: &Path) -> Result<bool> {
    let paths = schema::discover(dir)?;
    let mut ok = true;
    let today = Local::now().date_naive();

    // First pass: parse all files
    let mut parsed = Vec::new();
    for path in &paths {
        match schema::parse_file(path) {
            Ok(mf) => parsed.push(mf),
            Err(e) => {
                eprintln!("{}", e);
                ok = false;
            }
        }
    }

    // Second pass: temporal consistency checks on successfully parsed files
    for mf in &parsed {
        let filename = mf
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        // superseded_by must not reference self
        if let Some(ref target) = mf.frontmatter.superseded_by {
            if target == filename.as_ref() {
                eprintln!("{}: superseded_by references itself", filename);
                ok = false;
            } else if !dir.join(target).exists() {
                // superseded_by target must exist
                eprintln!("{}: superseded_by target '{}' does not exist", filename, target);
                ok = false;
            }
        }

        // valid_until in the past is a warning, not an error
        if let Some(valid_until) = mf.frontmatter.valid_until {
            if valid_until <= today {
                eprintln!("warn: {}: valid_until {} is in the past", filename, valid_until);
            }
        }
    }

    if ok {
        eprintln!("validated {} files", paths.len());
    }

    Ok(ok)
}
