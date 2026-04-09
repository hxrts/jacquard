//! Enforces the intended developer-facing DX surface for presets and clients.
//!
//! Rules:
//! - `ReferenceLink` / `ReferenceNode` must not reappear in the workspace as
//!   developer-facing preset names.
//! - `jacquard-reference-client` must not reintroduce public `build_*client*`
//!   factory functions; client construction goes through `ClientBuilder`.
//! - Human-facing preset/client modules should not drift back toward long
//!   positional public signatures.
//!
//! Registered as: `cargo xtask check dx-surface`

use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::util::{normalize_rel_path, workspace_root, Violation};

const PRESET_NAME_NEEDLES: &[&str] = &["ReferenceLink", "ReferenceNode"];
const MAX_POSITIONAL_PARAMS: usize = 4;

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    violations.extend(scan_forbidden_preset_names(&root)?);
    violations.extend(scan_factory_surface(&root)?);
    violations.extend(scan_public_signature_lengths(&root)?);

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!("dx-surface: found {} DX regression(s)", violations.len());
        bail!("dx-surface failed");
    }

    println!("dx-surface: preset/client DX surface is valid");
    Ok(())
}

fn scan_forbidden_preset_names(root: &Path) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for rel in [
        "crates/mem-link-profile/src",
        "crates/mem-node-profile/src",
        "crates/reference-client/src",
    ] {
        let dir = root.join(rel);
        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let contents = std::fs::read_to_string(entry.path())
                .with_context(|| format!("reading {}", entry.path().display()))?;
            let rel = normalize_rel_path(root, entry.path());
            for (line_no, line) in contents.lines().enumerate() {
                if PRESET_NAME_NEEDLES
                    .iter()
                    .any(|needle| line.contains(needle))
                {
                    violations.push(Violation::new(
                        rel.clone(),
                        line_no + 1,
                        "developer-facing preset surface must use `LinkPreset` / `NodePreset` naming",
                    ));
                }
            }
        }
    }
    Ok(violations)
}

fn scan_factory_surface(root: &Path) -> Result<Vec<Violation>> {
    let path = root.join("crates/reference-client/src/clients.rs");
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let rel = normalize_rel_path(root, &path);
    let mut violations = Vec::new();

    for (line_no, line) in contents.lines().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("pub fn build_") || !trimmed.contains("client") {
            continue;
        }
        let Some(name) = trimmed
            .strip_prefix("pub fn ")
            .and_then(|rest| rest.split('(').next())
        else {
            continue;
        };
        violations.push(Violation::new(
            rel.clone(),
            line_no + 1,
            format!(
                "public client factory `{name}` is forbidden; construct clients through `ClientBuilder`"
            ),
        ));
    }

    Ok(violations)
}

fn scan_public_signature_lengths(root: &Path) -> Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for rel in [
        "crates/mem-link-profile/src/authoring.rs",
        "crates/mem-node-profile/src/authoring.rs",
        "crates/reference-client/src/clients.rs",
        "crates/reference-client/src/topology.rs",
    ] {
        let path = root.join(rel);
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        violations.extend(scan_file_for_long_public_signatures(
            &normalize_rel_path(root, &path),
            &contents,
        ));
    }
    Ok(violations)
}

fn scan_file_for_long_public_signatures(rel: &str, contents: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut iter = contents.lines().enumerate().peekable();

    while let Some((line_no, line)) = iter.next() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("pub fn ") {
            continue;
        }

        let mut signature = trimmed.to_owned();
        while !signature.contains('{') && !signature.contains(';') {
            let Some((_, next_line)) = iter.peek() else {
                break;
            };
            signature.push(' ');
            signature.push_str(next_line.trim());
            let _ = iter.next();
        }

        let Some(name) = signature
            .strip_prefix("pub fn ")
            .and_then(|rest| rest.split('(').next())
        else {
            continue;
        };
        let Some(params) = signature
            .split_once('(')
            .and_then(|(_, rest)| rest.split_once(')'))
            .map(|(params, _)| params)
        else {
            continue;
        };

        let count = params
            .split(',')
            .map(str::trim)
            .filter(|param| {
                !param.is_empty()
                    && *param != "&self"
                    && *param != "&mut self"
                    && *param != "self"
            })
            .count();
        if count > MAX_POSITIONAL_PARAMS {
            violations.push(Violation::new(
                rel.to_owned(),
                line_no + 1,
                format!(
                    "public human-facing API `{name}` has {count} positional parameters; prefer typed options/builders"
                ),
            ));
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::scan_file_for_long_public_signatures;

    #[test]
    fn flags_long_human_facing_signatures() {
        let violations = scan_file_for_long_public_signatures(
            "crates/example/src/authoring.rs",
            "pub fn route_capable(a: u8, b: u8, c: u8, d: u8, e: u8) {}",
        );

        assert_eq!(violations.len(), 1);
    }
}
