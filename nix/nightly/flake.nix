{
  description = "Jacquard nightly tooling shell for Dylint and rustc_private-based checks";

  # Inputs are declared directly (not followed from the root flake via
  # `path:../..`) because newer nix rejects mutable path locks on CI.
  # Keep these pins in rough sync with the root flake manually.
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchainNightly = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "rustc-dev"
            "llvm-tools-preview"
          ];
        };

        cargoWrapper = pkgs.writeShellScriptBin "cargo" ''
          set -euo pipefail
          if [ -z "''${RUSTUP_TOOLCHAIN:-}" ]; then
            host="$(rustc -vV | awk '/^host: / { print $2 }')"
            export RUSTUP_TOOLCHAIN="jacquard-nightly-''${host}"
          fi
          exec "$HOME/.cargo/bin/cargo" "$@"
        '';

        installDylint = pkgs.writeShellScriptBin "install-dylint" ''
          set -euo pipefail
          dylint_repo="''${XDG_CACHE_HOME:-$HOME/.cache}/jacquard/dylint"
          dylint_rev="4bd91ce7729b74c7ee5664bbb588f7baf30b4a09"
          mkdir -p "$(dirname "$dylint_repo")"
          if [ ! -d "$dylint_repo/.git" ]; then
            git clone https://github.com/trailofbits/dylint.git "$dylint_repo"
          fi
          git -C "$dylint_repo" fetch --tags origin
          git -C "$dylint_repo" checkout --force "$dylint_rev"
          ${rustToolchainNightly}/bin/cargo install --locked --force --path "$dylint_repo/cargo-dylint"
          ${rustToolchainNightly}/bin/cargo install --locked --force --path "$dylint_repo/dylint-link"
          host="$(rustc -vV | awk '/^host: / { print $2 }')"
          toolchain_name="jacquard-nightly-''${host}"
          toolchain_root="$(dirname "$(dirname "$(command -v rustc)")")"
          rustup toolchain remove "$toolchain_name" >/dev/null 2>&1 || true
          rustup toolchain link "$toolchain_name" "$toolchain_root"
          if [ -d "$PWD/lints" ]; then
            (cd "$PWD/lints" && rustup override set "$toolchain_name" >/dev/null)
          fi
        '';

        dylintLinkWrapper = pkgs.writeShellScriptBin "jacquard-dylint-link" ''
          set -euo pipefail
          if [ -z "''${RUSTUP_TOOLCHAIN:-}" ]; then
            host="$(rustc -vV | awk '/^host: / { print $2 }')"
            export RUSTUP_TOOLCHAIN="jacquard-nightly-''${host}"
          fi
          exec dylint-link "$@"
        '';

        # Nightly `cargo fmt` wrapper. Bypasses the rustup-linking
        # `cargoWrapper` above so contributors and CI can run
        # `cargo-fmt-nightly --all` without first running `install-dylint`.
        # This invokes the nix-native nightly cargo directly, which finds the
        # nightly rustfmt colocated in the same toolchain directory.
        cargoFmtNightly = pkgs.writeShellScriptBin "cargo-fmt-nightly" ''
          set -euo pipefail
          exec ${rustToolchainNightly}/bin/cargo fmt "$@"
        '';
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            with pkgs;
            [
              cargoWrapper
              rustToolchainNightly
              installDylint
              dylintLinkWrapper
              cargoFmtNightly
              pkg-config
              git
              just
              ripgrep
              perl
              openssl
              zlib
            ]
            ++ lib.optionals stdenv.isDarwin [
              libiconv
            ];

          shellHook = ''
            echo "Jacquard nightly lint environment"
            echo "Rust: $(rustc --version)"
            echo "Run 'install-dylint' once in this shell if cargo-dylint is not installed."
          '';
        };
      }
    );
}
