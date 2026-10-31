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
        "i686-linux"
      ];
      allSystems = darwin ++ linux;

      forEachSystem = systems: f: lib.genAttrs systems (system: f system);
      forAllSystems = forEachSystem allSystems;

      version = lib.trim (builtins.readFile ./VERSION);

      mkRx =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "rx";
          inherit version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          postInstall = ''
            install -Dm644 ${./rx.1} "$out/share/man/man1/rx.1"
          '';
          meta = {
            description = "A minimal CLI to pull, stage, rebuild and clean up your NixOS system";
            license = pkgs.lib.licenses.mit;
            mainProgram = "rx";
            platforms = pkgs.lib.platforms.unix;
            homepage = "https://github.com/badwatt/rx";
          };
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          rx = mkRx pkgs;
        in
        {
          # e.g. nix build .#rx  (or plain: nix build)
          default = rx;
          rx = rx;
        }
      );

      checks = forAllSystems (system: {
        # `nix flake check` builds the package (and its man page) for the
        # current system; the derivation is the same one `packages` exposes.
        default = self.packages.${system}.default;
      });

      # Cross builds live under legacyPackages so `packages` stays purely
      # derivations and `nix flake show` stays quiet.
      # e.g. nix build .#cross.x86_64-linux-musl.default --impure
      legacyPackages = forAllSystems (
        system:
        {
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
          toolchain = with pkgs; [
            gcc
            gnumake
            clippy
            rustc
            cargo
            rustfmt
            rust-analyzer
            cargo-llvm-cov
            llvm
            pkg-config
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

            packages = toolchain;
          };
        }
      );

      formatter = forAllSystems (system: (import nixpkgs { inherit system; }).nixfmt-rfc-style);
    };
}
