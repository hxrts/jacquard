# Jacquard Model Policy Dylint

This lint library holds workspace policy checks that are broader than a single
proc-macro expansion site.

Current lints:

- handle-like public model types should carry `#[must_use_handle]`

Run it with:

```bash
nix develop ./nix/nightly
install-dylint
cargo dylint --path lints/model_policy --all -- --all-targets
```
