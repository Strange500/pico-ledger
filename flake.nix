{
  description = "pico-ledger — RP2350 (Pi Pico 2) Rust embedded development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Provides rustc/cargo with the thumbv8m.main-none-eabihf cross target bundled.
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
        };

        # Stable Rust with the Pi Pico 2 (Cortex-M33) target installed.
        toolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = ["thumbv8m.main-none-eabihf"];
        };

        # wokwi-cli is NOT packaged in nixpkgs; it ships as a single static
        # binary per platform. Package the official GitHub release directly.
        # Hashes pinned for v0.28.1 (see https://github.com/wokwi/wokwi-cli/releases).
        wokwi-cli = pkgs.stdenv.mkDerivation rec {
          pname = "wokwi-cli";
          version = "0.28.1";
          src = let
            asset =
              {
                x86_64-linux = "wokwi-cli-linuxstatic-x64";
                aarch64-linux = "wokwi-cli-linuxstatic-arm64";
                x86_64-darwin = "wokwi-cli-macos-x64";
                aarch64-darwin = "wokwi-cli-macos-arm64";
              }.${
                system
              };
          in
            pkgs.fetchurl {
              url = "https://github.com/wokwi/wokwi-cli/releases/download/v${version}/${asset}";
              sha256 =
                {
                  x86_64-linux = "869f194ee473257c6112ed58d786376527769020056320b0a49a37d9e7d3bdd7";
                  aarch64-linux = "2aa99971f7b1bcadce3f42a0cc0b088c4faabc2d052f7c830008560c3f1dfe4c";
                  x86_64-darwin = "e47e93402157f21328af7cd6b23d1077be9fb86523ef2326bcd1670a58b809db";
                  aarch64-darwin = "022e89a1539db9a25f9d0df2d9d44bde941b8a677aaf697f01533f0fd2e171e5";
                }.${
                  system
                };
            };
          dontUnpack = true;
          dontStrip = true;
          installPhase = ''
            install -Dm755 "$src" "$out/bin/wokwi-cli"
          '';
          meta = {
            description = "Wokwi Simulation API command line interface";
            homepage = "https://github.com/wokwi/wokwi-cli";
            platforms = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
          };
        };

        # Firmware built through Nix. `.cargo/config.toml` supplies the
        # thumbv8m.main-none-eabihf target + flip-link linker, so a plain
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
                ["target" ".github" ".git"]);
          };
          nativeBuildInputs = [toolchain pkgs.llvmPackages.clang-unwrapped pkgs.flip-link];
          buildPhase = ''
            # Nix builds run with an unwritable HOME; give cargo a writable home.
            export CARGO_HOME="$(pwd)/.cargo-home"
            cargo build -p ledger-firmware --release --target thumbv8m.main-none-eabihf
          '';
          installPhase = ''
            mkdir -p "$out/bin"
            cp target/thumbv8m.main-none-eabihf/release/ledger-firmware "$out/bin/ledger-firmware.elf"
          '';
          meta = {
            description = "pico-ledger RP2350 firmware (ELF)";
            platforms = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
          };
        };
        flash-script = pkgs.writeShellScriptBin "flash-pico" ''
          set -euo pipefail
          export PATH="${pkgs.lib.makeBinPath [toolchain pkgs.llvmPackages.clang-unwrapped pkgs.flip-link pkgs.picotool pkgs.util-linux pkgs.coreutils]}:$PATH"

          echo "== 1. Building release firmware =="
          cargo build -p ledger-firmware --release --target thumbv8m.main-none-eabihf

          echo "== 2. Converting ELF to RP2350 UF2 =="
          ELF_PATH="target/thumbv8m.main-none-eabihf/release/ledger-firmware"
          UF2_PATH="target/thumbv8m.main-none-eabihf/release/ledger-firmware.uf2"
          picotool uf2 convert -t elf "$ELF_PATH" "$UF2_PATH" --family rp2350-arm-s

          echo "== 3. Locating RP2350 device =="
          MOUNT_POINT=$(findmnt -rn -o TARGET -S LABEL=RP2350 2>/dev/null || true)
          if [ -z "$MOUNT_POINT" ]; then
            if [ ! -e "/dev/disk/by-label/RP2350" ]; then
              echo "Error: RP2350 device not found in BOOTSEL mode." >&2
              echo "Please connect your Raspberry Pi Pico 2 while holding down the BOOTSEL button." >&2
              exit 1
            fi
            echo "Mounting RP2350 volume..."
            UDISKSCTL=$(command -v udisksctl || echo "${pkgs.udisks2}/bin/udisksctl")
            "$UDISKSCTL" mount -b /dev/disk/by-label/RP2350 >/dev/null
            MOUNT_POINT=$(findmnt -rn -o TARGET -S LABEL=RP2350 2>/dev/null || true)
          fi

          if [ -z "$MOUNT_POINT" ] || [ ! -d "$MOUNT_POINT" ]; then
            echo "Error: Failed to determine mount point for RP2350." >&2
            exit 1
          fi

          echo "Mounted at: $MOUNT_POINT"
          echo "== 4. Copying UF2 firmware to Pico 2 =="
          cp "$UF2_PATH" "$MOUNT_POINT/"
          sync

          echo "Success! The Pico 2 is now flashing and rebooting."
        '';

        watch-script = pkgs.writeShellScriptBin "watch-logs" ''
          set -euo pipefail
          export PATH="${pkgs.lib.makeBinPath [pkgs.tio pkgs.coreutils]}:$PATH"

          echo "Waiting for Pico USB serial port (/dev/ttyACM*)..."
          for i in $(seq 1 50); do
            DEVICE=$(ls /dev/ttyACM* 2>/dev/null | head -n 1 || true)
            if [ -n "$DEVICE" ] && [ -e "$DEVICE" ]; then
              break
            fi
            sleep 0.1
          done

          if [ -z "$DEVICE" ] || [ ! -e "$DEVICE" ]; then
            echo "Error: Timed out waiting for /dev/ttyACM*." >&2
            echo "Make sure your Raspberry Pi Pico is connected and powered." >&2
            exit 1
          fi

          echo "== Connected to $DEVICE (press Ctrl+T then q to exit) =="
          exec tio "$DEVICE"
        '';

        flash-watch-script = pkgs.writeShellScriptBin "flash-watch" ''
          set -euo pipefail
          export PATH="${pkgs.lib.makeBinPath [flash-script watch-script]}:$PATH"

          flash-pico
          echo ""
          watch-logs
        '';
      in {
        packages.default = firmware;
        packages.wokwi-cli = wokwi-cli;
        packages.flash-pico = flash-script;
        packages.flash-watch = flash-watch-script;
        packages.watch-logs = watch-script;

        apps.default = {
          type = "app";
          program = "${flash-watch-script}/bin/flash-watch";
        };
        apps.flash = {
          type = "app";
          program = "${flash-script}/bin/flash-pico";
        };
        apps.flash-pico = {
          type = "app";
          program = "${flash-script}/bin/flash-pico";
        };
        apps.flash-watch = {
          type = "app";
          program = "${flash-watch-script}/bin/flash-watch";
        };
        apps.watch = {
          type = "app";
          program = "${watch-script}/bin/watch-logs";
        };
        apps.watch-logs = {
          type = "app";
          program = "${watch-script}/bin/watch-logs";
        };
        apps.logs = {
          type = "app";
          program = "${watch-script}/bin/watch-logs";
        };
        apps.wokwi-cli = {
          type = "app";
          program = "${wokwi-cli}/bin/wokwi-cli";
        };

        devShells.default = pkgs.mkShell {
          name = "pico-ledger-dev";
          packages = [
            toolchain
            pkgs.llvmPackages.clang-unwrapped
            pkgs.flip-link
            pkgs.probe-rs-tools
            pkgs.picotool
            pkgs.tio
            flash-script
            flash-watch-script
            watch-script
            wokwi-cli
            pkgs.SDL2
            pkgs.SDL2_sound
            pkgs.SDL2.dev
          ];

          # The `.cargo/config.toml` supplies the linker + memory layout link-args;
          # do NOT set RUSTFLAGS here — it overrides them and breaks flip-link.
          CARGO_BUILD_TARGET = "thumbv8m.main-none-eabihf";

          shellHook = ''
            echo "pico-ledger dev shell ready."
            echo "  target:      $CARGO_BUILD_TARGET"
            echo "  build:       cargo build"
            echo "  flash & log: flash-watch                  (flashes and attaches live USB logs)"
            echo "  flash:       flash-pico                   (builds, converts, mounts, flashes Pico 2)"
            echo "  logs:        watch-logs                   (streams live USB serial logs via tio)"
            echo "  probe:       probe-rs run --chip RP235x   (over SWD probe)"
            echo "  sim:         wokwi-cli"
            rustc --version 2>/dev/null && cargo --version 2>/dev/null
          '';
        };
      }
    );
}
