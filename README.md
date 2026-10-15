# pico-ledger

Raspberry Pi Pico (RP2040, ARM Cortex-M0+) bare-metal firmware in Rust.
Generated from the [rp2040-project-template](https://github.com/rp-rs/rp2040-project-template).
Stock template blinks the on-board LED on GP25 via `defmt` logging over RTT.

## Prerequisites (plain Rust)

```bash
# Cortex-M0+ target
rustup target add thumbv6m-none-eabi

# Linker used by rp-hal for stack-clobber-free placement across RAM banks
cargo install flip-link

# Optional: flashing / debugging over SWD
cargo install probe-rs --features cli
```

## Build

```bash
cargo build                    # debug
cargo build --release          # optimized (LTO)
```

The raw firmware is an ELF at:

```
target/thumbv6m-none-eabi/{debug,release}/pico-ledger
```

## Flash & run

Hold the Pico's BOOTSEL button while plugging it in, then either:

- **probe-rs** over SWD (needs a debug probe):
  ```bash
  probe-rs download --chip RP2040 target/thumbv6m-none-eabi/release/pico-ledger
  probe-rs run --chip RP2040 target/thumbv6m-none-eabi/release/pico-ledger
  ```
- **`.uf2`** (drag & drop onto the mass-storage drive):
  ```bash
  cargo install elf2uf2-rs
  elf2uf2-rs target/thumbv6m-none-eabi/release/pico-ledger
  ```
  Drop the produced `.uf2` onto the `RPI-RP2` mount point.

## Nix development environment

A flake (`flake.nix`) provides a reproducible dev shell with the `thumbv6m-none-eabi`
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