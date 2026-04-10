use std::{fs, sync::OnceLock};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::util::workspace_root;

#[derive(Clone, Debug, Default, Deserialize)]
struct ToolkitConfig {
    #[serde(default)]
    policy: PolicyConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct PolicyConfig {
    #[serde(default)]
    exemptions: ExemptionsConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ExemptionsConfig {
    #[serde(default)]
    bare_primitives_exempt_paths: Vec<String>,
    #[serde(default)]
    style_guide: Vec<StyleGuideException>,
    #[serde(default)]
    ownership_permits: Vec<OwnershipPermit>,
}

#[derive(Clone, Debug, Deserialize)]
struct StyleGuideException {
    symbol: String,
    reason: String,
}

#[derive(Clone, Debug, Deserialize)]
struct OwnershipPermit {
    name: String,
    reason: String,
}

fn config() -> Result<&'static ToolkitConfig> {
    static CONFIG: OnceLock<ToolkitConfig> = OnceLock::new();
    if let Some(config) = CONFIG.get() {
        return Ok(config);
    }

    let root = workspace_root()?;
    let path = root.join("policy/toolkit.toml");
    let contents =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let parsed: ToolkitConfig =
        toml::from_str(&contents).with_context(|| format!("parsing {}", path.display()))?;
    Ok(CONFIG.get_or_init(|| parsed))
}

#[allow(dead_code)]
pub fn bare_primitives_exempt_paths() -> Result<Vec<String>> {
    Ok(config()?
        .policy
        .exemptions
        .bare_primitives_exempt_paths
        .clone())
}

pub fn style_guide_exceptions() -> Result<Vec<(String, String)>> {
    Ok(config()?
        .policy
        .exemptions
        .style_guide
        .iter()
        .map(|entry| (entry.symbol.clone(), entry.reason.clone()))
        .collect())
}

#[allow(dead_code)]
pub fn ownership_permits() -> Result<Vec<(String, String)>> {
    Ok(config()?
        .policy
        .exemptions
        .ownership_permits
        .iter()
        .map(|entry| (entry.name.clone(), entry.reason.clone()))
        .collect())
}
