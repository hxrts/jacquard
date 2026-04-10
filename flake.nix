{
  description = "Jacquard - Adaptive mesh routing with choreographic protocols";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    toolkit = {
      url = "github:hxrts/rust-toolkit";
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
        };

        toolkitPackages = toolkit.packages.${system};

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          just
          mdbook
          mdbook-mermaid
          ripgrep
          perl
          elan
          toolkitPackages.toolkit-xtask
          toolkitPackages.toolkit-fmt
          toolkitPackages.toolkit-install-dylint
          toolkitPackages.toolkit-dylint
          toolkitPackages.toolkit-dylint-link
        ];

        buildInputs =
          with pkgs;
          [
            openssl
          ]
          ++ lib.optionals stdenv.isDarwin [
            libiconv
          ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          shellHook = ''
            [[ -r "$HOME/.local/state/secrets/cargo-registry-token" ]] && export CARGO_REGISTRY_TOKEN="$(cat "$HOME/.local/state/secrets/cargo-registry-token")"
            export TOOLKIT_ROOT="${toolkit.outPath}"

            echo "Jacquard development environment"
            echo "Rust: $(rustc --version)"
            echo "Lean: $(elan show 2>/dev/null | head -1 || echo 'run: elan default leanprover/lean4:v4.26.0')"
          '';
        };
      }
    );
}
