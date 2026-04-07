//! Fast pre-commit gate. Runs the staged-file gitignored guard plus
//! `cargo fmt --check` and `cargo check` scoped to the affected crates
//! so contributors get quick feedback before pushing.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};

use crate::util::{workspace_metadata, workspace_root};

pub fn run() -> Result<()> {
    println!("Running pre-commit checks...");

    let root = workspace_root()?;
    let staged_files = staged_files(&root)?;

    ensure_no_gitignored_files(&root, &staged_files)?;
    let staged_rs = staged_rust_files(staged_files);
    let crates = affected_crates(&root, &staged_rs)?;

    run_optional_cargo_check(
        "Checking formatting",
        &root,
        &crates,
        "fmt",
        &["--", "--check"],
        "Run `cargo fmt --all` to fix formatting",
    )?;
    run_optional_cargo_check(
        "Checking compilation",
        &root,
        &crates,
        "check",
        &["--all-targets", "--all-features"],
        "",
    )?;

    println!("Pre-commit checks passed!");
    Ok(())
}

fn ensure_no_gitignored_files(root: &Path, staged_files: &[String]) -> Result<()> {
    print!("Checking for gitignored files... ");
    let ignored: Vec<String> = staged_files
        .iter()
        .filter(|path| is_gitignored(root, path))
        .cloned()
        .collect();
    if !ignored.is_empty() {
        println!("FAILED");
        eprintln!("pre-commit: gitignored files must not be included in source control:");
        for path in &ignored {
            eprintln!("  {path}");
        }
        eprintln!("pre-commit: remove them with: git reset HEAD <file>");
        bail!("pre-commit failed");
    }
    println!("OK");
    Ok(())
}

fn staged_rust_files(staged_files: Vec<String>) -> Vec<String> {
    staged_files
        .into_iter()
        .filter(|path| path.ends_with(".rs"))
        .collect()
}

fn affected_crates(root: &Path, staged_rs: &[String]) -> Result<BTreeSet<String>> {
    if staged_rs.is_empty() {
        return Ok(BTreeSet::new());
    }
    owning_packages(root, staged_rs)
}

fn run_optional_cargo_check(
    label: &str,
    root: &Path,
    crates: &BTreeSet<String>,
    subcommand: &str,
    extra_args: &[&str],
    help_message: &str,
) -> Result<()> {
    print!("{label}... ");
    if crates.is_empty() {
        println!("OK (no Rust files staged)");
        return Ok(());
    }

    run_cargo(
        root,
        subcommand,
        &package_args(crates),
        extra_args,
        help_message,
    )?;
    println!("OK");
    Ok(())
}

fn staged_files(root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=d"])
        .current_dir(root)
        .output()
        .context("running git diff --cached")?;
    if !output.status.success() {
        bail!("pre-commit: failed to enumerate staged files");
    }
    Ok(String::from_utf8(output.stdout)
        .context("git diff output utf8")?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(std::string::ToString::to_string)
        .collect())
}

fn is_gitignored(root: &Path, rel_path: &str) -> bool {
    Command::new("git")
        .args(["check-ignore", "--no-index", "-q", rel_path])
        .current_dir(root)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn owning_packages(root: &Path, files: &[String]) -> Result<BTreeSet<String>> {
    let metadata = workspace_metadata()?;
    let package_dirs: Vec<(String, PathBuf)> = metadata
        .packages
        .iter()
        .map(|package| {
            let manifest = PathBuf::from(package.manifest_path.as_str());
            let dir = manifest.parent().unwrap_or(&manifest).to_path_buf();
            (package.name.clone(), dir)
        })
        .collect();

    let mut packages = BTreeSet::new();
    for rel in files {
        let abs = root.join(rel);
        let mut best: Option<(&str, usize)> = None;
        for (name, dir) in &package_dirs {
            if abs.starts_with(dir) {
                let depth = dir.components().count();
                match best {
                    Some((_, best_depth)) if best_depth >= depth => {}
                    _ => best = Some((name.as_str(), depth)),
                }
            }
        }
        if let Some((name, _)) = best {
            packages.insert(name.to_string());
        }
    }
    Ok(packages)
}

fn package_args(packages: &BTreeSet<String>) -> Vec<String> {
    let mut args = Vec::new();
    for package in packages {
        args.push("-p".to_string());
        args.push(package.clone());
    }
    args
}

fn run_cargo(
    root: &Path,
    subcommand: &str,
    package_args: &[String],
    extra_args: &[&str],
    help_message: &str,
) -> Result<()> {
    let mut command = Command::new("cargo");
    command.arg(subcommand);
    command.args(package_args);
    command.args(extra_args);
    command.current_dir(root);
    let status = command
        .status()
        .with_context(|| format!("running cargo {subcommand}"))?;
    if !status.success() {
        if !help_message.is_empty() {
            eprintln!("pre-commit: {help_message}");
        }
        bail!("pre-commit failed");
    }
    Ok(())
}
