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
        | Some(other) => bail!("policy-xtask: unknown command: {other}"),
        | None => bail!(
            "policy-xtask: usage: cargo run --manifest-path policy/xtask/Cargo.toml -- <check|pre-commit> ..."
        ),
    }
}
