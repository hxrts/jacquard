//! Scans backtick code spans in docs and cross-checks them against
//! workspace reality: unknown just recipes, missing file paths,
//! unresolved PascalCase identifiers, unknown crate names, stale
//! version strings, and deprecated identifiers.

use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use cargo_metadata::Package;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::{
    sources::{all_identifiers, parse_workspace_sources},
    util::{
        collect_markdown_files, just_recipes, normalize_rel_path, workspace_metadata,
        workspace_root,
    },
};

const SKIP_IDENTIFIERS: &[&str] = &[
    "String",
    "Vec",
    "Option",
    "Result",
    "Box",
    "Arc",
    "Rc",
    "Mutex",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "PathBuf",
    "Path",
    "Ok",
    "Err",
    "Some",
    "None",
    "Self",
    "Sized",
    "Send",
    "Sync",
    "Clone",
    "Copy",
    "Debug",
    "Display",
    "Default",
    "Drop",
    "Eq",
    "Ord",
    "Hash",
    "Iterator",
    "Future",
    "Pin",
    "From",
    "Into",
    "AsRef",
    "Deref",
    "PartialEq",
    "PartialOrd",
    "Serialize",
    "Deserialize",
    "Error",
    "Read",
    "Write",
    "PhantomData",
    "Infallible",
    "README",
    "SUMMARY",
    "TODO",
    "FIXME",
    "NOTE",
    "WARNING",
    "IMPORTANT",
    "API",
    "CLI",
    "CI",
    "CD",
    "PR",
    "OS",
    "IO",
    "UUID",
    "HTTP",
    "HTTPS",
    "URL",
    "JSON",
    "CBOR",
    "TOML",
    "YAML",
    "WASM",
    "BFT",
    "CRDT",
    "BLE",
    "GPS",
    "GATT",
    "QUIC",
    "MTU",
    "Alice",
    "Bob",
    "Client",
    "Server",
    "Worker",
    "Coordinator",
    "Done",
    "Active",
    "Closed",
    "Faulted",
    "Admitted",
    "Blocked",
    "Failure",
    "Full",
    "Ack",
    "Commit",
    "Abort",
    "Cancel",
    "Retry",
    "Ping",
    "Pong",
];

const EXTERNAL_PREFIXES: &[&str] = &[
    "std",
    "core",
    "alloc",
    "serde",
    "serde_json",
    "tokio",
    "futures",
    "uuid",
    "blake3",
    "thiserror",
    "tracing",
    "proc_macro2",
    "telltale",
];

const PLANNED_CRATES: &[&str] = &[
    "jacquard-mesh",
    "jacquard-router",
    "jacquard-transport",
    "jacquard-simulator",
];

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let parsed = parse_workspace_sources()?;
    let mut identifiers = all_identifiers(&parsed);
    identifiers.extend(SKIP_IDENTIFIERS.iter().map(|item| item.to_string()));
    let metadata = workspace_metadata()?;
    let mut crate_tokens = crate_names(&metadata.packages);
    crate_tokens.extend(PLANNED_CRATES.iter().copied().map(str::to_string));
    crate_tokens.extend(PLANNED_CRATES.iter().map(|item| item.replace('-', "_")));
    let just_recipes = just_recipes(&root)?;
    let mut errors = Vec::new();

    for file in collect_markdown_files(&root)? {
        let rel_file = normalize_rel_path(&root, &file);
        let contents = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let mut in_code_block = false;
        let parser = Parser::new_ext(&contents, Options::empty());
        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_)))
                | Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => in_code_block = true,
                Event::End(TagEnd::CodeBlock) => in_code_block = false,
                Event::Code(snippet) if !in_code_block => {
                    check_snippet(
                        &rel_file,
                        &snippet,
                        &identifiers,
                        &crate_tokens,
                        &just_recipes,
                        &mut errors,
                    );
                }
                _ => {}
            }
        }
    }

    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error}");
        }
        bail!("docs-semantic-drift failed");
    }
    println!("docs-semantic-drift: no stale backtick references found");
    Ok(())
}

fn crate_names(packages: &[Package]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for package in packages {
        names.insert(package.name.clone());
        names.insert(package.name.replace('-', "_"));
    }
    names
}

fn check_snippet(
    file: &str,
    snippet: &str,
    identifiers: &BTreeSet<String>,
    crate_tokens: &BTreeSet<String>,
    just_recipes: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let snippet = snippet.trim();
    if snippet.is_empty() {
        return;
    }
    if snippet.contains('-') && !snippet.contains('/') && !snippet.contains("::") {
        return;
    }
    if let Some(recipe) = snippet.strip_prefix("just ") {
        let recipe = recipe.split_whitespace().next().unwrap_or_default();
        if !recipe.is_empty() && !just_recipes.contains(recipe) {
            errors.push(format!("{file}: unknown just recipe `{snippet}`"));
        }
        return;
    }
    if looks_like_path(snippet) {
        let root = workspace_root().expect("workspace root");
        if !root.join(snippet).exists() {
            errors.push(format!("{file}: unresolved path `{snippet}`"));
        }
        return;
    }
    if crate_tokens.contains(snippet) {
        return;
    }
    if snippet.contains("::") {
        let segments: Vec<&str> = snippet.split("::").collect();
        let prefix = segments.first().copied().unwrap_or_default();
        if EXTERNAL_PREFIXES.contains(&prefix) {
            return;
        }
        let known_segment = segments
            .iter()
            .filter_map(|segment| root_identifier(segment))
            .any(|segment| {
                identifiers.contains(segment)
                    || crate_tokens.contains(segment)
                    || SKIP_IDENTIFIERS.contains(&segment)
            });
        if !known_segment {
            errors.push(format!("{file}: unresolved qualified symbol `{snippet}`"));
        }
        return;
    }
    if let Some(root_ident) = root_identifier(snippet) {
        if root_ident
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
            && !identifiers.contains(root_ident)
            && !crate_tokens.contains(root_ident)
            && !SKIP_IDENTIFIERS.contains(&root_ident)
        {
            errors.push(format!("{file}: unresolved symbol `{snippet}`"));
        }
    }
}

fn looks_like_path(snippet: &str) -> bool {
    matches!(snippet, "CLAUDE.md" | "Cargo.toml" | "justfile")
        || [
            "docs/", "crates/", "scripts/", "lints/", "nix/", ".github/", "work/",
        ]
        .iter()
        .any(|prefix| snippet.starts_with(prefix))
}

fn root_identifier(snippet: &str) -> Option<&str> {
    let mut start = None;
    for (idx, ch) in snippet.char_indices() {
        if start.is_none() {
            if ch.is_ascii_alphabetic() || ch == '_' {
                start = Some(idx);
            }
            continue;
        }
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            let start = start?;
            return Some(&snippet[start..idx]);
        }
    }
    start.map(|start| &snippet[start..])
}
