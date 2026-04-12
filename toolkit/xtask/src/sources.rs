//! Workspace source cache and AST visitor helpers.
//!
//! `parse_workspace_sources` walks every `.rs` file under `crates/*/src/`
//! once, parses each with `syn`, and returns a `Vec<ParsedSource>` that
//! AST-based checks can iterate without re-reading or re-parsing files.
//!
//! Also exposes targeted visitor helpers reused across multiple checks:
//! - `public_traits` — iterate public trait items in a parsed file.
//! - `attributes_match` — test whether any attribute on an item has a given
//!   identifier, covering both `#[foo]` and `#[foo(...)]` forms.
//! - `all_identifiers` — collect every declared type and function identifier
//!   from the workspace, used by the semantic-drift check.

use std::collections::BTreeSet;

use anyhow::{Context, Result};
use syn::{Attribute, File, Item, ItemTrait, Visibility};

use crate::util::{normalize_rel_path, workspace_root};

#[derive(Clone)]
pub struct ParsedSource {
    pub rel_path: String,
    pub file: File,
    pub source: String,
}

pub fn parse_workspace_sources() -> Result<Vec<ParsedSource>> {
    let root = workspace_root()?;
    let mut parsed = Vec::new();
    for entry in ignore::Walk::new(root.join("crates")) {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let rel_path = normalize_rel_path(&root, path);
        if rel_path.contains("/tests/")
            || rel_path.contains("/benches/")
            || rel_path.contains("/examples/")
            || rel_path.starts_with("toolkit/fixtures/")
            || rel_path.ends_with("/build.rs")
        {
            continue;
        }
        let source =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let file =
            syn::parse_file(&source).with_context(|| format!("parsing {}", path.display()))?;
        parsed.push(ParsedSource {
            rel_path,
            file,
            source,
        });
    }
    parsed.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(parsed)
}

pub fn public_traits(source: &ParsedSource) -> impl Iterator<Item = &ItemTrait> {
    source.file.items.iter().filter_map(|item| match item {
        Item::Trait(item_trait) if matches!(item_trait.vis, Visibility::Public(_)) => {
            Some(item_trait)
        }
        _ => None,
    })
}

// Raw text split rather than the syn AST so identifiers in string
// literals, comments, and macro expansions are all captured for
// docs-semantic-drift matching.
#[allow(dead_code)]
pub fn all_identifiers(parsed: &[ParsedSource]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for source in parsed {
        for token in source
            .source
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .filter(|token| !token.is_empty())
        {
            out.insert(token.to_string());
        }
    }
    out
}

pub fn attributes_match(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}
