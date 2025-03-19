{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachSystem ["x86_64-linux" "i686-linux" "aarch64-linux"]
    (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        checkArgs = {
          inherit self pkgs system;
        };
      in {
        packages = rec {
          nvmetcfg = pkgs.callPackage ./. {};
          nvmetcfg-static = pkgs.pkgsStatic.callPackage ./. {};
          nvmetcfg-coverage = nvmetcfg.overrideAttrs (o: {
            RUSTFLAGS = "-C instrument-coverage";
            dontStrip = true;
          });
          default = nvmetcfg;
        };
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            (rust-bin.stable.latest.default.override {
              extensions = ["llvm-tools-preview"];
            })
            cargo-bloat
            cargo-llvm-cov
            llvmPackages_17.bintools
          ];
        };

        checks = {
          loop = import ./tests/loop.nix checkArgs;
          tcp = import ./tests/tcp.nix checkArgs;
          tcp-ipv6 = import ./tests/tcp-ipv6.nix checkArgs;
          rdma = import ./tests/rdma.nix checkArgs;
        };

        formatter = pkgs.alejandra;
      }
    );
}
