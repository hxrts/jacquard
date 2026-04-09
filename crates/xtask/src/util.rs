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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerTag {
    Core          = 1,
    Traits        = 2,
    PathwayRouter = 3,
    MockInfra     = 4,
    Tests         = 5,
}

impl LayerTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            | Self::Core => "L1",
            | Self::Traits => "L2",
            | Self::PathwayRouter => "L3",
            | Self::MockInfra => "L4",
            | Self::Tests => "L5",
        }
    }
}

pub fn layer_of(crate_name: &str) -> LayerTag {
    match crate_name {
        | "jacquard-core" => LayerTag::Core,
        | "jacquard-traits" => LayerTag::Traits,
        | "jacquard-pathway" | "jacquard-router" => LayerTag::PathwayRouter,
        | "jacquard-mem-link-profile" | "jacquard-reference-client" => {
            LayerTag::MockInfra
        },
        | _ => LayerTag::Tests,
    }
}

pub fn layer_for_rel_path(rel_path: &str) -> LayerTag {
    if rel_path.starts_with("crates/core/") {
        LayerTag::Core
    } else if rel_path.starts_with("crates/traits/") {
        LayerTag::Traits
    } else if rel_path.starts_with("crates/pathway/")
        || rel_path.starts_with("crates/router/")
    {
        LayerTag::PathwayRouter
    } else if rel_path.starts_with("crates/mem-link-profile/")
        || rel_path.starts_with("crates/reference-client/")
        || rel_path.starts_with("crates/mem-node-profile/")
    {
        LayerTag::MockInfra
    } else {
        LayerTag::Tests
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub message: String,
    pub layer: Option<LayerTag>,
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
            layer: None,
        }
    }

    pub fn with_layer(
        file: impl Into<String>,
        line: usize,
        message: impl Into<String>,
        layer: LayerTag,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            message: message.into(),
            layer: Some(layer),
        }
    }

    pub fn render(&self) -> String {
        if let Some(layer) = self.layer {
            format!(
                "[{}] {}:{}: {}",
                layer.as_str(),
                self.file,
                self.line,
                self.message
            )
        } else {
            format!("{}:{}: {}", self.file, self.line, self.message)
        }
    }
}

// Walk upward rather than invoking `cargo metadata`: xtask lives inside
// the workspace so CARGO_MANIFEST_DIR is always a crate subdirectory.
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

pub fn collect_rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let crates_dir = root.join("crates");
    if !crates_dir.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(&crates_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            let rel = normalize_rel_path(root, path);
            // Skip target/ build artifacts
            if rel.contains("/target/") {
                continue;
            }
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
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
