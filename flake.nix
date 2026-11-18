{
  description = "Tessera: an iTerm2-style terminal (Rust + cxx-qt + libghostty-vt)";

  # Pinned to the same nixpkgs revision as /etc/nixos, so the app uses the
  # Qt build the desktop's style plugins were made for.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      # (qtwebengine: rich output (HTML) in notebook cells.)
      qtFor = pkgs: with pkgs.qt6; [ qtbase qtdeclarative qtsvg qtwayland qtwebengine qtwebchannel qtpositioning ];
      qtEnvFor = pkgs: pkgs.qt6.env "qt-tessera-${pkgs.qt6.qtbase.version}" ((builtins.tail (qtFor pkgs)) ++ [ pkgs.libGL ]);
    in
    {
      # `nix build` / `nix run github:benwbooth/tessera`.
      packages = forAll (pkgs:
        let
          # The Ghostty commit libghostty-vt-sys pins (its build.rs), and the
          # Zig packages it needs, fetched ahead of time (builds are offline).
          ghosttySrc = pkgs.fetchFromGitHub {
            owner = "ghostty-org";
            repo = "ghostty";
            rev = "a887df42c56f6de86c0fe6da9c4eeca37931e083";
            hash = "sha256-1Zz65SCk3rkJ9+Q0MmyNOTNiDSLBRIHRd3IvFM4iNXw=";
          };
          zigDeps = pkgs.stdenvNoCC.mkDerivation {
            name = "ghostty-zig-deps";
            src = ghosttySrc;
            nativeBuildInputs = [ pkgs.zig_0_15 ];
            dontConfigure = true;
            dontFixup = true;
            buildPhase = ''
              export ZIG_GLOBAL_CACHE_DIR=$TMPDIR/zig
              zig build --fetch=all --global-cache-dir $TMPDIR/zig
            '';
            installPhase = "cp -r $TMPDIR/zig/p $out";
            outputHashMode = "recursive";
            outputHashAlgo = "sha256";
            outputHash = "sha256-PnM+hZIlLyQwK8vJgd/Bhjt1lNIz06T8FahwliRmMrY=";
          };
          qtEnv = qtEnvFor pkgs;
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type: !(pkgs.lib.hasInfix "/target" path) && baseNameOf path != ".qt-env" && baseNameOf path != "result";
          };
          ghosttyEnv = {
            GHOSTTY_SOURCE_DIR = "${ghosttySrc}";
            GHOSTTY_ZIG_SYSTEM_DIR = "${zigDeps}";
          };
          ghosttyPreBuild = ''
            export ZIG_GLOBAL_CACHE_DIR=$TMPDIR/zig-cache
            # zig writes into the source tree: give it a writable copy.
            cp -r ${ghosttySrc} $TMPDIR/ghostty && chmod -R u+w $TMPDIR/ghostty
            export GHOSTTY_SOURCE_DIR=$TMPDIR/ghostty
          '';
          # tesserad alone, static (musl): it runs on any Linux of its
          # architecture, so the window copies it to SSH hosts that lack it.
          tesseradStatic = pkgs.pkgsStatic.rustPlatform.buildRustPackage {
            pname = "tesserad";
            version = (pkgs.lib.importTOML ./tesserad/Cargo.toml).package.version;
            inherit src;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [ "-p" "tesserad" ];
            nativeBuildInputs = [ pkgs.zig_0_15 ];
            dontUseZigBuild = true;
            dontUseZigInstall = true;
            dontUseZigCheck = true;
            doCheck = false;
            env = ghosttyEnv;
            preBuild = ghosttyPreBuild;
            meta = {
              description = "Tessera's backend (shells, notebook kernels, cells, Nix runs), static";
              license = pkgs.lib.licenses.mit;
              mainProgram = "tesserad";
              platforms = pkgs.lib.platforms.linux;
            };
          };
          tessera = pkgs.rustPlatform.buildRustPackage {
            pname = "tessera";
            version = (pkgs.lib.importTOML ./Cargo.toml).package.version;
            inherit src;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config pkgs.zig_0_15 pkgs.qt6.wrapQtAppsHook pkgs.qt6.qtbase ];
            buildInputs = qtFor pkgs ++ [ qtEnv pkgs.libGL ];
            dontUseZigBuild = true;
            dontUseZigInstall = true;
            dontUseZigCheck = true;
            doCheck = false;
            env = ghosttyEnv // {
              # cxx-qt wants every Qt module under one prefix.
              QMAKE = "${qtEnv}/bin/qmake";
            };
            preBuild = ''
              # (Qt's setup hook sets QMAKE to qtbase's alone.)
              export QMAKE=${qtEnv}/bin/qmake
            '' + ghosttyPreBuild;
            postInstall = ''
              # The static tesserad, for SSH hosts without one (remote.rs).
              install -Dm755 ${tesseradStatic}/bin/tesserad $out/libexec/tessera/tesserad-${pkgs.stdenv.hostPlatform.uname.processor}-linux
              install -Dm644 icons/tessera.svg $out/share/icons/hicolor/scalable/apps/tessera.svg
              # (Outside a Flatpak the app id is plain "tessera".)
              mkdir -p $out/share/applications
              sed 's/^Icon=.*/Icon=tessera/' packaging/linux/io.github.benwbooth.Tessera.desktop > $out/share/applications/tessera.desktop
            '';
            meta = {
              description = "A terminal in the spirit of iTerm2 (Rust + Qt Quick + libghostty-vt)";
              homepage = "https://github.com/benwbooth/tessera";
              license = pkgs.lib.licenses.mit;
              mainProgram = "tessera";
              platforms = pkgs.lib.platforms.linux;
            };
          };
        in
        { default = tessera; inherit tessera; tesserad-static = tesseradStatic; });

      apps = forAll (pkgs: {
        default = { type = "app"; program = "${self.packages.${pkgs.system}.default}/bin/tessera"; };
      });

      # `nix develop`: Rust, the same Qt, and Zig 0.15 for libghostty-vt.
      devShells = forAll (pkgs:
        let qtEnv = qtEnvFor pkgs; in
        {
          default = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.clippy pkgs.rustfmt pkgs.pkg-config qtEnv pkgs.libGL pkgs.zig_0_15 pkgs.git pkgs.watchexec pkgs.tmux ];
            QMAKE = "${qtEnv}/bin/qmake";
            # Link with this shell's compiler (a system clang links against the
            # system glibc and the binary can't load this nixpkgs' Qt).
            "CARGO_TARGET_${pkgs.stdenv.hostPlatform.rust.cargoEnvVarTarget}_LINKER" = "cc";
            shellHook = ''
              ln -sfn ${qtEnv} .qt-env
              export NIX_LDFLAGS="''${NIX_LDFLAGS//"-rpath $out/lib"/}"
            '';
          };
        });
    };
}
