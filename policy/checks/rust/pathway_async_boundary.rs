//! Enforces pathway's synchronous/asynchronous envelope.
//!
//! The pathway engine and runtime layer must stay synchronous and driver-free.
//! `executor::block_on` and `async fn` are permitted only inside pathway's
//! choreography modules, where Telltale-generated protocol sessions are driven
//! to completion within a single synchronous round. Pathway must not own
//! transport drivers or spawn background tasks.
//!
//! Scans: all `.rs` files under `crates/pathway/src/`. Files inside
//! `choreography/` sub-paths are allowed to use async vocabulary; all other
//! pathway files that contain `async fn` or `block_on` are reported as
//! violations.
//!
//! Registered as: `cargo xtask check pathway-async-boundary`

use anyhow::{bail, Context, Result};

use crate::util::{
    collect_rust_files, layer_for_rel_path, normalize_rel_path, workspace_root, Violation,
};

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    for path in collect_rust_files(&root)? {
        let rel = normalize_rel_path(&root, &path);
        if !rel.starts_with("crates/pathway/src/") {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        violations.extend(scan_file(&rel, &contents));
    }

    if !violations.is_empty() {
        for violation in &violations {
            eprintln!("{}", violation.render());
        }
        eprintln!();
        eprintln!(
            "pathway-async-boundary: found {} boundary violation(s)",
            violations.len()
        );
        bail!("pathway-async-boundary failed");
    }

    println!("pathway-async-boundary: pathway async envelope is valid");
    Ok(())
}

fn scan_file(rel: &str, contents: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let in_choreography = rel.starts_with("crates/pathway/src/choreography/");
    let owns_guest_runtime = rel == "crates/pathway/src/choreography/runtime.rs";
    let mut pending_test_module = false;
    let mut test_module_depth: Option<usize> = None;
    let mut brace_depth = 0_usize;

    for (index, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == "#[cfg(test)]" {
            pending_test_module = true;
        } else if pending_test_module && trimmed.starts_with("mod tests") {
            test_module_depth = Some(brace_depth + line.matches('{').count());
            pending_test_module = false;
        } else if !trimmed.is_empty() {
            pending_test_module = false;
        }

        let in_test_module = test_module_depth.is_some_and(|depth| brace_depth >= depth);
        push_driver_boundary_violation(&mut violations, rel, index, line);
        push_async_envelope_violation(&mut violations, rel, index, line, in_choreography);
        push_guest_runtime_violation(
            &mut violations,
            rel,
            index,
            line,
            owns_guest_runtime,
            in_test_module,
        );

        brace_depth = brace_depth
            .saturating_add(line.matches('{').count())
            .saturating_sub(line.matches('}').count());
        if test_module_depth.is_some_and(|depth| brace_depth < depth) {
            test_module_depth = None;
        }
    }

    violations
}

fn push_driver_boundary_violation(
    violations: &mut Vec<Violation>,
    rel: &str,
    index: usize,
    line: &str,
) {
    if line.contains("TransportDriver") || line.contains("drain_transport_ingress(") {
        violations.push(Violation::with_layer(
            rel.to_owned(),
            index + 1,
            "pathway must not own transport drivers or drain transport ingress directly",
            layer_for_rel_path(rel),
        ));
    }
}

fn push_async_envelope_violation(
    violations: &mut Vec<Violation>,
    rel: &str,
    index: usize,
    line: &str,
    in_choreography: bool,
) {
    if !in_choreography && (line.contains("executor::block_on(") || line.contains("async fn ")) {
        violations.push(Violation::with_layer(
            rel.to_owned(),
            index + 1,
            "pathway async envelope must stay inside choreography modules",
            layer_for_rel_path(rel),
        ));
    }
}

fn push_guest_runtime_violation(
    violations: &mut Vec<Violation>,
    rel: &str,
    index: usize,
    line: &str,
    owns_guest_runtime: bool,
    in_test_module: bool,
) {
    if !owns_guest_runtime
        && (line.contains("PathwayGuestRuntime::new(")
            || line.contains("PathwayGuestRuntime::with_spec_resolver("))
        && !in_test_module
    {
        violations.push(Violation::with_layer(
            rel.to_owned(),
            index + 1,
            "pathway guest-runtime construction must stay local to choreography/runtime.rs",
            layer_for_rel_path(rel),
        ));
    }
}
