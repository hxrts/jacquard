//! `cargo xtask` entry point. Dispatches `check <name>` and `pre-commit`
//! sub-commands to the corresponding module under `checks::`.
//!
//! Usage:
//!   `cargo xtask check <name>` — run a single named policy check
//!   `cargo xtask pre-commit`   — run the staged-file pre-commit lane
//!
//! Registered check names are listed in `checks/mod.rs`. Each check
//! enforces one workspace invariant and is also reachable from CI and
//! the git pre-commit hook installed by `just install-hooks`. Unknown
//! sub-commands or missing arguments produce an informative error.

#![forbid(unsafe_code)]

mod checks;
mod exemptions;
mod sources;
mod util;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        | Some("check") => checks::run(args.collect()),
        | Some("pre-commit") => checks::pre_commit::run(),
        | Some(other) => bail!("xtask: unknown command: {other}"),
        | None => bail!("xtask: usage: cargo xtask <check|pre-commit> ..."),
    }
}
