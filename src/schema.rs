use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use gray_matter::Matter;
use gray_matter::engine::YAML;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    User,
    Feedback,
    Project,
    Reference,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Feedback => write!(f, "feedback"),
            Self::Project => write!(f, "project"),
            Self::Reference => write!(f, "reference"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Preserve unknown fields through round-trips (e.g. originSessionId)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone)]
pub struct MemoryFile {
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    pub body: String,
}

/// Parse a single memory file from disk.
pub fn parse_file(path: &Path) -> Result<MemoryFile> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let matter = Matter::<YAML>::new();
    let result = matter.parse(&raw);

    let yaml_str = result.matter.trim();
    if yaml_str.is_empty() {
        bail!("{}: missing frontmatter", path.display());
    }

    let fm: Frontmatter = serde_yaml::from_str(yaml_str)
        .with_context(|| format!("{}: invalid frontmatter", path.display()))?;

    Ok(MemoryFile {
        path: path.to_path_buf(),
        frontmatter: fm,
        body: result.content,
    })
}

/// Discover all .md files in a directory (non-recursive), excluding MEMORY.md.
pub fn discover(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md")
            && path.file_name().is_some_and(|n| n != "MEMORY.md")
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Parse all memory files in a directory.
/// Returns valid files and error messages for malformed ones.
/// Callers decide whether to bail or continue with the good files.
pub fn parse_all(dir: &Path) -> Result<(Vec<MemoryFile>, Vec<String>)> {
    let paths = discover(dir)?;
    let mut memories = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match parse_file(&path) {
            Ok(mf) => memories.push(mf),
            Err(e) => errors.push(format!("{}", e)),
        }
    }

    Ok((memories, errors))
}
