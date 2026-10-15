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
      in
      {
        devShells.default = pkgs.mkShell {
          name = "pico-ledger-dev";
          packages = [
            toolchain
            # Surface `cargo build` works directly: cargo + flip-link linker.
            pkgs.flip-link
            # Flashing / debugging the board over SWD (cargo embed, probe-rs, ...).
            pkgs.probe-rs
            pkgs.llvm-tools
          ];

          # Same target forced in a normal cargo build (not just via .cargo/config).
          RUSTFLAGS = "-D warnings";
          CARGO_BUILD_TARGET = "thumbv6m-none-eabi";

          shellHook = ''
            echo "pico-ledger dev shell ready."
            echo "  target:  $CARGO_BUILD_TARGET"
            echo "  build:   cargo build"
            echo "  flash:   probe-rs download --chip RP2040   (after: probe-rs run --chip RP2040)"
            rustc --version 2>/dev/null && cargo --version 2>/dev/null
          '';
        };
      }
    );
}