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
          # tessh, Tessera's shell language: a Rust program that embeds Python
          # (PyO3). Packages come from PyPI with uv (`need numpy`): ready-made,
          # where nixpkgs only prebuilds its default Python's.
          tesshPython = pkgs.python315;
          tesshWrap = bin: ''
            wrapProgram ${bin} \
              --suffix PATH : ${pkgs.uv}/bin
          '';
          tessh = pkgs.rustPlatform.buildRustPackage {
            pname = "tessh";
            version = (pkgs.lib.importTOML ./tessh/Cargo.toml).package.version;
            inherit src;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [ "-p" "tessh" ];
            nativeBuildInputs = [ tesshPython pkgs.makeWrapper ];
            buildInputs = [ tesshPython ];
            doCheck = false;
            env.PYO3_PYTHON = "${tesshPython}/bin/python3";
            postInstall = tesshWrap "$out/bin/tessh";
            meta = {
              description = "tessh: a shell language that compiles to Python (indentation, infix and shell operators, hygienic macros)";
              license = pkgs.lib.licenses.mit;
              mainProgram = "tessh";
              platforms = pkgs.lib.platforms.unix;
            };
          };
          # tessh alone, as one static file that runs on any x86-64 Linux
          # (NixOS and bare containers too): `nix build .#tessh-static`, or
          # scripts/tessh-static.sh for a release copy. tessh is built for
          # musl against Python's stable ABI (the stable-ABI symbols of a
          # python-build-standalone CPython 3.12, the oldest it takes), and
          # carried inside tessh/launcher with musl's loader; the launcher
          # links in the CPython uv installs (any 3.12+) when it starts.
          standalone = pkgs.lib.optionalAttrs (builtins.elem pkgs.stdenv.hostPlatform.system [ "x86_64-linux" "aarch64-linux" ]) (
            let
              # (For this machine's architecture: x86_64 or aarch64.)
              arch = pkgs.stdenv.hostPlatform.parsed.cpu.name;
              hashes = {
                x86_64 = {
                  musl = "sha256-dpPyfF8VNrbEt1EIrm04iLf4T6HUMAAfoPDMmy/Arzs=";
                  gnu = "sha256-yFgR01lx0GYSWFoFfbO3GAY3jfbqhHYnX7Fqpcn+VqA=";
                  uv = "sha256-WMz18XkrilqI0BvJEOoTGos9tKMZoXrV+tdeCEWj4nE=";
                  pyMusl = "sha256-QgkyPGvS0XT9Xbl/imR0iWxPF/9mPW6/zL+KA1PbzYg=";
                  pyGnu = "sha256-GMVuEFuncTYCU29Jl2xp0U1zzqfTLP4sto64p5Vo3kc=";
                };
                aarch64 = {
                  musl = "sha256-a6ocPUQsOAoFIdakUAVDDch3tweJ9WcTczvqeJSQL0I=";
                  gnu = "sha256-l7yh0JYCQLchtuWOwtwIBRn80R+Xm/ZDVkNK6dD5aUc=";
                  uv = "sha256-N/u3xbxmz5nBKqiBlJ0hYYHiAd7ESXcsQynlZIfWEXk=";
                  pyMusl = "sha256-hsCbjHjb70HvMGI5gJxT/96FZdekkrSe+owpGU8SkIw=";
                  pyGnu = "sha256-ibsHYwxwVvX+4Gbl//x8moFB9vKlpEAusccrM1jiaAk=";
                };
              }.${arch};
              pbsUrl = libc: "https://github.com/astral-sh/python-build-standalone/releases/download/20261123/cpython-3.12.16%2B20261123-${arch}-unknown-linux-${libc}-install_only_stripped.tar.gz";
              # The CPython each file carries (the newest released one).
              pyVersion = "3.15.2";
              pyBundled = libc: pkgs.fetchurl {
                url = "https://github.com/astral-sh/python-build-standalone/releases/download/20261123/cpython-${pyVersion}%2B20261123-${arch}-unknown-linux-${libc}-install_only_stripped.tar.gz";
                hash = if libc == "musl" then hashes.pyMusl else hashes.pyGnu;
              };
              cross = if arch == "aarch64" then pkgs.pkgsCross.aarch64-multiplatform-musl else pkgs.pkgsCross.musl64;
              pbs = pkgs.fetchurl { url = pbsUrl "musl"; hash = hashes.musl; };
              # uv, carried along (its own static build): Python, and packages.
              uvStatic = pkgs.fetchurl {
                url = "https://github.com/astral-sh/uv/releases/download/0.13.9/uv-${arch}-unknown-linux-musl.tar.gz";
                hash = hashes.uv;
              };
              # (Linked as libpython3.so, the name the launcher gives
              # whichever version's library it links in.)
              pyLink = pkgs.runCommand "tessh-python-abi3-link" { nativeBuildInputs = [ pkgs.patchelf ]; } ''
                tar xzf ${pbs}
                mkdir -p $out/lib
                cp python/lib/libpython3.12.so.1.0 $out/lib/libpython3.so
                chmod u+w $out/lib/libpython3.so
                patchelf --set-soname libpython3.so $out/lib/libpython3.so
              '';
              pyo3Config = pkgs.writeText "pyo3-abi3.txt" ''
                implementation=CPython
                version=3.12
                shared=true
                abi3=true
                lib_name=python3
                lib_dir=${pyLink}/lib
                pointer_width=64
                suppress_build_script_link_lines=false
              '';
              # Built in a persistent target directory (/var/tmp/cargo-nix/NAME,
              # exposed to the sandbox by Ben's system flake through
              # extra-sandbox-paths), incremental and without LTO: a change
              # recompiles what it touches, not every dependency from
              # scratch. Without that directory, a throwaway one as before.
              # (The sources go in by content: cargo's freshness check reads
              # mtimes, and the store's are all the same.)
              persistentBuild = { name, deps, cargo, artifact, release ? false }: ''
                runHook preBuild
                workDir=$TMPDIR/cargo-nix
                if [ -w /var/tmp/cargo-nix ]; then
                  workDir=/var/tmp/cargo-nix/${name}${if release then "-lto" else ""}
                fi
                sed -i "s|^directory = .*|directory = \"${deps}\"|" "$NIX_BUILD_TOP/.cargo/config.toml"
                (
                  # (Writable by everyone: each build may run as another nixbld user.)
                  umask 000
                  mkdir -p "$workDir/src" "$workDir/target"
                  touch "$workDir"
                  rsync -rc --no-times --no-perms --chmod=a+rwX --delete --exclude /target ./ "$workDir/src/"
                  # (Build scripts' outputs another build user left: made this
                  # one's. A build script's fs::copy sets the copy's mode,
                  # which only a file's owner may.)
                  find "$workDir/target" -path '*/build/*/out/*' -type f ! -user "$(id -u)" \
                    -exec sh -c 'for f; do cp -p "$f" "$f.own" && mv -f "$f.own" "$f"; done' _ {} + 2>/dev/null || true
                  # (The linker wrapper rejects output paths outside the build dir.)
                  export NIX_ENFORCE_PURITY=0
                  cd "$workDir/src"
                  export CARGO_HOME="$NIX_BUILD_TOP/.cargo" CARGO_TARGET_DIR="$workDir/target" \
                    ${if release then "CARGO_PROFILE_RELEASE_INCREMENTAL=false CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1"
                      else "CARGO_PROFILE_RELEASE_INCREMENTAL=true CARGO_PROFILE_RELEASE_LTO=false"}
                  ${cargo}
                  find "$workDir/target" -mindepth 1 -user "$(id -u)" ! -perm -o+w -exec chmod a+rwX {} + 2>/dev/null || true
                )
                install -Dm755 "$workDir/target/${artifact}" $out/bin/tessh
                runHook postBuild
              '';
              muslTarget = cross.stdenv.hostPlatform.rust.rustcTargetSpec;
              muslDeps = cross.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
              mkTesshMusl = release: cross.rustPlatform.buildRustPackage {
                pname = "tessh-musl${if release then "-lto" else ""}";
                version = (pkgs.lib.importTOML ./tessh/Cargo.toml).package.version;
                inherit src;
                cargoDeps = muslDeps;
                nativeBuildInputs = [ pkgs.rsync ];
                buildPhase = persistentBuild {
                  name = "tessh-musl";
                  inherit release;
                  deps = muslDeps;
                  cargo = "cargo build --release --offline -p tessh --no-default-features --features standalone --target ${muslTarget} -j $NIX_BUILD_CORES";
                  artifact = "${muslTarget}/release/tessh";
                };
                installPhase = "true";
                doCheck = false;
                # (Its loader is a placeholder the launcher fills in, and it
                # finds libpython beside itself: leave both be.)
                dontPatchELF = true;
                env.PYO3_CONFIG_FILE = "${pyo3Config}";
              };
              # The glibc build, for systems with glibc (the launcher picks):
              # linked against glibc 2.17 by zig, so it runs on any
              # distribution of the last decade, with uv's glibc CPython.
              pbsGnu = pkgs.fetchurl { url = pbsUrl "gnu"; hash = hashes.gnu; };
              pyLinkGnu = pkgs.runCommand "tessh-python-abi3-link-gnu" { nativeBuildInputs = [ pkgs.patchelf ]; } ''
                tar xzf ${pbsGnu}
                mkdir -p $out/lib
                cp python/lib/libpython3.12.so.1.0 $out/lib/libpython3.so
                chmod u+w $out/lib/libpython3.so
                patchelf --set-soname libpython3.so $out/lib/libpython3.so
              '';
              pyo3ConfigGnu = pkgs.writeText "pyo3-abi3-gnu.txt" ''
                implementation=CPython
                version=3.12
                shared=true
                abi3=true
                lib_name=python3
                lib_dir=${pyLinkGnu}/lib
                pointer_width=64
                suppress_build_script_link_lines=false
              '';
              gnuDeps = pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
              mkTesshGnu = release: pkgs.rustPlatform.buildRustPackage {
                pname = "tessh-gnu${if release then "-lto" else ""}";
                version = (pkgs.lib.importTOML ./tessh/Cargo.toml).package.version;
                inherit src;
                cargoDeps = gnuDeps;
                nativeBuildInputs = [ pkgs.cargo-zigbuild pkgs.zig_0_15 pkgs.rsync ];
                dontUseZigBuild = true;
                dontUseZigInstall = true;
                dontUseZigCheck = true;
                doCheck = false;
                buildPhase = ''
                  export HOME=$TMPDIR XDG_CACHE_HOME=$TMPDIR/cache ZIG_GLOBAL_CACHE_DIR=$TMPDIR/zig
                '' + persistentBuild {
                  name = "tessh-gnu";
                  inherit release;
                  deps = gnuDeps;
                  cargo = "cargo zigbuild --release --offline -p tessh --no-default-features --features standalone --target ${arch}-unknown-linux-gnu.2.17 -j $NIX_BUILD_CORES";
                  artifact = "${arch}-unknown-linux-gnu/release/tessh";
                };
                installPhase = "true";
                # (The system's loader, libpython beside it: leave both be.)
                dontPatchELF = true;
                dontAutoPatchelf = true;
                env.PYO3_CONFIG_FILE = "${pyo3ConfigGnu}";
              };
              launcher = pkgs.pkgsStatic.rustPlatform.buildRustPackage {
                pname = "tessh-launcher";
                version = (pkgs.lib.importTOML ./tessh/launcher/Cargo.toml).package.version;
                inherit src;
                cargoLock.lockFile = ./Cargo.lock;
                cargoBuildFlags = [ "-p" "tessh-launcher" ];
                doCheck = false;
                # (It's at the front of every file, uncompressed: built for size.)
                env = {
                  CARGO_PROFILE_RELEASE_OPT_LEVEL = "z";
                  CARGO_PROFILE_RELEASE_LTO = "fat";
                  CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "1";
                  CARGO_PROFILE_RELEASE_PANIC = "abort";
                  CARGO_PROFILE_RELEASE_STRIP = "symbols";
                };
              };
              # One payload per C library: that tessh (stripped), the CPython
              # it runs with (stripped by python-build-standalone; its tests,
              # IDLE, Tk, pip and its python program left out), uv, and for musl the loader and
              # libgcc_s. zstd -19 with a 128 MiB window (--long=27): -22 is
              # 1% smaller (39.5 MB, not 39.8) but takes 114 s, not 49, on
              # every rebuild. Unpacked once, at the first run.
              payload = libc: release: pkgs.runCommand "tessh-payload-${libc}${if release then "-lto" else ""}" { nativeBuildInputs = [ pkgs.zstd pkgs.binutils pkgs.patchelf ]; } ''
                mkdir -p root/bin root/lib $out
                cp ${(if libc == "musl" then mkTesshMusl else mkTesshGnu) release}/bin/tessh root/bin/tessh
                chmod u+w root/bin/tessh
                strip --strip-all root/bin/tessh
                patchelf --set-rpath '$ORIGIN/../lib' root/bin/tessh
                tar xzf ${uvStatic} --strip-components=1 -C root/bin uv-${arch}-unknown-linux-musl/uv
                echo ${libc} > root/libc
                tar xzf ${pyBundled libc} -C root
                echo cpython-${pyVersion}-linux-${arch}-${libc} > root/pyname
                chmod -R u+w root
                ( cd root/python/lib
                  rm -rf python3.*/test python3.*/idlelib python3.*/tkinter python3.*/turtledemo python3.*/turtle.py \
                    python3.*/lib-dynload/_tkinter* python3.*/lib-dynload/*_test*.so tcl* tk* itcl* thread* libtcl* libtk* pkgconfig
                  find . -name '*.a' -delete )
                # (tessh is its own python3, and uv installs packages: no
                # second python program, no pip.)
                rm -rf root/python/share root/python/bin root/python/lib/python3.*/site-packages/pip*
                ${if libc == "musl" then ''
                  cp ${cross.stdenv.cc.libc}/lib/libc.so root/lib/ld-musl-${arch}.so.1
                  # (What else it needs: libgcc_s, for unwinding.)
                  for n in $(readelf -d root/bin/tessh | sed -n 's/.*NEEDED.*\[\(.*\)\]/\1/p'); do
                    case $n in libc.so|libpython3.so) ;; *)
                      f=$(find -L ${cross.stdenv.cc.cc.lib}/ ${cross.stdenv.cc.libc}/lib -name "$n" | head -1)
                      [ -n "$f" ] || { echo "no $n"; exit 1; }
                      cp -L "$f" root/lib/ ;;
                    esac
                  done
                  chmod u+w root/lib/*
                  strip --strip-unneeded root/lib/*
                '' else ""}
                chmod -R a+rX root
                chmod 755 root/bin/*
                tar --owner=0 --group=0 --numeric-owner --mtime=@0 --sort=name -C root -cf - . | zstd -19 --long=27 -T0 -q > $out/payload.tar.zst
              '';
              single = libc: release: pkgs.runCommand "tessh-${if release then "release" else "static"}-${libc}-${(pkgs.lib.importTOML ./tessh/Cargo.toml).package.version}" { nativeBuildInputs = [ pkgs.coreutils ]; meta.mainProgram = "tessh"; } ''
                mkdir -p $out/bin
                p=${payload libc release}/payload.tar.zst
                off=$(stat -c%s ${launcher}/bin/tessh)
                cat ${launcher}/bin/tessh $p > $out/bin/tessh
                hash=$(sha256sum $p | cut -c1-16)
                printf '%-63s\n' "TESSH-PAYLOAD $(printf %020d $off) $hash" >> $out/bin/tessh
                chmod 755 $out/bin/tessh
              '';
            in
            rec {
              # musl's runs on any Linux with nothing installed; glibc's takes
              # manylinux wheels. tessh-static is musl's (the login shell's).
              # The fast builds (incremental, no LTO: the dev loop's) and the
              # release ones (fat LTO, one codegen unit: smaller, slower to
              # build; what CI and scripts/tessh-static.sh ship).
              tessh-static-musl = single "musl" false;
              tessh-static-gnu = single "gnu" false;
              tessh-static = tessh-static-musl;
              tessh-release-musl = single "musl" true;
              tessh-release-gnu = single "gnu" true;
            });
          tessera = pkgs.rustPlatform.buildRustPackage {
            pname = "tessera";
            version = (pkgs.lib.importTOML ./Cargo.toml).package.version;
            inherit src;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config pkgs.zig_0_15 pkgs.qt6.wrapQtAppsHook pkgs.qt6.qtbase tesshPython pkgs.makeWrapper ];
            buildInputs = qtFor pkgs ++ [ qtEnv pkgs.libGL tesshPython ];
            dontUseZigBuild = true;
            dontUseZigInstall = true;
            dontUseZigCheck = true;
            doCheck = false;
            env = ghosttyEnv // {
              # cxx-qt wants every Qt module under one prefix.
              QMAKE = "${qtEnv}/bin/qmake";
              PYO3_PYTHON = "${tesshPython}/bin/python3";
            };
            preBuild = ''
              # (Qt's setup hook sets QMAKE to qtbase's alone.)
              export QMAKE=${qtEnv}/bin/qmake
            '' + ghosttyPreBuild;
            # (tessh, built with it: a plain program, not a Qt one.)
            dontWrapQtApps = true;
            postFixup = ''
              wrapQtApp $out/bin/tessera
              ${tesshWrap "$out/bin/tessh"}
            '';
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
        { default = tessera; inherit tessera tessh; tesserad-static = tesseradStatic; } // standalone);

      apps = forAll (pkgs: {
        default = { type = "app"; program = "${self.packages.${pkgs.system}.default}/bin/tessera"; };
        tessh = { type = "app"; program = "${self.packages.${pkgs.system}.tessh}/bin/tessh"; };
      });

      # `nix develop`: Rust, the same Qt, and Zig 0.15 for libghostty-vt.
      devShells = forAll (pkgs:
        let
          qtEnv = qtEnvFor pkgs;
          # The static tesserad, built incrementally like the rest (copied to
          # SSH hosts): Rust for the musl target, and musl's C compiler for
          # the crates with C in them. `cargo build -p tesserad --target
          # <musl> --config target/musl.toml` (written by the shell hook:
          # that rustc, by its whole path; a compiler cache like kache only
          # takes one named rustc).
          musl = pkgs.pkgsStatic;
          muslTarget = musl.stdenv.hostPlatform.rust.rustcTarget;
          muslVar = musl.stdenv.hostPlatform.rust.cargoEnvVarTarget;
          muslUnder = builtins.replaceStrings [ "-" ] [ "_" ] muslTarget;
          muslCc = "${musl.stdenv.cc}/bin/${musl.stdenv.cc.targetPrefix}";
        in
        {
          default = pkgs.mkShell {
            packages = [ pkgs.cargo pkgs.rustc pkgs.clippy pkgs.rustfmt pkgs.pkg-config qtEnv pkgs.libGL pkgs.zig_0_15 pkgs.git pkgs.watchexec pkgs.python315 pkgs.uv ];
            # (tessh's Python, pkgs.python315 as in the package: what PyO3 builds against.)
            PYO3_PYTHON = "${pkgs.python315}/bin/python3";
            QMAKE = "${qtEnv}/bin/qmake";
            TESSERA_MUSL_TARGET = muslTarget;
            "CARGO_TARGET_${muslVar}_LINKER" = "${muslCc}cc";
            "CC_${muslUnder}" = "${muslCc}cc";
            "CXX_${muslUnder}" = "${muslCc}c++";
            "AR_${muslUnder}" = "${muslCc}ar";
            # Link with this shell's compiler (a system clang links against the
            # system glibc and the binary can't load this nixpkgs' Qt).
            "CARGO_TARGET_${pkgs.stdenv.hostPlatform.rust.cargoEnvVarTarget}_LINKER" = "cc";
            shellHook = ''
              # Only in tessera's own checkout: a shell started anywhere else
              # (a terminal opened in ~/Canopy) must not leave files there.
              if [ -f Cargo.toml ] && grep -q '^name = "tessera"' Cargo.toml; then
                ln -sfn ${qtEnv} .qt-env
                mkdir -p target
                printf '[build]\nrustc = "%s"\n' "${musl.buildPackages.rustc}/bin/rustc" > target/musl.toml
              fi
              export NIX_LDFLAGS="''${NIX_LDFLAGS//"-rpath $out/lib"/}"
            '';
          };
        });
    };
}
