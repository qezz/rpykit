{
  description = "rpykit env";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs-mozilla.url = "github:mozilla/nixpkgs-mozilla";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      nixpkgs-mozilla,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import nixpkgs-mozilla) ];
        };

        rusttoolchain =
          (pkgs.rustChannelOf {
            rustToolchain = ./rust-toolchain.toml;
            sha256 = "sha256-+9FmLhAOezBZCOziO0Qct1NOrfpjNsXxc/8I0c7BdKE=";
          }).rust;
      in
      {
        devShells = {
          default = pkgs.mkShell {
            buildInputs = [ pkgs.openssl ];
            nativeBuildInputs = with pkgs; [
              nixpkgs-fmt
              rusttoolchain
              pkg-config
              python312
              perf
            ];

            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          };
        };
      }
    );
}
