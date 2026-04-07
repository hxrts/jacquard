//! High-leverage routing-invariant checks. Each rule is a small AST
//! or text visitor that either passes cleanly or reports violations.
//! No waivers: if a rule fires on real code, the code gets fixed.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use regex::Regex;

use crate::util::{normalize_rel_path, workspace_root, Violation};

type RuleFn = fn(&Path) -> Result<Vec<Violation>>;

struct Rule {
    description: &'static str,
    collect: RuleFn,
}

const RULES: &[Rule] = &[
    Rule {
        description: "explicit-topology planner signatures",
        collect: explicit_topology,
    },
    Rule {
        description: "world-extension error purity",
        collect: world_error_purity,
    },
    Rule {
        description: "shared/private boundary",
        collect: shared_private_boundary,
    },
    Rule {
        description: "planner cache is optimization only",
        collect: planner_cache_dependence,
    },
    Rule {
        description: "fail-closed mutation ordering",
        collect: fail_closed_ordering,
    },
    Rule {
        description: "Tick/RouteEpoch separation",
        collect: tick_epoch_conflation,
    },
    Rule {
        description: "committee failure is not silently erased",
        collect: committee_swallow,
    },
    Rule {
        description: "namespaced storage keys",
        collect: storage_key_scope,
    },
    Rule {
        description: "no synthetic authoritative-state fallback",
        collect: synthetic_fallback,
    },
];

pub fn run(args: &[String]) -> Result<()> {
    let mut validate = false;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("Usage: cargo xtask check routing-invariants [--validate|--strict]");
                return Ok(());
            }
            "--validate" => validate = true,
            "--strict" => {}
            other => bail!("routing-invariants: unknown argument: {other}"),
        }
    }

    let root = if validate {
        workspace_root()?.join("crates/xtask/fixtures/routing_invariants")
    } else {
        workspace_root()?
    };
    let mut failures = 0usize;
    let mut matched_rules = 0usize;

    for rule in RULES {
        let violations = (rule.collect)(&root)
            .with_context(|| format!("routing-invariants: collecting {}", rule.description))?;
        if violations.is_empty() {
            println!("routing-invariants: {}: OK", rule.description);
        } else {
            matched_rules += 1;
            eprintln!("routing-invariants: {}: violation(s)", rule.description);
            for violation in &violations {
                eprintln!("  {}", violation.render());
            }
            failures += 1;
        }
    }

    if validate {
        if matched_rules != RULES.len() {
            bail!(
                "routing-invariants: validation expected {} rule matches, found {}",
                RULES.len(),
                matched_rules
            );
        }
        println!("routing-invariants: validation fixtures exercised every rule");
        return Ok(());
    }

    if failures > 0 {
        bail!("routing-invariants: found {failures} rule failure(s)");
    }

    println!("routing-invariants: all checks passed");
    Ok(())
}

fn explicit_topology(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();

    for path in rust_files(root.join("crates"))? {
        let rel_path = normalize_rel_path(root, &path);
        if rel_path.starts_with("crates/xtask/")
            || rel_path.starts_with("crates/xtask/fixtures/")
            || rel_path.contains("/tests/")
            || rel_path.contains("/benches/")
            || rel_path.contains("/examples/")
        {
            continue;
        }
        let source =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let lines: Vec<&str> = source.lines().collect();
        let mut idx = 0usize;
        while idx < lines.len() {
            let line = lines[idx];
            let mut fn_name = None;
            if line.contains("fn check_candidate(") {
                fn_name = Some("check_candidate");
            } else if line.contains("fn admit_route(") {
                fn_name = Some("admit_route");
            }
            if let Some(fn_name) = fn_name {
                let start_line = idx + 1;
                let mut signature = line.to_string();
                idx += 1;
                while idx < lines.len() && !signature.contains(") ->") {
                    signature.push(' ');
                    signature.push_str(lines[idx]);
                    idx += 1;
                }
                if !signature.contains("topology: &Observation<Configuration>") {
                    out.push(Violation::new(
                        &rel_path,
                        start_line,
                        format!("{fn_name} is missing explicit topology parameter"),
                    ));
                }
                continue;
            }
            idx += 1;
        }
    }

    Ok(out)
}

fn world_error_purity(root: &Path) -> Result<Vec<Violation>> {
    let path = root.join("crates/traits/src/world.rs");
    let rel = normalize_rel_path(root, &path);
    let contents =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(contents
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("RouteError"))
        .map(|(idx, _)| {
            Violation::new(
                rel.clone(),
                idx + 1,
                "world-extension boundary mentions RouteError instead of WorldError",
            )
        })
        .collect())
}

