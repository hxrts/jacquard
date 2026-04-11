//! Keeps `verification/Field/CODE_MAP.md` synchronized with the Lean module set.
//!
//! Rules:
//! - every non-doc `verification/Field/**/*.lean` module must appear in the code map
//! - every module listed in the code map must still exist on disk
//! - module entries must not be duplicated
//! - every listed module entry must have at least one descriptive continuation bullet
//!
//! Registered as: `cargo xtask check field-code-map`

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{bail, Context, Result};
use regex::Regex;

use crate::util::{normalize_rel_path, workspace_root, Violation};

const CODE_MAP_PATH: &str = "verification/Field/CODE_MAP.md";
const FIELD_ROOT: &str = "verification/Field";

pub fn run() -> Result<()> {
    let violations = collect_violations()?;
    report_violations(&violations)?;

    println!("field-code-map: verification/Field/CODE_MAP.md matches the current Lean module set");
    Ok(())
}

fn collect_violations() -> Result<Vec<Violation>> {
    let root = workspace_root()?;
    let code_map_path = root.join(CODE_MAP_PATH);
    let code_map = std::fs::read_to_string(&code_map_path)
        .with_context(|| format!("reading {}", code_map_path.display()))?;

    let listed_entries = listed_modules(&code_map)?;
    let listed_modules = listed_entries.keys().cloned().collect::<BTreeSet<_>>();
    let actual_modules = actual_modules(&root)?;
    let mut violations = Vec::new();

    for (module, entry) in &listed_entries {
        violations.extend(module_entry_violations(module, entry));
    }

    violations.extend(missing_module_violations(&actual_modules, &listed_modules));
    violations.extend(stale_module_violations(
        &listed_modules,
        &actual_modules,
        &listed_entries,
    ));
    Ok(violations)
}

fn report_violations(violations: &[Violation]) -> Result<()> {
    if violations.is_empty() {
        return Ok(());
    }
    for violation in violations {
        eprintln!("{}", violation.render());
    }
    eprintln!();
    eprintln!(
        "field-code-map: found {} CODE_MAP coverage/drift issue(s)",
        violations.len()
    );
    bail!("field-code-map failed");
}

fn actual_modules(root: &Path) -> Result<BTreeSet<String>> {
    let mut modules = BTreeSet::new();
    for entry in walkdir::WalkDir::new(root.join(FIELD_ROOT))
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("lean") {
            continue;
        }
        let rel = normalize_rel_path(root, path);
        if rel.contains("/Docs/") {
            continue;
        }
        let Some(module) = rel.strip_prefix("verification/") else {
            continue;
        };
        modules.insert(module.to_string());
    }
    Ok(modules)
}

fn listed_modules(contents: &str) -> Result<BTreeMap<String, ListedModuleEntry>> {
    let module_re = Regex::new(r"^- `(?P<module>Field/[A-Za-z0-9_/]+\.lean)`\s*$")?;
    let mut entries = BTreeMap::new();
    let lines = contents.lines().collect::<Vec<_>>();
    let mut index = 0usize;

    while let Some(line) = lines.get(index) {
        if let Some(captures) = module_re.captures(line) {
            let module = captures["module"].to_string();
            let entry = entries.entry(module).or_insert_with(|| ListedModuleEntry {
                line: index + 1,
                occurrences: 0,
                description_lines: 0,
            });
            entry.occurrences += 1;

            let mut description_lines = 0usize;
            let mut cursor = index + 1;
            while let Some(next_line) = lines.get(cursor) {
                if module_re.is_match(next_line)
                    || next_line.starts_with("## ")
                    || next_line.starts_with("### ")
                {
                    break;
                }
                if next_line.starts_with("  - ") {
                    description_lines += 1;
                }
                cursor += 1;
            }
            entry.description_lines = entry.description_lines.max(description_lines);
        }
        index += 1;
    }

    Ok(entries)
}

struct ListedModuleEntry {
    line: usize,
    occurrences: usize,
    description_lines: usize,
}

fn module_entry_violations(module: &str, entry: &ListedModuleEntry) -> Vec<Violation> {
    let mut violations = Vec::new();
    if entry.occurrences > 1 {
        violations.push(Violation::new(
            CODE_MAP_PATH,
            entry.line,
            format!("module `{module}` is listed multiple times in CODE_MAP"),
        ));
    }
    if entry.description_lines == 0 {
        violations.push(Violation::new(
            CODE_MAP_PATH,
            entry.line,
            format!("module `{module}` is missing a descriptive continuation bullet"),
        ));
    }
    violations
}

fn missing_module_violations(
    actual_modules: &BTreeSet<String>,
    listed_modules: &BTreeSet<String>,
) -> Vec<Violation> {
    actual_modules
        .difference(listed_modules)
        .map(|module| {
            Violation::new(
                CODE_MAP_PATH,
                1,
                format!(
                    "module `{module}` exists under `verification/Field` but is missing from CODE_MAP"
                ),
            )
        })
        .collect()
}

fn stale_module_violations(
    listed_modules: &BTreeSet<String>,
    actual_modules: &BTreeSet<String>,
    listed_entries: &BTreeMap<String, ListedModuleEntry>,
) -> Vec<Violation> {
    listed_modules
        .difference(actual_modules)
        .map(|module| {
            let line = listed_entries
                .get(module)
                .map(|entry| entry.line)
                .unwrap_or(1);
            Violation::new(
                CODE_MAP_PATH,
                line,
                format!("module `{module}` is listed in CODE_MAP but no longer exists"),
            )
        })
        .collect()
}
