use anyhow::Result;
use chrono::Local;
use std::path::Path;

use crate::schema::{self, MemoryFile};

/// Generate MEMORY.md content deterministically.
pub fn generate(dir: &Path) -> Result<String> {
    let (memories, errors) = schema::parse_all(dir)?;
    for e in &errors {
        eprintln!("warn: {}", e);
    }

    // Filter: active memories only (no superseded_by, no expired valid_until)
    let today = Local::now().date_naive();
    let active: Vec<&MemoryFile> = memories
        .iter()
        .filter(|m| {
            m.frontmatter.superseded_by.is_none()
                && m.frontmatter
                    .valid_until
                    .map_or(true, |d| d > today)
        })
        .collect();

    // Sort by type then name
    let mut sorted = active;
    sorted.sort_by(|a, b| {
        let type_a = a.frontmatter.memory_type.to_string();
        let type_b = b.frontmatter.memory_type.to_string();
        type_a.cmp(&type_b).then_with(|| a.frontmatter.name.cmp(&b.frontmatter.name))
    });

    let mut lines = Vec::new();
    for mf in &sorted {
        let filename = mf
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        let desc = if mf.frontmatter.description.len() > 100 {
            // Find a safe char boundary at or before byte 97 to avoid panic on multi-byte UTF-8
            let safe = mf.frontmatter.description
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= 97)
                .last()
                .unwrap_or(0);
            format!("{}...", &mf.frontmatter.description[..safe])
        } else {
            mf.frontmatter.description.clone()
        };

        let tags_suffix = if mf.frontmatter.tags.is_empty() {
            String::new()
        } else {
            let tags: Vec<String> = mf.frontmatter.tags.iter().map(|t| format!("#{t}")).collect();
            format!(" {}", tags.join(" "))
        };

        lines.push(format!(
            "- [{}]({}) \u{2014} {}{}",
            mf.frontmatter.name, filename, desc, tags_suffix
        ));
    }

    Ok(lines.join("\n"))
}

/// Write MEMORY.md to disk.
pub fn run(dir: &Path) -> Result<()> {
    let content = generate(dir)?;
    let out = dir.join("MEMORY.md");
    std::fs::write(&out, format!("{}\n", content))?;
    eprintln!("wrote {}", out.display());
    Ok(())
}
