//! Validates markdown links. Rejects broken docs targets, links into
//! the `work/` scratch directory, and absolute filesystem paths.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::util::{collect_markdown_files, normalize_rel_path, workspace_root};

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let docs_root = root.join("docs");
    let report = scan_links(&root, &docs_root)?;

    report_missing_links(report.checked, &report.missing)?;
    report_work_links(&report.work_links)?;
    report_absolute_links(&report.abs_links)?;
    Ok(())
}

#[derive(Default)]
struct LinkScanReport {
    checked: usize,
    missing: Vec<String>,
    work_links: Vec<String>,
    abs_links: Vec<String>,
}

fn scan_links(root: &Path, docs_root: &Path) -> Result<LinkScanReport> {
    let mut report = LinkScanReport::default();

    for file in collect_markdown_files(root)? {
        scan_file_links(root, docs_root, &file, &mut report)?;
    }

    Ok(report)
}

fn scan_file_links(
    root: &Path,
    docs_root: &Path,
    file: &Path,
    report: &mut LinkScanReport,
) -> Result<()> {
    let rel_file = normalize_rel_path(root, file);
    let contents = std::fs::read_to_string(file)
        .with_context(|| format!("reading {}", file.display()))?;
    let parser = Parser::new_ext(&contents, Options::empty());

    for event in parser {
        let Event::Start(Tag::Link { dest_url, .. }) = event else {
            continue;
        };
        let target = dest_url.to_string();
        if should_skip_target(&target) {
            continue;
        }

        record_link_policy_violations(&rel_file, &target, report);
        record_missing_docs_target(root, docs_root, file, &rel_file, &target, report);
    }

    Ok(())
}

fn should_skip_target(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with('#')
}

fn record_link_policy_violations(
    rel_file: &str,
    target: &str,
    report: &mut LinkScanReport,
) {
    if target.contains("work/") {
        report.work_links.push(format!("{rel_file} -> {target}"));
    }

    if rel_file.starts_with("docs/")
        && target.starts_with('/')
        && matches!(
            target,
            s if s.starts_with("/Users/")
                || s.starts_with("/home/")
                || s.starts_with("/tmp/")
                || s.starts_with("/var/")
                || s.starts_with("/opt/")
                || s.starts_with("/root/")
        )
    {
        report.abs_links.push(format!("{rel_file} -> {target}"));
    }
}

fn record_missing_docs_target(
    root: &Path,
    docs_root: &Path,
    file: &Path,
    rel_file: &str,
    target: &str,
    report: &mut LinkScanReport,
) {
    let path_part = target.split('#').next().unwrap_or_default();
    if path_part.is_empty() {
        return;
    }
    let Some(resolved) = resolve_target(root, file, path_part) else {
        return;
    };
    if !resolved.starts_with(docs_root) {
        return;
    }

    report.checked += 1;
    if !resolved.is_file() {
        report.missing.push(format!(
            "{rel_file} -> {}",
            normalize_rel_path(root, &resolved)
        ));
    }
}

fn report_missing_links(checked: usize, missing: &[String]) -> Result<()> {
    if !missing.is_empty() {
        for miss in missing {
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
    Ok(())
}

fn report_work_links(work_links: &[String]) -> Result<()> {
    if !work_links.is_empty() {
        for link in work_links {
            eprintln!("link to work/ found: {link}");
        }
        eprintln!();
        eprintln!("found {} link(s) to work/ directory", work_links.len());
        bail!("docs-link-check failed");
    }
    println!("no links to work/ directory found");
    Ok(())
}

fn report_absolute_links(abs_links: &[String]) -> Result<()> {
    if !abs_links.is_empty() {
        for link in abs_links {
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