fn shared_private_boundary(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    let re = Regex::new(r"pub (struct|enum|type)\s+(Mesh|Onion|Field)[A-Z]\w*")?;
    for dir in ["crates/core/src", "crates/traits/src"] {
        for path in rust_files(root.join(dir))? {
            let rel = normalize_rel_path(root, &path);
            if rel.starts_with("crates/xtask/fixtures/") {
                continue;
            }
            let contents =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            for (idx, line) in contents.lines().enumerate() {
                if re.is_match(line) {
                    out.push(Violation::new(
                        rel.clone(),
                        idx + 1,
                        "shared crate defines engine-specific runtime/schema vocabulary",
                    ));
                }
            }
        }
    }
    Ok(out)
}

fn planner_cache_dependence(root: &Path) -> Result<Vec<Violation>> {
    grep_rule(
        root,
        &["crates/mesh/src"],
        r"find_cached_candidate_by_route_id\(",
        "materialization depends on cache lookup helper",
    )
}

fn fail_closed_ordering(root: &Path) -> Result<Vec<Violation>> {
    let runtime_file = root.join("crates/mesh/src/engine/runtime.rs");
    let rel = normalize_rel_path(root, &runtime_file);
    let contents = fs::read_to_string(&runtime_file)
        .with_context(|| format!("reading {}", runtime_file.display()))?;
    let lines: Vec<&str> = contents.lines().collect();
    let mut out = Vec::new();

    if let (Some(insert_line), Some(record_line)) = (
        first_line_containing(&lines, &["self.active_routes", ".insert("]),
        first_line_containing(&lines, &["self.record_event(RouteEvent::RouteMaterialized"]),
    ) {
        if insert_line < record_line {
            out.push(Violation::new(
                rel.clone(),
                insert_line,
                "active route table is mutated before RouteMaterialized is recorded",
            ));
        }
    }

    if let (Some(apply_line), Some(checkpoint_line)) = (
        first_line_containing(&lines, &["Self::apply_maintenance_trigger("]),
        first_line_containing(&lines, &["self.store_checkpoint(&active_route_snapshot)"]),
    ) {
        if apply_line < checkpoint_line {
            out.push(Violation::new(
                rel,
                apply_line,
                "maintenance trigger mutates runtime state before checkpoint persistence",
            ));
        }
    }

    Ok(out)
}

fn tick_epoch_conflation(root: &Path) -> Result<Vec<Violation>> {
    grep_rule(
        root,
        &["crates"],
        r"RouteEpoch\([^)]*tick[^)]*\.0\)|Tick\([^)]*(epoch|current_epoch)[^)]*\.0\)",
        "Tick and RouteEpoch are being conflated by wrapper re-construction",
    )
}

fn committee_swallow(root: &Path) -> Result<Vec<Violation>> {
    grep_rule(
        root,
        &["crates/mesh/src"],
        r"\.ok\(\)\.flatten\(\)",
        "committee selector error is being silently erased",
    )
}

fn storage_key_scope(root: &Path) -> Result<Vec<Violation>> {
    grep_rule(
        root,
        &["crates/mesh/src"],
        r#"b"mesh/(topology-epoch|route/)"#,
        "storage key is not scoped by local engine identity",
    )
}

fn synthetic_fallback(root: &Path) -> Result<Vec<Violation>> {
    grep_rule(
        root,
        &["crates/mesh/src"],
        r"fallback_health_configuration|map_or_else\(\s*\|\|\s*self\.fallback_health_configuration",
        "synthetic authoritative-state fallback detected",
    )
}

fn rust_files(dir: PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.into_path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn is_validation_root(root: &Path) -> bool {
    root.to_string_lossy()
        .replace('\\', "/")
        .ends_with("crates/xtask/fixtures/routing_invariants")
}

fn grep_rule(root: &Path, dirs: &[&str], pattern: &str, message: &str) -> Result<Vec<Violation>> {
    let re = Regex::new(pattern)?;
    let mut out = Vec::new();
    for dir in dirs {
        for path in rust_files(root.join(dir))? {
            let path_str = path.to_string_lossy().replace('\\', "/");
            if !is_validation_root(root) && path_str.contains("/crates/xtask/fixtures/") {
                continue;
            }
            let rel = normalize_rel_path(root, &path);
            let contents =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            for (idx, line) in contents.lines().enumerate() {
                if re.is_match(line) {
                    out.push(Violation::new(rel.clone(), idx + 1, message));
                }
            }
        }
    }
    Ok(out)
}

fn first_line_containing(lines: &[&str], needles: &[&str]) -> Option<usize> {
    lines
        .iter()
        .position(|line| needles.iter().all(|needle| line.contains(needle)))
        .map(|idx| idx + 1)
}
