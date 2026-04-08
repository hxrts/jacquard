//! Validates detailed invariant specification format.

use std::{fs, path::Path};

use anyhow::Result;

use crate::util::{collect_markdown_files, workspace_root};

#[derive(Default)]
struct InvariantState {
    in_invariant: bool,
    title: String,
    has_locus: bool,
    has_failure: bool,
    has_verification: bool,
}

impl InvariantState {
    fn missing_fields(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if !self.has_locus {
            out.push("Enforcement locus");
        }
        if !self.has_failure {
            out.push("Failure mode");
        }
        if !self.has_verification {
            out.push("Verification hooks");
        }
        out
    }

    fn is_complete(&self) -> bool {
        self.has_locus && self.has_failure && self.has_verification
    }

    fn record_missing(&self, file_path: &Path, violations: &mut Vec<String>) {
        let missing = self.missing_fields().join(", ");
        violations.push(format!(
            "{}: Invariant '{}' missing: {}",
            file_path.display(),
            self.title,
            missing
        ));
    }

    fn start_new(&mut self, line: &str) {
        self.in_invariant = true;
        self.title = line
            .strip_prefix("## Invariant:")
            .unwrap_or("")
            .trim()
            .to_string();
        self.has_locus = false;
        self.has_failure = false;
        self.has_verification = false;
    }

    fn observe_metadata_line(&mut self, line: &str) {
        if line.contains("**Enforcement locus:**")
            || line.contains("Enforcement locus:")
        {
            self.has_locus = true;
        }
        if line.contains("**Failure mode:**") || line.contains("Failure mode:") {
            self.has_failure = true;
        }
        if line.contains("**Verification hooks:**")
            || line.contains("Verification hooks:")
            || line.contains("**Test:")
        {
            self.has_verification = true;
        }
    }
}

fn scan_file(file_path: &Path, contents: &str, violations: &mut Vec<String>) {
    let mut state = InvariantState::default();

    for line in contents.lines() {
        if line.starts_with("## Invariant:") {
            if state.in_invariant && !state.is_complete() {
                state.record_missing(file_path, violations);
            }
            state.start_new(line);
        } else if state.in_invariant && line.starts_with("## ") {
            if !state.is_complete() {
                state.record_missing(file_path, violations);
            }
            state.in_invariant = false;
        } else if state.in_invariant {
            state.observe_metadata_line(line);
        }
    }

    if state.in_invariant && !state.is_complete() {
        state.record_missing(file_path, violations);
    }
}

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let md_files = collect_markdown_files(&root)?;
    let mut violations = Vec::new();

    for file_path in md_files {
        if let Ok(contents) = fs::read_to_string(&file_path) {
            scan_file(&file_path, &contents, &mut violations);
        }
    }

    if violations.is_empty() {
        println!("invariant-specs: all invariants properly specified");
        return Ok(());
    }

    eprintln!("invariant-specs: found violations:");
    for v in &violations {
        eprintln!("  {v}");
    }
    Ok(())
}
