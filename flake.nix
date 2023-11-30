{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "i686-linux" "aarch64-linux" ]
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          checkArgs = {
            inherit self pkgs system;
          };
        in
        {
          packages = rec {
            nvmetcfg = pkgs.callPackage ./. {};
            default = nvmetcfg;
          };
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [ rust-bin.stable.latest.default cargo-bloat ];
          };

          checks = {
            loop = import ./tests/loop.nix checkArgs;
            tcp = import ./tests/tcp.nix checkArgs;
          };
        }
      );
}
