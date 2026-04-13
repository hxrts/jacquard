#!/usr/bin/env bash

# Publishable crates in release order (leaves first, dependents last).
# Mirrors the production dependency graph — dev-only deps do not appear here.
#
# Dependency order:
#   macros → core → traits, adapter, mem-node-profile
#   adapter → mem-link-profile → pathway, batman, field → router
#   router → reference-client → field-client → simulator
RELEASE_PACKAGES=(
  "jacquard-macros"
  "jacquard-core"
  "jacquard-traits"
  "jacquard-adapter"
  "jacquard-mem-node-profile"
  "jacquard-mem-link-profile"
  "jacquard-pathway"
  "jacquard-batman"
  "jacquard-field"
  "jacquard-router"
  "jacquard-reference-client"
  "jacquard-field-client"
  "jacquard-simulator"
)

manifest_path() {
  local crate="$1"
  case "${crate}" in
    jacquard-macros)           echo "crates/macros/Cargo.toml" ;;
    jacquard-core)             echo "crates/core/Cargo.toml" ;;
    jacquard-traits)           echo "crates/traits/Cargo.toml" ;;
    jacquard-adapter)          echo "crates/adapter/Cargo.toml" ;;
    jacquard-mem-node-profile) echo "crates/mem-node-profile/Cargo.toml" ;;
    jacquard-mem-link-profile) echo "crates/mem-link-profile/Cargo.toml" ;;
    jacquard-pathway)          echo "crates/pathway/Cargo.toml" ;;
    jacquard-batman)           echo "crates/batman/Cargo.toml" ;;
    jacquard-field)            echo "crates/field/Cargo.toml" ;;
    jacquard-router)           echo "crates/router/Cargo.toml" ;;
    jacquard-reference-client) echo "crates/reference-client/Cargo.toml" ;;
    jacquard-field-client)     echo "crates/field-client/Cargo.toml" ;;
    jacquard-simulator)        echo "crates/simulator/Cargo.toml" ;;
    *)
      echo "unknown package: ${crate}" >&2
      return 1
      ;;
  esac
}
