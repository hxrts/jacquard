//! Validates detailed invariant specification format.

use anyhow::Result;
use std::fs;

use crate::util::collect_markdown_files;
use crate::util::workspace_root;

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let md_files = collect_markdown_files(&root)?;

    let mut violations = Vec::new();

    for file_path in md_files {
        let contents = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Find all "## Invariant:" sections
        let mut in_invariant = false;
        let mut invariant_title = String::new();
        let mut has_locus = false;
        let mut has_failure = false;
        let mut has_verification = false;

        for line in contents.lines() {
            if line.starts_with("## Invariant:") {
                // Report previous invariant if incomplete
                if in_invariant
                    && (!has_locus || !has_failure || !has_verification)
                {
                    let missing = [
                        (!has_locus).then_some("Enforcement locus"),
                        (!has_failure).then_some("Failure mode"),
                        (!has_verification).then_some("Verification hooks"),
                    ]
                    .iter()
                    .filter_map(|&x| x)
                    .collect::<Vec<_>>()
                    .join(", ");

                    violations.push(format!(
                        "{}: Invariant '{}' missing: {}",
                        file_path.display(),
                        invariant_title,
                        missing
                    ));
                }

                in_invariant = true;
                invariant_title = line.strip_prefix("## Invariant:").unwrap_or("").trim().to_string();
                has_locus = false;
                has_failure = false;
                has_verification = false;
            } else if in_invariant && line.starts_with("## ") {
                // New section, check previous invariant
                if !has_locus || !has_failure || !has_verification {
                    let missing = [
                        (!has_locus).then_some("Enforcement locus"),
                        (!has_failure).then_some("Failure mode"),
                        (!has_verification).then_some("Verification hooks"),
                    ]
                    .iter()
                    .filter_map(|&x| x)
                    .collect::<Vec<_>>()
                    .join(", ");

                    violations.push(format!(
                        "{}: Invariant '{}' missing: {}",
                        file_path.display(),
                        invariant_title,
                        missing
                    ));
                }
                in_invariant = false;
            } else if in_invariant {
                if line.contains("**Enforcement locus:**") || line.contains("Enforcement locus:") {
                    has_locus = true;
                }
                if line.contains("**Failure mode:**") || line.contains("Failure mode:") {
                    has_failure = true;
                }
                if line.contains("**Verification hooks:**")
                    || line.contains("Verification hooks:")
                    || line.contains("**Test:")
                {
                    has_verification = true;
                }
            }
        }

        // Check final invariant
        if in_invariant && (!has_locus || !has_failure || !has_verification) {
            let missing = [
                (!has_locus).then_some("Enforcement locus"),
                (!has_failure).then_some("Failure mode"),
                (!has_verification).then_some("Verification hooks"),
            ]
            .iter()
            .filter_map(|&x| x)
            .collect::<Vec<_>>()
            .join(", ");

            violations.push(format!(
                "{}: Invariant '{}' missing: {}",
                file_path.display(),
                invariant_title,
                missing
            ));
        }
    }

    if violations.is_empty() {
        println!("invariant-specs: all invariants properly specified");
        return Ok(());
    }

    eprintln!("invariant-specs: found violations:");
    for v in &violations {
        eprintln!("  {}", v);
    }

    Ok(())
}
