use anyhow::{Result, bail};
use std::path::Path;

use crate::schema;

/// Mark old_file as superseded by new_file.
/// Updates the old file's frontmatter with superseded_by pointing to new_file.
pub fn run(dir: &Path, old_name: &str, new_name: &str) -> Result<()> {
    let old_path = dir.join(old_name);
    let new_path = dir.join(new_name);

    if !old_path.exists() {
        bail!("{} not found", old_path.display());
    }
    if !new_path.exists() {
        bail!("{} not found", new_path.display());
    }

    // Validate new file parses
    schema::parse_file(&new_path)?;

    // Parse old file, update frontmatter, rewrite
    let mf = schema::parse_file(&old_path)?;
    let mut fm = mf.frontmatter;
    fm.superseded_by = Some(new_name.to_string());

    let yaml = serde_yaml::to_string(&fm)?;
    let output = format!("---\n{}---\n\n{}\n", yaml, mf.body.trim());
    std::fs::write(&old_path, output)?;

    eprintln!("{} superseded by {}", old_name, new_name);
    Ok(())
}
