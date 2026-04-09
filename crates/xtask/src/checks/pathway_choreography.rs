//! Enforces the internal pathway choreography module structure.
//!
//! Pathway choreography modules must follow a prescribed layout: each
//! choreography sub-path must contain the required module files and must not
//! introduce transport drivers or other non-choreography concerns.
//!
//! Supports two modes:
//! - Default (no flags): validates the live workspace under `crates/pathway/`.
//! - `--validate`: runs against synthetic fixtures in
//!   `crates/xtask/fixtures/pathway_choreography/` and expects at least one
//!   violation to be triggered, confirming the rules fire correctly.
//!
//! Scans: `.rs` files under the chosen root, filtered to pathway choreography
//! paths, checking for required module structure and forbidden patterns.
//! Registered as: `cargo xtask check pathway-choreography [--validate]`

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::util::{layer_for_rel_path, normalize_rel_path, workspace_root, Violation};

pub fn run(args: &[String]) -> Result<()> {
    let validate = args.iter().any(|arg| arg == "--validate");
    if args
        .iter()
        .any(|arg| arg != "--validate" && arg != "--strict")
    {
        bail!("pathway-choreography: usage: cargo xtask check pathway-choreography [--validate|--strict]");
    }

    let root = if validate {
        workspace_root()?.join("crates/xtask/fixtures/pathway_choreography")
    } else {
        workspace_root()?
    };
    let violations = collect_violations(&root)?;

    if validate {
        if violations.is_empty() {
            bail!("pathway-choreography: validation fixtures did not trigger any rule");
        }
        println!(
            "pathway-choreography: validation fixtures triggered {} violation(s)",
            violations.len()
        );
        return Ok(());
    }

    if violations.is_empty() {
        println!("pathway-choreography: OK");
        return Ok(());
    }

    eprintln!("pathway-choreography: violation(s)");
    for violation in &violations {
        eprintln!("  {}", violation.render());
    }
    bail!("pathway-choreography failed");
}

fn collect_violations(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    out.extend(no_parallel_tell_tree(root)?);
    out.extend(required_inline_protocol_modules_exist(root)?);
    out.extend(inline_protocol_modules_are_valid(root)?);
    out.extend(forbidden_runtime_effect_calls(root)?);
    out.extend(shared_crates_remain_runtime_free(root)?);
    out.extend(no_pathway_choreography_types_in_shared_crates(root)?);
    out.extend(router_does_not_depend_on_pathway_private_choreography(
        root,
    )?);
    Ok(out)
}

fn no_parallel_tell_tree(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    let choreography_root = root.join("crates/pathway/src/choreography");
    for path in files_with_extension(&choreography_root, "tell")? {
        let rel = normalize_rel_path(root, &path);
        out.push(Violation::with_layer(
            rel.clone(),
            1,
            "pathway choreography must use inline `tell!` modules; parallel `.tell` sources are forbidden",
            layer_for_rel_path(&rel),
        ));
    }
    Ok(out)
}

fn required_inline_protocol_modules_exist(root: &Path) -> Result<Vec<Violation>> {
    let expected = [
        "activation.rs",
        "anti_entropy.rs",
        "forwarding.rs",
        "handoff.rs",
        "hold_replay.rs",
        "neighbor_advertisement.rs",
        "repair.rs",
        "route_export.rs",
    ];
    let choreography_root = root.join("crates/pathway/src/choreography");
    let mut out = Vec::new();
    for expected_file in expected {
        let path = choreography_root.join(expected_file);
        if !path.exists() {
            out.push(Violation::with_layer(
                normalize_rel_path(root, &path),
                1,
                "required inline pathway choreography module is missing",
                crate::util::LayerTag::PathwayRouter,
            ));
        }
    }
    Ok(out)
}

