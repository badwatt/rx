{
  description = "RX";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      lib = nixpkgs.lib;

      darwin = [
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      linux = [
        "x86_64-linux"
        "x86_64-linux-musl"
        "aarch64-linux"
        "aarch64-linux-android"
        "i686-linux"
      ];
      allSystems = darwin ++ linux;

      forEachSystem = systems: f: lib.genAttrs systems (system: f system);
      forAllSystems = forEachSystem allSystems;

      mkRx =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "rx";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          # e.g. nix build .#rx  (or plain: nix build)
          default = mkRx pkgs;
          rx = mkRx pkgs;

          # e.g. nix build .#cross.x86_64-linux-musl.default --impure
          cross = forEachSystem (lib.filter (sys: sys != system) allSystems) (
            targetSystem:
            let
              crossPkgs = import nixpkgs {
                inherit system;
                crossSystem = targetSystem;
              };
            in
            {
              default = mkRx crossPkgs;
            }
          );
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          llvm = pkgs.llvmPackages.llvm;
          devRequirements = with pkgs; [
            gcc
            gnumake
            clippy
            rustc
            cargo
            rustfmt
            rust-analyzer
            cargo-llvm-cov
            llvm
          ];
        in
        {
          default = pkgs.mkShell {
            RUST_BACKTRACE = 1;

            # For cross compilation
            NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM = 1;

            # Let cargo-llvm-cov find the LLVM tools.
            LLVM_COV = "${llvm}/bin/llvm-cov";
            LLVM_PROFDATA = "${llvm}/bin/llvm-profdata";

            buildInputs = devRequirements;
            packages = devRequirements;
          };
        }
      );
    };
}
