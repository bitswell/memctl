use anyhow::{Result, bail};
use std::path::Path;

use crate::schema;

/// Show a memory file by name (fuzzy match on filename or frontmatter name).
pub fn run(dir: &Path, query: &str) -> Result<()> {
    let (memories, errors) = schema::parse_all(dir)?;
    for e in &errors {
        eprintln!("warn: {}", e);
    }

    // Exact filename match first
    let query_lower = query.to_lowercase();
    let found = memories.iter().find(|m| {
        let fname = m
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        fname == query_lower
            || fname == format!("{}.md", query_lower)
            || m.frontmatter.name.to_lowercase().contains(&query_lower)
    });

    match found {
        Some(mf) => {
            println!("name: {}", mf.frontmatter.name);
            println!("type: {}", mf.frontmatter.memory_type);
            println!("description: {}", mf.frontmatter.description);
            if let Some(ref s) = mf.frontmatter.superseded_by {
                println!("superseded_by: {}", s);
            }
            if let Some(d) = mf.frontmatter.valid_until {
                println!("valid_until: {}", d);
            }
            if !mf.frontmatter.tags.is_empty() {
                println!("tags: {}", mf.frontmatter.tags.join(", "));
            }
            println!("---");
            println!("{}", mf.body.trim());
            Ok(())
        }
        None => bail!("no memory matching '{}'", query),
    }
}