fn inline_protocol_modules_are_valid(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    let choreography_root = root.join("crates/pathway/src/choreography");
    for path in rust_files(&choreography_root)? {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if matches!(
            file_name,
            "artifacts.rs" | "effects.rs" | "mod.rs" | "runtime.rs"
        ) {
            continue;
        }
        let rel = normalize_rel_path(root, &path);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        if !contents.contains("tell! {") {
            out.push(Violation::with_layer(
                rel.clone(),
                1,
                "inline pathway choreography modules must declare a `tell!` protocol",
                layer_for_rel_path(&rel),
            ));
        }
        for required in [
            "pub(crate) const SOURCE_PATH",
            "pub(crate) const PROTOCOL_NAME",
            "pub(crate) const ROLE_NAMES",
        ] {
            if !contents.contains(required) {
                out.push(Violation::with_layer(
                    rel.clone(),
                    1,
                    format!(
                        "inline pathway choreography module must contain `{required}`"
                    ),
                    layer_for_rel_path(&rel),
                ));
            }
        }
        let has_entrypoint = contents.contains("pub(crate) fn execute")
            || (file_name == "hold_replay.rs"
                && contents.contains("pub(crate) fn retain")
                && contents.contains("pub(crate) fn replay"));
        if !has_entrypoint {
            out.push(Violation::with_layer(
                rel.clone(),
                1,
                "inline pathway choreography module must expose its public protocol entrypoint",
                layer_for_rel_path(&rel),
            ));
        }
    }
    Ok(out)
}

fn forbidden_runtime_effect_calls(root: &Path) -> Result<Vec<Violation>> {
    let patterns = [
        ("self.transport.send_transport(", "pathway runtime must send transport payloads through choreography guest runtime"),
        ("self.transport.drain_transport_ingress(", "pathway runtime must drain ingress through choreography guest runtime"),
        ("self.retention.retain_payload(", "pathway runtime must retain payloads through choreography guest runtime"),
        ("self.retention.take_retained_payload(", "pathway runtime must recover retained payloads through choreography guest runtime"),
        ("self.effects.record_route_event(", "pathway runtime must record route events through choreography guest runtime"),
    ];
    let mut out = Vec::new();
    let engine_root = root.join("crates/pathway/src/engine");
    for path in rust_files(&engine_root)? {
        let rel = normalize_rel_path(root, &path);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        for (idx, line) in contents.lines().enumerate() {
            for (pattern, message) in patterns {
                if line.contains(pattern) {
                    out.push(Violation::with_layer(
                        rel.clone(),
                        idx + 1,
                        message,
                        layer_for_rel_path(&rel),
                    ));
                }
            }
        }
    }
    Ok(out)
}

fn shared_crates_remain_runtime_free(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    for dir in ["crates/core/src", "crates/traits/src"] {
        for path in rust_files(&root.join(dir))? {
            let rel = normalize_rel_path(root, &path);
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            for (idx, line) in contents.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                if line.contains("telltale") {
                    out.push(Violation::with_layer(
                        rel.clone(),
                        idx + 1,
                        "shared crates must stay runtime-free and must not import telltale",
                        layer_for_rel_path(&rel),
                    ));
                }
            }
        }
    }
    Ok(out)
}

fn no_pathway_choreography_types_in_shared_crates(
    root: &Path,
) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    for dir in ["crates/core/src", "crates/traits/src"] {
        for path in rust_files(&root.join(dir))? {
            let rel = normalize_rel_path(root, &path);
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            for (idx, line) in contents.lines().enumerate() {
                if line.contains("PathwayProtocol")
                    || line.contains("PathwayChoreo")
                    || line.contains("PathwayProtocolRuntime")
                {
                    out.push(Violation::with_layer(
                        rel.clone(),
                        idx + 1,
                        "pathway-private choreography types must not leak into shared crates",
                        layer_for_rel_path(&rel),
                    ));
                }
            }
        }
    }
    Ok(out)
}

fn router_does_not_depend_on_pathway_private_choreography(
    root: &Path,
) -> Result<Vec<Violation>> {
    let router_root = root.join("crates/router/src");
    if !router_root.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for path in rust_files(&router_root)? {
        let rel = normalize_rel_path(root, &path);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        for (idx, line) in contents.lines().enumerate() {
            if line.contains("PathwayProtocol")
                || line.contains("PathwayChoreo")
                || line.contains("PathwayProtocolRuntime")
                || line.contains("PathwayGuestRuntime")
            {
                out.push(Violation::with_layer(
                    rel.clone(),
                    idx + 1,
                    "router must not depend on pathway-private choreography internals",
                    layer_for_rel_path(&rel),
                ));
            }
        }
    }
    Ok(out)
}

fn rust_files(dir: &Path) -> Result<Vec<PathBuf>> {
    files_with_extension(dir, "rs")
}

fn files_with_extension(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
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
        if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}
