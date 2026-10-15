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

A reproducible dev shell is provided (`flake.nix`), with the
`thumbv6m-none-eabi` target, `flip-link`, and `probe-rs` pre-installed:

```bash
nix develop
cargo build
```

Or use the flake's shell directly:

```bash
nix develop .#default --command cargo build
```

> Note: `Cargo.lock` is committed so builds are reproducible across machines
> and through the Nix dev shell.

## License

Dual-licensed under MIT OR Apache-2.0, matching the upstream rp-hal template.