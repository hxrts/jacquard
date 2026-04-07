//! Shared helpers: `Violation` reporter, workspace metadata access,
//! path normalization, markdown file enumeration, and `just` recipe
//! lookup.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    pub file:    String,
    pub line:    usize,
    pub message: String,
}

impl Violation {
    pub fn new(
        file: impl Into<String>,
        line: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            message: message.into(),
        }
    }

    pub fn render(&self) -> String {
        format!("{}:{}: {}", self.file, self.line, self.message)
    }
}

pub fn workspace_root() -> Result<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while dir.pop() {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            let contents = std::fs::read_to_string(&manifest)
                .with_context(|| format!("reading {}", manifest.display()))?;
            if contents.contains("[workspace]") {
                return Ok(dir);
            }
        }
    }
    bail!("xtask: could not find workspace root")
}

pub fn workspace_metadata() -> Result<Metadata> {
    let root = workspace_root()?;
    MetadataCommand::new()
        .manifest_path(root.join("Cargo.toml"))
        .exec()
        .context("cargo metadata")
}

pub fn normalize_rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn collect_markdown_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let include_roots = ["docs", "crates", "scripts", ".github"];
    for rel in include_roots {
        let dir = root.join(rel);
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                if normalize_rel_path(root, path).starts_with("docs/book/") {
                    continue;
                }
                files.push(path.to_path_buf());
            }
        }
    }
    for rel in ["CLAUDE.md", "README.md"] {
        let path = root.join(rel);
        if path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

pub fn just_recipes(root: &Path) -> Result<BTreeSet<String>> {
    let output = Command::new("just")
        .arg("--summary")
        .current_dir(root)
        .output()
        .context("running just --summary")?;
    if !output.status.success() {
        bail!("xtask: just --summary failed");
    }
    let stdout = String::from_utf8(output.stdout).context("just summary utf8")?;
    Ok(stdout
        .split_whitespace()
        .map(std::string::ToString::to_string)
        .collect())
}
