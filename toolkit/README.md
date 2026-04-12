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
    lean/
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

## Lean Style

Generic Lean source-style enforcement is toolkit-owned and configured here
through `[checks.lean_style]` in `toolkit.toml`.

Current rollout:

- `just lean-style` runs the generic Lean source-style checker only
- `just lean-check` runs `just lean-style`, then `just lean-setup`, then
  `lake build`
- CI currently enforces the style checker, not the full Lean build, so the
  toolkit lane stays fast while the verification build remains a separate
  developer workflow

That keeps the Lean style check blocking in the normal repo workflow without
pretending the full Lean build is already CI-portable.
