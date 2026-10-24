# pico-ledger

Bare-metal Bitcoin hardware wallet firmware for the **Raspberry Pi Pico 2 W** (RP2350, ARM Cortex-M33 + Infineon CYW43439) written in asynchronous Rust using the **Embassy** framework.

---

## Overview

- **BIP-39 Mnemonic Generation**: Generates cryptographically secure recovery seed phrases using the RP2350 hardware True Random Number Generator (TRNG) and SHA-256 checksums.
- **SSD1306 128x64 OLED Display**: Async I2C driver powered by `buoyant` and `embedded-graphics` displaying wallet status, seed phrases, and interactive prompts.
- **Onboard LED Control**: On the Pico 2 W, the onboard LED is physically connected to the Infineon CYW43439 co-processor (GPIO 0). Driven via Embassy's `cyw43` and `cyw43-pio` drivers.
- **Interactive Pushbutton**: Pushbutton on `GP2` (active-low with internal pull-up) triggers seed phrase generation, screen navigation, and LED control.
- **Live USB Serial Logging**: Exposes a USB CDC-ACM virtual COM port (`/dev/ttyACM0`) through `embassy-usb-logger` for real-time diagnostics.
- **SWD RTT Logging**: Simultaneously streams logs over SWD via `defmt-rtt` when using a debug probe like `probe-rs`.
- **Desktop UI Simulator**: Standalone desktop runner using `embedded-graphics-simulator` to preview and test display views on the host without flashing hardware.
- **Reproducible Nix Environment**: Complete Nix flake providing `picotool`, `tio`, `probe-rs`, `flip-link`, Wokwi simulation, and cross-compilation toolchains.

---

## Hardware Pinout

| Pin / Interface | Target | Function | Notes |
|---|---|---|---|
| **USB** | Host PC | Power, Firmware Flashing, CDC-ACM Serial Logs | Connects via micro-USB; appears as `/dev/ttyACM0` |
| **GP2** | Pushbutton | User Input | Active-low; internal pull-up enabled; other button leg to GND |
| **GP4** | SSD1306 OLED | I2C0 SDA (Data) | Pulled high by display module; connects to OLED SDA |
| **GP5** | SSD1306 OLED | I2C0 SCL (Clock) | 400 kHz fast-mode I2C; connects to OLED SCL |
| **3V3 (Pin 36)** | SSD1306 OLED | Display Power (VCC) | 3.3V power supply rail |
| **GND (Pin 38)** | SSD1306 OLED / Button | Ground Reference | Common ground |
| **CYW43 GPIO 0** | Onboard LED | System Status Indicator | Driven via CYW43 PIO-SPI co-processor interface |
| **GP23** | CYW43 WL_REG_ON | Wireless Power Enable | Internal co-processor line |
| **GP25** | CYW43 SPI CS | Wireless Chip Select | Internal co-processor line |
| **GP24 / GP29** | CYW43 DATA / CLK | Half-duplex PIO SPI | Internal co-processor line |
| **SWD Header** | Debug Probe | Hardware Debugging & RTT Logs | `SWCLK`, `SWDIO`, `GND` |

---

## To-Do List & Roadmap

### Reliability & Code Audit ([#12](https://github.com/Strange500/pico-ledger/issues/12))
- [ ] **OLED I2C Fault Tolerance**: Handle I2C flush transmission errors gracefully without panicking the MCU (`src/tasks/display.rs`).
- [ ] **Cryptographic Self-Test Enforcement**: Halt or enter safe panic mode on boot if BIP-39 test vector validation fails (`src/main.rs`).
- [ ] **Display Event Wiring**: Connect `WORD_SIGNAL` and UI state updates so generated seed phrases render on the OLED display (`src/tasks/display.rs`).
- [ ] **Button & LED Interaction**: Wire `BUTTON_SIGNAL` to LED control to allow toggling or deactivating status blinking.
- [ ] **Full BIP-39 Checksum Pipeline**: Replace single-word modulo sampling with 128/256-bit entropy extraction and SHA-256 checksum derivation (`src/tasks/rng.rs`).
- [ ] **Workspace & Simulator Fix**: Isolate `crates/ui` in cargo workspace configuration so `cargo check-ui` and desktop simulation run reliably.
- [ ] **Zero-Allocation BIP-39 & Memory Security**: Remove 128 KB heap reservation by converting BIP-39 to zero-alloc fixed arrays, and zeroize secret entropy in RAM.
- [ ] **UI Deduplication**: Share common screen views between bare-metal firmware and desktop simulator.
- [ ] **Display Frame Rate Limiter**: Add frame interval pacing to prevent 100% CPU utilization during UI animations.

