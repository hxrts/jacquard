{
  description = "Jacquard - Adaptive mesh routing with choreographic protocols";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
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

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          just
          mdbook
          mdbook-mermaid
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
            echo "Jacquard development environment"
            echo "Rust: $(rustc --version)"
          '';
        };
      }
    );
}
