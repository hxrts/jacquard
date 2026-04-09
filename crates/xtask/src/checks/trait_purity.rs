//! Rejects public traits under `crates/traits/src` that lack purity
//! annotations.
//!
//! Every public trait in the `L2` (traits) layer must carry either
//! `#[purity(...)]` or `#[effect_trait]` to make its side-effect contract
//! explicit. This is the stable-toolchain fast-path mirror of the nightly
//! dylint rule in `lints/trait_purity`, allowing CI to enforce the rule
//! without requiring a nightly compiler.
//!
//! Exempt traits: `Sealed`, `EffectDefinition`, `HandlerDefinition` — these
//! are infrastructure traits for the macro system that do not need purity
//! annotations. Scans: all parsed sources in `crates/traits/src/`.
//! Registered as: `cargo xtask check trait-purity`

use anyhow::{bail, Result};

use crate::sources::{attributes_match, parse_workspace_sources, public_traits};

pub fn run() -> Result<()> {
    let parsed = parse_workspace_sources()?;
    let mut missing = Vec::new();

    for source in &parsed {
        if !source.rel_path.starts_with("crates/traits/src/") {
            continue;
        }
        for item_trait in public_traits(source) {
            let name = item_trait.ident.to_string();
            if matches!(
                name.as_str(),
                "Sealed" | "EffectDefinition" | "HandlerDefinition"
            ) {
                continue;
            }
            if attributes_match(&item_trait.attrs, "purity")
                || attributes_match(&item_trait.attrs, "effect_trait")
            {
                continue;
            }
            missing.push(format!("{}:{name}", source.rel_path));
        }
    }

    if !missing.is_empty() {
        eprintln!(
            "trait-purity: public traits missing #[purity(...)] or #[effect_trait]:"
        );
        for entry in &missing {
            eprintln!("  {entry}");
        }
        bail!("trait-purity failed");
    }

    println!("trait-purity: all public traits are annotated");
    Ok(())
}
