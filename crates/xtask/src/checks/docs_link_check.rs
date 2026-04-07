//! Validates markdown links. Rejects broken docs targets, links into
//! the `work/` scratch directory, and absolute filesystem paths.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::util::{collect_markdown_files, normalize_rel_path, workspace_root};

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let docs_root = root.join("docs");
    let mut checked = 0_usize;
    let mut missing = Vec::new();
    let mut work_links = Vec::new();
    let mut abs_links = Vec::new();

    for file in collect_markdown_files(&root)? {
        let rel_file = normalize_rel_path(&root, &file);
        let contents = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let parser = Parser::new_ext(&contents, Options::empty());
        for event in parser {
            let Event::Start(Tag::Link { dest_url, .. }) = event else {
                continue;
            };
            let target = dest_url.to_string();
            if target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
                || target.starts_with('#')
            {
                continue;
            }

            if target.contains("work/") {
                work_links.push(format!("{rel_file} -> {target}"));
            }

            if rel_file.starts_with("docs/")
                && target.starts_with('/')
                && matches!(
                    target.as_str(),
                    s if s.starts_with("/Users/")
                        || s.starts_with("/home/")
                        || s.starts_with("/tmp/")
                        || s.starts_with("/var/")
                        || s.starts_with("/opt/")
                        || s.starts_with("/root/")
                )
            {
                abs_links.push(format!("{rel_file} -> {target}"));
            }

            let path_part = target.split('#').next().unwrap_or_default();
            if path_part.is_empty() {
                continue;
            }
            let resolved = resolve_target(&root, &file, path_part);
            if let Some(resolved) = resolved {
                if resolved.starts_with(&docs_root) {
                    checked += 1;
                    if !resolved.is_file() {
                        missing.push(format!(
                            "{rel_file} -> {}",
                            normalize_rel_path(&root, &resolved)
                        ));
                    }
                }
            }
        }
    }

    if !missing.is_empty() {
        for miss in &missing {
            eprintln!("missing docs link: {miss}");
        }
        eprintln!();
        eprintln!(
            "checked {checked} docs link(s); found {} missing target(s)",
            missing.len()
        );
        bail!("docs-link-check failed");
    }
    println!("checked {checked} docs link(s); all targets exist");

    if !work_links.is_empty() {
        for link in &work_links {
            eprintln!("link to work/ found: {link}");
        }
        eprintln!();
        eprintln!("found {} link(s) to work/ directory", work_links.len());
        bail!("docs-link-check failed");
    }
    println!("no links to work/ directory found");

    if !abs_links.is_empty() {
        for link in &abs_links {
            eprintln!("absolute path in link: {link}");
        }
        eprintln!();
        eprintln!(
            "found {} link(s) with absolute filesystem paths in docs/",
            abs_links.len()
        );
        bail!("docs-link-check failed");
    }
    println!("no absolute filesystem paths in docs links");
    Ok(())
}

fn resolve_target(root: &Path, source_file: &Path, target: &str) -> Option<PathBuf> {
    if target.starts_with('/') {
        return Some(PathBuf::from(target));
    }
    let target_path = Path::new(target);
    if target.starts_with("docs/") {
        return Some(root.join(target_path));
    }
    Some(
        source_file
            .parent()
            .unwrap_or(root)
            .join(target_path)
            .components()
            .collect(),
    )
}
