{
  description = "pico-ledger — RP2040 (Pi Pico) Rust embedded development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Provides rustc/cargo with the thumbv6m-none-eabi cross target bundled.
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Stable Rust with the Pi Pico (Cortex-M0+) target installed.
        toolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "thumbv6m-none-eabi" ];
        };

        # wokwi-cli is NOT packaged in nixpkgs; it ships as a single static
        # binary per platform. Package the official GitHub release directly.
        # Hashes pinned for v0.28.1 (see https://github.com/wokwi/wokwi-cli/releases).
        wokwi-cli = pkgs.stdenv.mkDerivation rec {
          pname = "wokwi-cli";
          version = "0.28.1";
          src = pkgs.fetchurl rec {
            asset = {
              x86_64-linux = "wokwi-cli-linuxstatic-x64";
              aarch64-linux = "wokwi-cli-linuxstatic-arm64";
              x86_64-darwin = "wokwi-cli-macos-x64";
              aarch64-darwin = "wokwi-cli-macos-arm64";
            }.${system};
            url = "https://github.com/wokwi/wokwi-cli/releases/download/v${version}/${asset}";
            sha256 = {
              x86_64-linux = "869f194ee473257c6112ed58d786376527769020056320b0a49a37d9e7d3bdd7";
              aarch64-linux = "2aa99971f7b1bcadce3f42a0cc0b088c4faabc2d052f7c830008560c3f1dfe4c";
              x86_64-darwin = "e47e93402157f21328af7cd6b23d1077be9fb86523ef2326bcd1670a58b809db";
              aarch64-darwin = "022e89a1539db9a25f9d0df2d9d44bde941b8a677aaf697f01533f0fd2e171e5";
            }.${system};
          };
          dontUnpack = true;
          installPhase = ''
            install -Dm755 "$src" "$out/bin/wokwi-cli"
          '';
          meta = {
            description = "Wokwi Simulation API command line interface";
            homepage = "https://github.com/wokwi/wokwi-cli";
            platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
          };
        };

        # Firmware built through Nix. `.cargo/config.toml` supplies the
        # thumbv6m-none-eabi target + flip-link linker, so a plain
        # `cargo build --release` suffices.
        #
        # NOTE: building fetches crates from crates.io, which is unreachable
        # under the default sandbox, so `nix build` must run with
        # `--impure` / `NIX_CONFIG="sandbox = false"` (see `.github/workflows/flake.yml`).
        # Day-to-day: `nix develop -c cargo build`.
        firmware = pkgs.stdenv.mkDerivation {
          pname = "pico-ledger";
          version = "0.1.0";
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = name: type:
              !(builtins.elem (baseNameOf (toString name))
                [ "target" ".github" ".git" ]);
          };
          nativeBuildInputs = [ toolchain pkgs.flip-link ];
          buildPhase = ''
            cargo build --release --target thumbv6m-none-eabi
          '';
          installPhase = ''
            mkdir -p "$out/bin"
            cp target/thumbv6m-none-eabi/release/pico-ledger "$out/bin/pico-ledger.elf"
          '';
          meta = {
            description = "pico-ledger RP2040 firmware (ELF)";
            platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
          };
        };
      in
      {
        packages.default = firmware;
        packages.wokwi-cli = wokwi-cli;

        # `nix run` → launches wokwi-cli; also `nix shell .#wokwi-cli` or the
        # dev shell below all expose it on PATH.
        apps.default = { type = "app"; program = "${wokwi-cli}/bin/wokwi-cli"; };
        apps.wokwi-cli = { type = "app"; program = "${wokwi-cli}/bin/wokwi-cli"; };

        devShells.default = pkgs.mkShell {
          name = "pico-ledger-dev";
          packages = [
            toolchain
            pkgs.flip-link
            pkgs.probe-rs
            pkgs.llvm-tools
            wokwi-cli
          ];

          # Same target forced in a normal cargo build (not just via .cargo/config).
          RUSTFLAGS = "-D warnings";
          CARGO_BUILD_TARGET = "thumbv6m-none-eabi";

          shellHook = ''
            echo "pico-ledger dev shell ready."
            echo "  target:  $CARGO_BUILD_TARGET"
            echo "  build:   cargo build"
            echo "  flash:   probe-rs run --chip RP2040   (or probe-rs download ...)"
            echo "  sim:     wokwi-cli"
            rustc --version 2>/dev/null && cargo --version 2>/dev/null && wokwi-cli --version 2>/dev/null
          '';
        };
      }
    );
}