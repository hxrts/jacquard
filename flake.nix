{
  description = "Jacquard - Adaptive mesh routing with choreographic protocols";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    toolkit = {
      url = "github:hxrts/toolkit";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      toolkit,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [
          (import rust-overlay)
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
          targets = [
            "wasm32-unknown-unknown"
          ];
        };

        toolkitSupport = toolkit.lib.${system}.consumerShellSupport;

        toolkitPackages = toolkit.packages.${system};

        pythonEnv = pkgs.python3.withPackages (
          ps: with ps; [
            polars
            altair
            vl-convert-python
            reportlab
            svglib
          ]
        );

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pythonEnv
          pkg-config
          just
          mdbook
          mdbook-mermaid
          ripgrep
          perl
          elan
          nodejs
        ]
        ++ toolkitSupport.packages
        ++ [
          toolkitPackages.toolkit-clippy
          toolkitPackages.toolkit-dylint
          toolkitPackages.toolkit-dylint-link
          toolkitPackages.toolkit-install-dylint
        ];

        buildInputs =
          with pkgs;
          [
            openssl
          ]
          ++ toolkitSupport.buildInputs;

      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          shellHook = ''
            [[ -r "$HOME/.local/state/secrets/cargo-registry-token" ]] && export CARGO_REGISTRY_TOKEN="$(cat "$HOME/.local/state/secrets/cargo-registry-token")"
            ${toolkitSupport.shellHook}

            echo "Jacquard development environment"
            echo "Rust: $(rustc --version)"
            echo "Lean: $(elan show 2>/dev/null | head -1 || echo 'run: elan default leanprover/lean4:v4.26.0')"
          '';
        };
      }
    );
}
