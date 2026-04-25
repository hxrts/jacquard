#!/usr/bin/env bash

# Publishable crates in release order (leaves first, dependents last).
# Mirrors the production dependency graph — dev-only deps do not appear here.
#
# Dependency order:
#   macros → core → traits, host-support, cast-support
#   host-support → mem-node-profile, mem-link-profile
#   core/traits → pathway, field, scatter, babel, batman-classic, batman-bellman, olsrv2, mercator
#   router depends on shared crates; reference-client and simulator depend on router + engines
#   testkit comes last because it depends on most of the publishable surface
RELEASE_PACKAGES=(
  "jacquard-macros"
  "jacquard-core"
  "jacquard-traits"
  "jacquard-host-support"
  "jacquard-cast-support"
  "jacquard-mem-node-profile"
  "jacquard-mem-link-profile"
  "jacquard-babel"
  "jacquard-batman-bellman"
  "jacquard-batman-classic"
  "jacquard-olsrv2"
  "jacquard-pathway"
  "jacquard-scatter"
  "jacquard-mercator"
  "jacquard-router"
  "jacquard-reference-client"
  "jacquard-simulator"
  "jacquard-testkit"
)

manifest_path() {
  local crate="$1"
  case "${crate}" in
    jacquard-macros)           echo "crates/macros/Cargo.toml" ;;
    jacquard-core)             echo "crates/core/Cargo.toml" ;;
    jacquard-traits)           echo "crates/traits/Cargo.toml" ;;
    jacquard-host-support)     echo "crates/host-support/Cargo.toml" ;;
    jacquard-cast-support)     echo "crates/cast-support/Cargo.toml" ;;
    jacquard-mem-node-profile) echo "crates/mem-node-profile/Cargo.toml" ;;
    jacquard-mem-link-profile) echo "crates/mem-link-profile/Cargo.toml" ;;
    jacquard-babel)            echo "crates/babel/Cargo.toml" ;;
    jacquard-batman-bellman)   echo "crates/batman-bellman/Cargo.toml" ;;
    jacquard-batman-classic)   echo "crates/batman-classic/Cargo.toml" ;;
    jacquard-olsrv2)           echo "crates/olsrv2/Cargo.toml" ;;
    jacquard-pathway)          echo "crates/pathway/Cargo.toml" ;;
    jacquard-scatter)          echo "crates/scatter/Cargo.toml" ;;
    jacquard-mercator)         echo "crates/mercator/Cargo.toml" ;;
    jacquard-router)           echo "crates/router/Cargo.toml" ;;
    jacquard-reference-client) echo "crates/reference-client/Cargo.toml" ;;
    jacquard-simulator)        echo "crates/simulator/Cargo.toml" ;;
    jacquard-testkit)          echo "crates/testkit/Cargo.toml" ;;
    *)
      echo "unknown package: ${crate}" >&2
      return 1
      ;;
  esac
}
