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

      # Reverse-DNS identity of the desktop surface: app id, window class,
      # desktop entry and icon all carry it.
      appId = "io.github.badwatt.rxui";

      # Libraries `gui/` links against and needs at runtime.
      guiLibraries =
        pkgs:
        with pkgs;
        [
          fontconfig
          freetype
          libx11
          libxcb
          libxcursor
          libxi
          libxkbcommon
          libxrandr
          vulkan-loader
          wayland
          wayland-protocols
        ];

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
          buildInputs = guiLibraries pkgs;

          # Desktop entry + icon, so the app launches without a terminal and
          # shows up in launchers once the package is installed.
          postInstall = ''
            install -Dm644 ${./gui/io.github.badwatt.rxui.desktop} \
              $out/share/applications/${appId}.desktop
            install -Dm644 ${./gui/assets/rxui.svg} \
              $out/share/icons/hicolor/scalable/apps/${appId}.svg
            install -Dm644 ${./gui/assets/io.github.badwatt.rxui.png} \
              $out/share/icons/hicolor/256x256/apps/${appId}.png
            install -Dm644 ${./gui/io.github.badwatt.rxui.metainfo.xml} \
              $out/share/metainfo/${appId}.metainfo.xml
            substituteInPlace $out/share/metainfo/${appId}.metainfo.xml \
              --replace-fail '@VERSION@' ${version}
          '';

          # Blade dlopens the Wayland client and the Vulkan loader, so patchelf
          # cannot record them: wayland and xkbcommon go in the RUNPATH (which
          # also covers dlopen).  The Vulkan loader must stay the system one —
          # NixOS exports /run/opengl-driver/lib, whose ICDs are built against
          # that driver — so it is only appended as a fallback.
          postFixup =
            let
              runPath = pkgs.lib.makeLibraryPath [ pkgs.libxkbcommon pkgs.wayland ];
            in
            ''
              patchelf --add-rpath ${runPath} $out/bin/rxui
              wrapProgram $out/bin/rxui \
                --suffix LD_LIBRARY_PATH : ${pkgs.vulkan-loader}/lib
            '';

          meta = {
            description = "Desktop companion for rx, the NixOS update CLI";
            license = pkgs.lib.licenses.mit;
            mainProgram = "rxui";
            # Lets nixpkgs/desktop tooling find the bundled entry.
            desktopFileName = appId;
            platforms = pkgs.lib.platforms.linux;
            homepage = "https://github.com/badwatt/rx";
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

            packages = toolchain ++ guiLibraries pkgs;
          };
        }
      );

      formatter = forAllSystems (system: (import nixpkgs { inherit system; }).nixfmt-rfc-style);
    };
}
