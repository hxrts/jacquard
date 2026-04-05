{
  description = "Jacquard nightly tooling shell for Dylint and rustc_private-based checks";

  inputs = {
    jacquard-root.url = "path:../..";
    nixpkgs.follows = "jacquard-root/nixpkgs";
    rust-overlay.follows = "jacquard-root/rust-overlay";
    flake-utils.follows = "jacquard-root/flake-utils";
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

        installDylint = pkgs.writeShellScriptBin "install-dylint" ''
          set -euo pipefail
          cargo install --locked cargo-dylint dylint-link
        '';
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            with pkgs;
            [
              rustToolchainNightly
              installDylint
              pkg-config
              just
              ripgrep
              perl
              openssl
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
