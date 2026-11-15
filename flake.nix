{
  description = "Tessera: an iTerm2-style terminal (Rust + cxx-qt + libghostty-vt)";

  # Pinned to the same nixpkgs revision as /etc/nixos, so the app uses the
  # Qt build the desktop's style plugins were made for.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      qtFor = pkgs: with pkgs.qt6; [ qtbase qtdeclarative qtsvg qtwayland ];
      qtEnvFor = pkgs: pkgs.qt6.env "qt-tessera-${pkgs.qt6.qtbase.version}" ((builtins.tail (qtFor pkgs)) ++ [ pkgs.libGL ]);
    in
    {
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
