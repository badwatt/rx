{
  description = "rx — pull, stage, rebuild and clean up your NixOS system";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";

    # nixpkgs lags a stable release, and the project pins its toolchain in
    # rust-toolchain.toml: this overlay turns that file into the toolchain
    # used by the shell and by the package.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, rust-overlay, ... }:
    let
      lib = nixpkgs.lib;

      # Native systems plus the cross targets the release matrix needs.
      allSystems = [
        "x86_64-darwin"
        "aarch64-darwin"
        "x86_64-linux"
        "x86_64-linux-musl"
        "aarch64-linux"
        "i686-linux"
      ];

      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      forAllSystems = lib.genAttrs allSystems;

      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;

      # The toolchain `rust-toolchain.toml` pins, for the shell and the package.
      rustFor = pkgs: pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      mkRx =
        pkgs: rustPlatform:
        rustPlatform.buildRustPackage {
          pname = "rx";
          inherit version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.makeWrapper ];
          postInstall = ''
            # The version lives in Cargo.toml: render it into the man page.
            install -d "$out/share/man/man1"
            substitute ${./rx.1} "$out/share/man/man1/rx.1" \
              --replace-fail '@VERSION@' "${version}"

            # The binary prints its own completions, so they never drift from
            # the parser.
            install -d "$out/share/bash-completion/completions" "$out/share/zsh/site-functions" "$out/share/fish/vendor_completions.d"
            "$out/bin/rx" --completions bash > "$out/share/bash-completion/completions/rx"
            "$out/bin/rx" --completions zsh  > "$out/share/zsh/site-functions/_rx"
            "$out/bin/rx" --completions fish > "$out/share/fish/vendor_completions.d/rx.fish"
          '';
          # The rebuild pipes its log through nom: ship it with the binary, so
          # `nix run`/`nix build` users need nothing on their PATH.
          postFixup = ''
            wrapProgram "$out/bin/rx" \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nix-output-monitor ]}
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
          pkgs = pkgsFor system;
          rust = rustFor pkgs;
          rx = mkRx pkgs (pkgs.makeRustPlatform { cargo = rust; rustc = rust; });
        in
        {
          # e.g. nix build .#rx  (or plain: nix build)
          default = rx;
          rx = rx;
        }
      );

      # `nix flake check` builds the package (and its man page) for the current
      # system; the derivation is the same one `packages` exposes.
      checks = forAllSystems (system: {
        default = self.packages.${system}.default;
      });

      # Cross builds live under legacyPackages so `packages` stays purely
      # derivations and `nix flake show` stays quiet.
      # e.g. nix build .#cross.x86_64-linux-musl.default --impure
      legacyPackages = forAllSystems (
        system:
        {
          cross = lib.genAttrs (lib.filter (target: target != system) allSystems) (target: {
            default =
              let
                crossPkgs = import nixpkgs {
                  inherit system;
                  crossSystem = target;
                };
              in
              mkRx crossPkgs crossPkgs.rustPlatform;
          });
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          rust = rustFor pkgs;
        in
        {
          default = pkgs.mkShell {
            RUST_BACKTRACE = 1;

            # For cross compilation
            NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM = 1;

            # cargo-llvm-cov needs the LLVM that matches the compiler: take it
            # from the toolchain (llvm-tools component) instead of nixpkgs.
            LLVM_COV = "${rust}/lib/rustlib/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/bin/llvm-cov";
            LLVM_PROFDATA = "${rust}/lib/rustlib/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/bin/llvm-profdata";

            packages = with pkgs; [
              actionlint
              cargo-llvm-cov
              gcc
              gnumake
              nix-output-monitor
              pkg-config
              rust
              rust-analyzer
            ];
          };
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt-rfc-style);
    };
}
