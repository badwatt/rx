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

      version = lib.removeSuffix "\n" (builtins.readFile ./VERSION);

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
          };
        };
      # The desktop companion.  GPUI brings a real dependency tree, so it lives
      # in its own crate (`gui/`) with its own lockfile; the CLI stays
      # dependency-free and keeps its strict coverage gate.
      mkRxUi =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "rxui";
          inherit version;
          # Keep the repository root as the source so the binary can embed the
          # shared VERSION file; cargo itself runs in `gui/`.
          src = ./.;
          cargoRoot = "gui";
          # `cargoRoot` only redirects the lockfile/vendor paths; the build,
          # check and install hooks need the subdirectory too.
          buildAndTestSubdir = "gui";
          cargoLock.lockFile = ./gui/Cargo.lock;

          nativeBuildInputs = [
            pkgs.makeWrapper
            pkgs.pkg-config
          ];
          buildInputs = with pkgs; [
            fontconfig
            freetype
            libx11
            libxcb
            libxcursor
            libxi
            libxkbcommon
            libxrandr
            wayland
            wayland-protocols
            vulkan-loader
          ];

          # Desktop entry + icon, so the app launches without a terminal and
          # shows up in launchers once the package is installed.
          postInstall = ''
            id=io.github.badwatt.rxui
            install -Dm644 ${./gui/io.github.badwatt.rxui.desktop} $out/share/applications/$id.desktop
            install -Dm644 ${./gui/assets/rxui.svg} $out/share/icons/hicolor/scalable/apps/$id.svg
            install -Dm644 ${./gui/assets/io.github.badwatt.rxui.png} $out/share/icons/hicolor/256x256/apps/$id.png
            install -Dm644 ${./gui/io.github.badwatt.rxui.metainfo.xml} $out/share/metainfo/$id.metainfo.xml
            substituteInPlace $out/share/metainfo/$id.metainfo.xml --replace-fail '@VERSION@' ${version}
          '';

          # Blade dlopens the Wayland client and the Vulkan loader, so patchelf
          # cannot record them: wayland and xkbcommon go in the RUNPATH (which
          # also covers dlopen).  The Vulkan loader must stay the system one —
          # NixOS exports /run/opengl-driver/lib, whose ICDs are built against
          # that driver — so it is only appended as a fallback.
          postFixup = ''
            patchelf --add-rpath ${
              pkgs.lib.makeLibraryPath [ pkgs.libxkbcommon pkgs.wayland ]
            } $out/bin/rxui
            wrapProgram $out/bin/rxui \
              --suffix LD_LIBRARY_PATH : ${pkgs.vulkan-loader}/lib
          '';

          meta = {
            description = "Desktop companion for rx, the NixOS update CLI";
            license = pkgs.lib.licenses.mit;
            mainProgram = "rxui";
            # Lets nixpkgs/desktop tooling find the bundled entry.
            desktopFileName = "io.github.badwatt.rxui";
            platforms = pkgs.lib.platforms.linux;
          };
        };
    in
    {
      # Opt-in desktop surface: `programs.rxui.enable = true;` installs the CLI
      # and the app and gives sudo one timestamp per user, so the password
      # entered once also covers the nested sudo of the rebuild.
      nixosModules.default =
        { config, lib, pkgs, ... }:
        let
          cfg = config.programs.rxui;
          system = pkgs.stdenv.hostPlatform.system;
        in
        {
          options.programs.rxui = {
            enable = lib.mkEnableOption "the rx NixOS system updater";

            globalSudoTimestamp = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = ''
                Set `Defaults timestamp_type=global` so a GUI-entered sudo
                password also covers the nested sudo of nixos-rebuild.
              '';
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [
              self.packages.${system}.rx
              self.packages.${system}.rxui
            ];

            security.sudo.extraConfig = lib.mkIf cfg.globalSudoTimestamp ''
              Defaults timestamp_type=global
              # nixos-rebuild elevates itself and resets the environment, so
              # the inner `sudo nix-env --set` never sees SUDO_ASKPASS unless
              # sudo keeps it. Without this the profile switch fails with
              # "a terminal is required to read the password".
              Defaults env_keep += "SUDO_ASKPASS"
            '';
          };
        };

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
          rxui = mkRxUi pkgs;
        }
      );

      checks = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          # `nix flake check` builds the package (and its man page) for the
          # current system.
          default = mkRx pkgs;
        }
      );

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
            pkg-config
            # `gui/` (GPUI) links against these at build time.
            fontconfig
            freetype
            libx11
            libxcb
            libxcursor
            libxi
            libxkbcommon
            libxrandr
            wayland
            wayland-protocols
            vulkan-loader
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
