use anyhow::Result;
use std::io::Write;
use std::path::Path;

use crate::schema;

/// Atomic write: write to tempfile in same dir, then rename over target.
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(content.as_bytes())?;
    tmp.persist(path)?;
    Ok(())
}

/// Backfill missing optional frontmatter fields (superseded_by, valid_until, tags).
pub fn run(dir: &Path, dry_run: bool) -> Result<u32> {
    let paths = schema::discover(dir)?;
    let mut count = 0u32;

    for path in &paths {
        let mf = match schema::parse_file(path) {
            Ok(mf) => mf,
            Err(e) => {
                eprintln!("skip {}: {}", path.display(), e);
                continue;
            }
        };

        // Re-serialize with all fields present (serde will include defaults)
        let yaml = serde_yaml::to_string(&mf.frontmatter)?;
        let output = format!("---\n{}---\n\n{}\n", yaml, mf.body.trim());
        let original = std::fs::read_to_string(path)?;

        if output != original {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if dry_run {
                println!("would migrate: {}", name);
            } else {
                atomic_write(path, &output)?;
                println!("migrated: {}", name);
            }
            count += 1;
        }
    }

    if count == 0 {
        eprintln!("nothing to migrate");
    }

    Ok(count)
}
