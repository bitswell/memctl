use anyhow::Result;
use std::path::Path;
use strsim::jaro_winkler;

use crate::schema;

const THRESHOLD: f64 = 0.85;

/// Find near-duplicate memory files by comparing descriptions.
pub fn run(dir: &Path) -> Result<bool> {
    let (memories, errors) = schema::parse_all(dir)?;
    for e in &errors {
        eprintln!("warn: {}", e);
    }
    let mut found = false;

    for i in 0..memories.len() {
        for j in (i + 1)..memories.len() {
            let a = &memories[i];
            let b = &memories[j];

            let sim = jaro_winkler(&a.frontmatter.description, &b.frontmatter.description);
            if sim >= THRESHOLD {
                let a_name = a.path.file_name().unwrap_or_default().to_string_lossy();
                let b_name = b.path.file_name().unwrap_or_default().to_string_lossy();
                println!("{:.2}  {} <-> {}", sim, a_name, b_name);
                found = true;
            }
        }
    }

    if !found {
        eprintln!("no duplicates found (threshold: {:.2})", THRESHOLD);
    }

    Ok(found)
}
