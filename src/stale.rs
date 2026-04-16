use anyhow::Result;
use chrono::Local;
use std::path::Path;

use crate::schema;

/// List memory files with valid_until in the past.
pub fn run(dir: &Path) -> Result<bool> {
    let (memories, errors) = schema::parse_all(dir)?;
    for e in &errors {
        eprintln!("warn: {}", e);
    }
    let today = Local::now().date_naive();
    let mut found = false;

    for mf in &memories {
        if let Some(valid_until) = mf.frontmatter.valid_until {
            if valid_until <= today {
                let name = mf.path.file_name().unwrap_or_default().to_string_lossy();
                println!("{} (expired {})", name, valid_until);
                found = true;
            }
        }
    }

    if !found {
        eprintln!("no stale memories");
    }

    Ok(found)
}
