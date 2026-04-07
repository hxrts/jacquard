//! Enforces the internal mesh choreography boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::util::{normalize_rel_path, workspace_root, Violation};
use anyhow::{bail, Context, Result};
use telltale::compile_choreography;

pub fn run(args: &[String]) -> Result<()> {
    let validate = args.iter().any(|arg| arg == "--validate");
    if args
        .iter()
        .any(|arg| arg != "--validate" && arg != "--strict")
    {
        bail!("mesh-choreography: usage: cargo xtask check mesh-choreography [--validate|--strict]");
    }

    let root = if validate {
        workspace_root()?.join("crates/xtask/fixtures/mesh_choreography")
    } else {
        workspace_root()?
    };
    let violations = collect_violations(&root)?;

    if validate {
        if violations.is_empty() {
            bail!("mesh-choreography: validation fixtures did not trigger any rule");
        }
        println!(
            "mesh-choreography: validation fixtures triggered {} violation(s)",
            violations.len()
        );
        return Ok(());
    }

    if violations.is_empty() {
        println!("mesh-choreography: OK");
        return Ok(());
    }

    eprintln!("mesh-choreography: violation(s)");
    for violation in &violations {
        eprintln!("  {}", violation.render());
    }
    bail!("mesh-choreography failed");
}

fn collect_violations(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    out.extend(choreography_sources_are_valid(root)?);
    out.extend(forbidden_runtime_effect_calls(root)?);
    out.extend(shared_crates_remain_runtime_free(root)?);
    out.extend(no_mesh_choreography_types_in_shared_crates(root)?);
    out.extend(router_does_not_depend_on_mesh_private_choreography(root)?);
    Ok(out)
}

fn choreography_sources_are_valid(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    let choreography_root = root.join("crates/mesh/src/choreography");
    for path in tell_files(&choreography_root)? {
        let rel = normalize_rel_path(root, &path);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;

        if !contents.contains("uses MeshRuntime, MeshAudit") {
            out.push(Violation::new(
                rel.clone(),
                1,
                "mesh choreography sources must declare explicit `uses MeshRuntime, MeshAudit`",
            ));
        }

        if let Err(error) = compile_choreography(&contents) {
            out.push(Violation::new(
                rel,
                1,
                format!(
                    "mesh choreography source must compile through telltale: {error}"
                ),
            ));
        }
    }
    Ok(out)
}

fn forbidden_runtime_effect_calls(root: &Path) -> Result<Vec<Violation>> {
    let patterns = [
        ("self.transport.send_frame(", "mesh runtime must send frames through choreography guest runtime"),
        ("self.transport.poll_observations(", "mesh runtime must poll ingress through choreography guest runtime"),
        ("self.retention.retain_payload(", "mesh runtime must retain payloads through choreography guest runtime"),
        ("self.retention.take_retained_payload(", "mesh runtime must recover retained payloads through choreography guest runtime"),
        ("self.effects.record_route_event(", "mesh runtime must record route events through choreography guest runtime"),
    ];
    let mut out = Vec::new();
    let engine_root = root.join("crates/mesh/src/engine");
    for path in rust_files(&engine_root)? {
        let rel = normalize_rel_path(root, &path);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        for (idx, line) in contents.lines().enumerate() {
            for (pattern, message) in patterns {
                if line.contains(pattern) {
                    out.push(Violation::new(rel.clone(), idx + 1, message));
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
                if line.contains("telltale") {
                    out.push(Violation::new(
                        rel.clone(),
                        idx + 1,
                        "shared crates must stay runtime-free and must not import telltale",
                    ));
                }
            }
        }
    }
    Ok(out)
}

fn no_mesh_choreography_types_in_shared_crates(root: &Path) -> Result<Vec<Violation>> {
    let mut out = Vec::new();
    for dir in ["crates/core/src", "crates/traits/src"] {
        for path in rust_files(&root.join(dir))? {
            let rel = normalize_rel_path(root, &path);
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            for (idx, line) in contents.lines().enumerate() {
                if line.contains("MeshProtocol")
                    || line.contains("MeshChoreo")
                    || line.contains("MeshProtocolRuntime")
                {
                    out.push(Violation::new(
                        rel.clone(),
                        idx + 1,
                        "mesh-private choreography types must not leak into shared crates",
                    ));
                }
            }
        }
    }
    Ok(out)
}

fn router_does_not_depend_on_mesh_private_choreography(
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
            if line.contains("MeshProtocol")
                || line.contains("MeshChoreo")
                || line.contains("MeshProtocolRuntime")
                || line.contains("MeshGuestRuntime")
            {
                out.push(Violation::new(
                    rel.clone(),
                    idx + 1,
                    "router must not depend on mesh-private choreography internals",
                ));
            }
        }
    }
    Ok(out)
}

fn rust_files(dir: &Path) -> Result<Vec<PathBuf>> {
    files_with_extension(dir, "rs")
}

fn tell_files(dir: &Path) -> Result<Vec<PathBuf>> {
    files_with_extension(dir, "tell")
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
