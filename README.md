# pico-ledger

Bare-metal firmware for the **Raspberry Pi Pico 2 W** (RP2350, ARM Cortex-M33 + Infineon CYW43439) written in asynchronous Rust using the **Embassy** framework.

---

## Overview

- **Onboard LED Control**: On the Pico 2 W, the onboard LED is physically connected to the Infineon CYW43439 co-processor (GPIO 0). This project uses Embassy's `cyw43` and `cyw43-pio` drivers to initialize the wireless chip and drive the LED.
- **Default State & Blinking**: The LED turns **ON by default** upon boot, and begins an asynchronous 500 ms blink cycle.
- **Interactive Button Deactivation**: Pressing an external button connected to `GP2` (active-low with internal pull-up) immediately deactivates the LED (turns it OFF) using Embassy's asynchronous `select` pattern. Pressing it again reactivates the LED and resumes blinking.
- **Live USB Serial Logging**: Exposes a USB CDC-ACM virtual COM port (`/dev/ttyACM0`) through `embassy-usb-logger`. You can view live formatted log messages directly over your standard USB cable in any terminal.
- **SWD RTT Logging**: Simultaneously streams logs over SWD via `defmt-rtt` when using a debug probe like `probe-rs`.
- **Reproducible Nix Environment**: Complete Nix flake providing `picotool`, `tio`, `probe-rs`, `flip-link`, and cross-compilation toolchains with one-step flash and live log watching commands.

---

## Hardware Pinout

| Pin / Interface | Target | Function | Notes |
|---|---|---|---|
| **USB** | Host PC | Power, Firmware Flashing, CDC-ACM Serial Logs | Connects via micro-USB; appears as `/dev/ttyACM0` |
| **GP2** | Pushbutton | Button Input | Active-low; internal pull-up enabled; other button leg to GND |
| **CYW43 GPIO 0** | Onboard LED | System Status Indicator | Driven via CYW43 PIO-SPI co-processor interface |
| **GP23** | CYW43 WL_REG_ON | Wireless Power Enable | Internal |
| **GP25** | CYW43 SPI CS | Wireless Chip Select | Internal (repurposed from LED on non-W Pico) |
| **GP24 / GP29** | CYW43 DATA / CLK | Half-duplex PIO SPI | Internal |
| **SWD Header** | Debug Probe | Optional Hardware Debugging & RTT Logs | `SWCLK`, `SWDIO`, `GND` |

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

### 2. Individual Commands

| Command (Nix shell) | Standalone (`nix run`) | Description |
|---|---|---|
| `flash-watch` | `nix run .#flash-watch` | Flashes firmware and immediately attaches live USB logs |
| `flash-pico` | `nix run .#flash` | Builds, converts, and flashes firmware to Pico 2 W |
| `watch-logs` | `nix run .#logs` | Connects `tio` to `/dev/ttyACM*` to view live logs without flashing |

---

## Manual Workflow (Standard Rust)

If you are not using Nix, install the standard Rust embedded prerequisites:

```bash
# Add Cortex-M33 target (hard-float ABI)
rustup target add thumbv8m.main-none-eabihf

# Linker used by rp-hal / embassy for stack-clobber-free placement across RAM banks
cargo install flip-link

# Optional: flashing / debugging over SWD
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

### Using `cat`
```bash
cat /dev/ttyACM0
```

### Example Log Output
```text
[INFO] RP2350 (Pico 2 W) initialized - USB Serial logging active
[INFO] Pico 2 W ready: LED ON by default. Press button to deactivate/reactivate.
[INFO] Button (GP2) clicked: toggling LED state
[INFO] LED deactivated (OFF)
[INFO] Button (GP2) clicked: toggling LED state
[INFO] LED reactivated (ON)
```

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
│   └── config.toml           # Linker flags (flip-link, link.x, defmt.x) & target config
├── firmware/
│   └── nvram_rp2040.bin      # CYW43439 NVRAM parameter binary blob
├── src/
│   └── main.rs               # Async entry point, tasks, and event loops
├── build.rs                  # Linker search path script
├── memory.x                  # RP2350 Flash and RAM layout with boot headers
├── Cargo.toml                # Dependencies (embassy-rp, cyw43, embassy-usb, etc.)
├── flake.nix                 # Nix devShell, reproducible build, and flash scripts
└── README.md                 # Project documentation
```

### Asynchronous Design Highlights
- **No Blocking Loops**: Every delay and wait uses non-blocking async timers (`Timer::after(...).await`) and interrupt futures (`button.wait_for_falling_edge().await`).
- **Concurrent Tasks**:
  - `logger_task`: Manages USB CDC-ACM buffers and packet transfers.
  - `cyw43_task`: Drives the SPI bus communication with the wireless co-processor.
  - `button_task`: Performs interrupt-driven edge detection and contact debounce on `GP2`.
  - `main`: Manages system initialization, default LED power, and the responsive `select` blink/deactivate loop.
- **Race Prevention via `select`**: Rather than polling an atomic flag after delays, the blink loop races `Timer::after(delay)` against `BUTTON_SIGNAL.wait()`. Clicking the button deactivates the LED immediately.

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

---

## License

Dual-licensed under either:
- **[Apache License, Version 2.0](file:///home/strange/Documents/pico-ledger/LICENSE-APACHE-2.0)**
- **[MIT License](file:///home/strange/Documents/pico-ledger/LICENSE-MIT)**

at your option.