### Hardware Wallet Milestones
- [ ] **Step 3 — Entropy + BIP-39 Mnemonic Generation** ([#5](https://github.com/Strange500/pico-ledger/issues/5)): Secure TRNG sampling, seed generation, and 12/24-word pagination.
- [ ] **Step 4 — BIP-32 Hierarchical Key Derivation** ([#6](https://github.com/Strange500/pico-ledger/issues/6)): Master key derivation and child key generation (secp256k1).
- [ ] **Step 5 — Bech32 / P2WPKH Address Derivation** ([#7](https://github.com/Strange500/pico-ledger/issues/7)): Native SegWit address calculation and QR / on-screen validation.
- [ ] **Step 6 — Transaction Signing** ([#8](https://github.com/Strange500/pico-ledger/issues/8)): Single-input / single-output Bitcoin transaction signing.
- [ ] **Step 7 — Milestone: Offline Signer (PSBT)** ([#9](https://github.com/Strange500/pico-ledger/issues/9)): Parse Partially Signed Bitcoin Transactions (PSBT) and require manual button confirmation.
- [ ] **Step 8 — Host Communication Protocol** ([#10](https://github.com/Strange500/pico-ledger/issues/10)): Framing and transport protocol over USB serial (CDC-ACM).

---

## Quick Start (Nix - Recommended)

The Nix flake provides all compilers, flash utilities, and serial monitors without needing manual tool installs.

### 1. Flash and Watch Live Logs in One Step

1. Connect your Pico 2 W while holding the **BOOTSEL** button.
2. Run:

```bash
# Inside the dev shell:
nix develop
flash-watch

# Or directly from any shell without entering nix develop:
nix run .#flash-watch      # or simply: nix run
```

This will automatically:
1. Compile the release firmware (`cargo build --release`).
2. Convert the ELF into an RP2350-compliant `.uf2` binary using `picotool`.
3. Locate, mount the `RP2350` drive, copy the UF2, and reboot the board.
4. Wait for the board to re-enumerate as a USB serial device (`/dev/ttyACM*`).
5. Launch `tio` and stream live logs directly to your terminal.

*(To exit the live log monitor, press `Ctrl+T` then `q`)*.

### 2. Available Commands

| Command (Nix shell) | Standalone (`nix run`) | Description |
|---|---|---|
| `flash-watch` | `nix run .#flash-watch` | Flashes firmware and immediately attaches live USB logs |
| `flash-pico` | `nix run .#flash` | Builds, converts, and flashes firmware to Pico 2 W |
| `watch-logs` | `nix run .#logs` | Connects `tio` to `/dev/ttyACM*` to view live logs without flashing |
| `wokwi-cli` | `nix run .#wokwi-cli` | Runs simulation via Wokwi CLI |

---

## Testing & Host Simulator

### Run Unit Tests
Because the default target is bare-metal Cortex-M, execute unit tests on the host target (`x86_64`):

```bash
cargo test-host
# Or directly:
cargo test --package bip39 --target x86_64-unknown-linux-gnu
```

### Run Desktop UI Simulator
Preview the OLED layout and animations on your host machine:

```bash
cargo run --package ui --target x86_64-unknown-linux-gnu
```

---

## Manual Workflow (Standard Rust)

If you are not using Nix, install the standard Rust embedded prerequisites:

```bash
# Add Cortex-M33 target (hard-float ABI)
rustup target add thumbv8m.main-none-eabihf

# Linker used by rp-hal / embassy for stack-clobber-free placement across RAM banks
cargo install flip-link

# Flashing / debugging tools
cargo install probe-rs-tools picotool
```

### Build Firmware

```bash
# Debug build
cargo build

# Optimized release build
cargo build --release
```

The resulting ELF binary will be located at:
```
target/thumbv8m.main-none-eabihf/{debug,release}/pico-ledger
```

### Convert to UF2 & Flash

1. Convert the compiled ELF to RP2350 UF2:
   ```bash
   picotool uf2 convert -t elf \
     target/thumbv8m.main-none-eabihf/release/pico-ledger \
     target/thumbv8m.main-none-eabihf/release/pico-ledger.uf2 \
     --family rp2350-arm-s
   ```
2. Hold the **BOOTSEL** button on the Pico 2 W while plugging it into USB.
3. Copy the UF2 to the mounted volume:
   ```bash
   cp target/thumbv8m.main-none-eabihf/release/pico-ledger.uf2 /run/media/$USER/RP2350/
   sync
   ```

---

## Viewing Live Logs

Once the Pico 2 W boots, it initializes the USB CDC-ACM stack and appears as a serial port (e.g. `/dev/ttyACM0`).

### Using `tio` (Recommended)
```bash
tio /dev/ttyACM0
```
*(Exit with `Ctrl+T` then `q`)*.

### Using `picocom`
```bash
picocom -q /dev/ttyACM0
```
*(Exit with `Ctrl+A` then `Ctrl+X`)*.

---

## Optional: SWD Debugging with `probe-rs`

If you have a debug probe (e.g. Raspberry Pi Debug Probe, CMSIS-DAP) wired to the Pico's SWD pins:

```bash
# Flash, run, and view RTT defmt logs in one command:
cargo run --release

# Or attach to an already running target:
probe-rs attach --chip RP235x target/thumbv8m.main-none-eabihf/release/pico-ledger
```

---

## Architecture & Code Structure

```
pico-ledger/
├── .cargo/
│   └── config.toml           # Cross-compilation target, runner, and linker configurations
├── crates/
│   ├── bip39/                # BIP-39 mnemonic implementation, wordlist, and test vectors
│   └── ui/                   # Host UI simulator (embedded-graphics-simulator)
├── firmware/
│   └── nvram_rp2040.bin      # CYW43439 NVRAM parameter binary blob
├── src/
│   ├── main.rs               # Async entry point and Embassy task spawner
│   ├── signals.rs            # Inter-task signals (BUTTON_SIGNAL, WORD_SIGNAL)
│   ├── tasks/
│   │   ├── button.rs         # GP2 debounced button detection task
│   │   ├── display.rs        # SSD1306 OLED display async rendering task
│   │   ├── rng.rs            # Hardware TRNG random word generation task
│   │   ├── usb.rs            # USB CDC-ACM serial logger task
│   │   └── wifi.rs           # CYW43 co-processor runner (drives onboard LED)
│   └── ui/
│       └── state.rs          # UI view hierarchies and screen state definitions
├── build.rs                  # Linker script search path setup
├── memory.x                  # RP2350 Flash and RAM layout with boot headers
├── flake.nix                 # Nix development shell and automated flash scripts
├── diagram.json              # Wokwi simulation hardware wiring
└── wokwi.toml                # Wokwi project configuration
```

---

## Troubleshooting

### Linux Serial Port Permissions (`/dev/ttyACM0`)
If you receive a permission denied error when opening `/dev/ttyACM0`, add your user to the appropriate dialout group:

```bash
# For Ubuntu / Debian:
sudo usermod -aG dialout $USER

# For Arch / Fedora:
sudo usermod -aG uucp $USER
```
Log out and log back in for group changes to take effect.

### RP2350 Drive Not Found
Ensure you hold down the white **BOOTSEL** button while connecting the USB cable to the PC. The device will present itself as a mass storage volume named `RP2350`.


