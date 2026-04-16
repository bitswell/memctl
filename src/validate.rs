use anyhow::Result;
use std::path::Path;

use crate::schema;

/// Validate all memory files in the directory. Prints errors to stderr.
/// Returns Ok(true) if all valid, Ok(false) if any invalid.
pub fn run(dir: &Path) -> Result<bool> {
    let paths = schema::discover(dir)?;
    let mut ok = true;

    for path in &paths {
        if let Err(e) = schema::parse_file(path) {
            eprintln!("{}", e);
            ok = false;
        }
    }

    if ok {
        eprintln!("validated {} files", paths.len());
    }

    Ok(ok)
}
