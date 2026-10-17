# pico-ledger

Raspberry Pi Pico 2 W (RP2350, ARM Cortex-M33) bare-metal firmware in Rust.
Blinks the on-board CYW43 LED via Embassy async tasks.

## Prerequisites (plain Rust)

```bash
# Cortex-M33 target (hard-float ABI)
rustup target add thumbv8m.main-none-eabihf

# Linker used by rp-hal / embassy for stack-clobber-free placement across RAM banks
cargo install flip-link

# Optional: flashing / debugging over SWD
cargo install probe-rs-tools
```

## Build

```bash
cargo build                    # debug
cargo build --release          # optimized (LTO)
```

The raw firmware is an ELF at:

```
target/thumbv8m.main-none-eabihf/{debug,release}/pico-ledger
```

## Flash & run

Hold the Pico 2's BOOTSEL button while plugging it in, then either:

- **Automated Flashing via Nix (Recommended)**:
  With the Pico 2 connected in BOOTSEL mode:

  ```bash
  # Inside nix develop:
  flash-pico

  # Or directly from any shell:
  nix run .#flash
  ```

  This automatically compiles the release binary, converts it to RP2350 UF2 using `picotool`, verifies/mounts the `RP2350` drive, and copies the UF2 to flash the device.

- **probe-rs** over SWD (needs a debug probe):
  ```bash
  probe-rs download --chip RP235x target/thumbv8m.main-none-eabihf/release/pico-ledger
  probe-rs run --chip RP235x target/thumbv8m.main-none-eabihf/release/pico-ledger
  ```
- **Manual `.uf2` copy**:
  ```bash
  picotool uf2 convert -t elf target/thumbv8m.main-none-eabihf/release/pico-ledger pico-ledger.uf2 --family rp2350-arm-s
  ```
  Drop the produced `.uf2` onto the `RP2350` mount point.

## Nix development environment

A flake (`flake.nix`) provides a reproducible dev shell with the `thumbv8m.main-none-eabihf`
target, `flip-link`, `probe-rs`, and `wokwi-cli` pre-installed:

```bash
nix develop
cargo build
```

The flake also exposes reproducible builds:

```bash
nix build .#default      # firmware → result/bin/pico-ledger.elf
nix build .#wokwi-cli    # the wokwi-cli binary (also included in the dev shell)
```

> `nix build` fetches crates from crates.io, which the default sandbox blocks;
> run it with `--impure` or `NIX_CONFIG="sandbox = false"` (CI does the latter).
> For day-to-day work, prefer `nix develop -c cargo build`.

`wokwi-cli` is not packaged in nixpkgs, so the flake packages the official
static binary from GitHub releases (sha256-pinned per platform).

CI (`.github/workflows/flake.yml`) installs Nix and verifies the flake:
`nix flake check`, builds `.#default` and `.#wokwi-cli`, and smoke-tests the
dev shell (`tools on PATH` + `cargo build`).

> Note: `Cargo.lock` is committed so builds are reproducible across machines
> and through the Nix dev shell.

## License

Dual-licensed under MIT OR Apache-2.0, matching the upstream rp-hal template.
