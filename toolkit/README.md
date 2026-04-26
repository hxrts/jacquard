# Toolkit

Jacquard-owned toolkit layer for rules that should not live inside the reusable
toolkit.

Use this directory for:

- Jacquard-specific toolkit configuration
- Jacquard-only checks
- Jacquard-only lints
- Jacquard-specific fixtures
- explicit exemption data declared in `toolkit.toml`
- docs that depend on Jacquard architecture terms

Jacquard consumes the reusable toolkit as a flake input. The default repo dev
shell exports toolkit-owned commands such as `toolkit-xtask`,
`toolkit-install-dylint`, and `toolkit-dylint`, and `scripts/toolkit-shell.sh`
is only a thin bootstrap for running those commands outside an active
`nix develop` shell.

## Layout

```text
toolkit/
  README.md
  toolkit.toml
  checks/
    rust/
    pre_commit.rs
  lints/
  fixtures/
  docs/
```

## Ownership Rule

- generic enforcement belongs in the toolkit
- Jacquard-specific semantics belong here

If a rule is generic and only needs Jacquard-specific scope, configure it in
`toolkit.toml`.

If a rule depends on Jacquard crate topology, routing semantics, transport
ownership, or other Jacquard architecture concepts, implement it under
`toolkit/`.
