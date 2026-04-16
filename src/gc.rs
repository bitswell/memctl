use anyhow::Result;
use chrono::Local;
use std::fs;
use std::path::Path;

use crate::schema;

/// Archive expired and superseded memory files by moving them to an archive/ subdir.
pub fn run(dir: &Path, dry_run: bool) -> Result<u32> {
    let (memories, errors) = schema::parse_all(dir)?;
    for e in &errors {
        eprintln!("warn: {}", e);
    }
    let today = Local::now().date_naive();
    let archive = dir.join("archive");

    let mut count = 0u32;

    for mf in &memories {
        let expired = mf
            .frontmatter
            .valid_until
            .map_or(false, |d| d <= today);
        let superseded = mf.frontmatter.superseded_by.is_some();

        if expired || superseded {
            let name = mf.path.file_name().unwrap_or_default().to_string_lossy();
            if dry_run {
                println!("would archive: {}", name);
            } else {
                if !archive.exists() {
                    fs::create_dir_all(&archive)?;
                }
                let dest = archive.join(name.as_ref());
                fs::rename(&mf.path, &dest)?;
                println!("archived: {}", name);
            }
            count += 1;
        }
    }

    if count == 0 {
        eprintln!("nothing to archive");
    }

    Ok(count)
}
