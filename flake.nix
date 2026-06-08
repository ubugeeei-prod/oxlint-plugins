{
  description = "Rust-backed oxlint plugin workspace";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      flake-utils,
      nixpkgs,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit overlays system; };
        rust = pkgs.rust-bin.stable."1.96.0".default.override {
          extensions = [
            "clippy"
            "rustfmt"
          ];
        };
        vp = pkgs.writeShellApplication {
          name = "vp";
          runtimeInputs = [
            pkgs.bash
            pkgs.curl
            pkgs.nodejs_24
          ];
          text = ''
            native_vp="$HOME/.vite-plus/bin/vp"
            fallback_vp="$HOME/.local/bin/vp"
            if [ ! -x "$native_vp" ] && [ ! -x "$fallback_vp" ]; then
              curl -fsSL https://vite.plus | bash
            fi

            if [ -x "$native_vp" ]; then
              exec "$native_vp" "$@"
            fi

            exec "$fallback_vp" "$@"
          '';
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            [
              pkgs.cargo-deny
              pkgs.cargo-insta
              pkgs.curl
              pkgs.git
              pkgs.jq
              pkgs.nodejs_24
              pkgs.openssl
              pkgs.pkg-config
              pkgs.python3
              rust
              vp
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];

          RUST_BACKTRACE = "1";
          LANG = "en_US.UTF-8";
          LC_ALL = "en_US.UTF-8";
          npm_config_python = "${pkgs.python3}/bin/python";

          shellHook = ''
            export LANG="en_US.UTF-8"
            export LC_ALL="en_US.UTF-8"
            export PATH="$PWD/node_modules/.bin:$PATH"
            echo "nix dev shell ready: run vp install, then vp build"
          '';
        };
      }
    );
}
