//! Validates that crate-level ownership and boundary docs stay explicit in the
//! shared crates.

use std::fs;

use anyhow::{bail, Context, Result};

use crate::util::{layer_for_rel_path, workspace_root, Violation};

struct DocRequirement {
    heading: &'static str,
    required_terms: &'static [&'static str],
}

struct FileRequirement {
    rel_path: &'static str,
    requirements: &'static [DocRequirement],
}

const CORE_REQUIREMENTS: &[DocRequirement] = &[
    DocRequirement {
        heading: "## Connectivity Surface",
        required_terms: &["LinkEndpoint", "TransportObservation"],
    },
    DocRequirement {
        heading: "## Service Surface",
        required_terms: &["ServiceDescriptor", "RouteServiceKind"],
    },
    DocRequirement {
        heading: "## Routing Engine Boundary",
        required_terms: &["RouteCandidate", "RouteMaterializationProof"],
    },
    DocRequirement {
        heading: "## Ownership",
        required_terms: &["canonical route truth", "jacquard-traits"],
    },
];

const TRAITS_REQUIREMENTS: &[DocRequirement] = &[
    DocRequirement {
        heading: "## Runtime-Free Boundary",
        required_terms: &["runtime-free", "telltale runtime"],
    },
    DocRequirement {
        heading: "## Effect Capabilities",
        required_terms: &["TransportEffects", "StorageEffects"],
    },
    DocRequirement {
        heading: "## Engine And Router Contracts",
        required_terms: &["RoutingEngine", "RouterManagedEngine", "RoutingMiddleware"],
    },
    DocRequirement {
        heading: "## Ownership",
        required_terms: &["canonical route truth", "route-private runtime state"],
    },
];

const REQUIRED_FILES: &[FileRequirement] = &[
    FileRequirement {
        rel_path: "crates/core/src/lib.rs",
        requirements: CORE_REQUIREMENTS,
    },
    FileRequirement {
        rel_path: "crates/traits/src/lib.rs",
        requirements: TRAITS_REQUIREMENTS,
    },
];

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    let mut violations = Vec::new();

    for file_requirement in REQUIRED_FILES {
        let path = root.join(file_requirement.rel_path);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        for requirement in file_requirement.requirements {
            let Some(section_body) = extract_section(&contents, requirement.heading)
            else {
                violations.push(Violation::with_layer(
                    file_requirement.rel_path,
                    1,
                    format!("crate-level docs must contain `{}`", requirement.heading),
                    layer_for_rel_path(file_requirement.rel_path),
                ));
                continue;
            };

            let normalized_section = normalize_section_text(section_body);
            for required_term in requirement.required_terms {
                if !normalized_section.contains(&normalize_section_text(required_term))
                {
                    violations.push(Violation::with_layer(
                        file_requirement.rel_path,
                        1,
                        format!(
                            "`{}` section must mention `{}`",
                            requirement.heading, required_term
                        ),
                        layer_for_rel_path(file_requirement.rel_path),
                    ));
                }
            }
        }
    }

    if violations.is_empty() {
        println!("ownership-invariants: OK");
        return Ok(());
    }

    eprintln!("ownership-invariants: violation(s)");
    for violation in &violations {
        eprintln!("  {}", violation.render());
    }
    bail!("ownership-invariants failed");
}

fn extract_section<'a>(contents: &'a str, heading: &str) -> Option<&'a str> {
    let start = contents.find(heading)?;
    let after_heading = &contents[start + heading.len()..];
    let next_heading = after_heading.find("\n//! ## ");
    Some(match next_heading {
        | Some(end) => &after_heading[..end],
        | None => after_heading,
    })
}

fn normalize_section_text(text: &str) -> String {
    text.replace("//!", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
