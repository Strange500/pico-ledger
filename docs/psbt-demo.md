# TEST ONLY: physical left-button PSBT demo

**Never use this build for funds. Never fund the demo key.** This is a closed
hardware exercise, not a usable hardware wallet or a transaction approval flow.
It is off by default. The normal wallet's seed/password are not used, replaced,
unlocked, or passed into the demo signer.

## Scope and trust boundary

With `--features psbt-demo`, Home displays `TEST ONLY - PSBT`,
`PUBLIC KEY / NO FUNDS`, and `Left: sign demo`. A physical left press (GP3 to GND)
travels through the existing 50 ms debounce/release logic as `Message::Back`,
then Home sends a payload-free `SignerMessage::SignDemo`. The signer task obtains
fresh RP2350 TRNG bytes for secp256k1 blinding and signs the **compiled-in**
`fixtures/demo.psbt` with an ephemeral, separate demo wallet. No USB/Wi-Fi input,
arbitrary PSBT import, user-wallet signing, remote approval, or raw hash signing
is enabled. Right-only and both-button presses do not sign on Home.

The demo seed is deliberately public: 32 zero BIP39 entropy bytes (24 words),
BIP39 passphrase `12345678`, testnet path `m/84'/1'/0'/0/0`. Its temporary encrypted
copy uses a fixed nonce only because both plaintext and key are public and
identical; **never copy that nonce practice to real wallets**. The existing core
policy is unchanged: full previous transaction, matching origins/amounts/scripts,
SIGHASH_ALL and a verified partial ECDSA signature. The demo callback approves
only the compiled fixture's 1 input, 1 output, 100000 sat in, 99000 sat out and
1000 sat fee. It is not a secure review/authorization mechanism.

Repeated presses while Home is busy are ignored; command backpressure permits
retry. Completion updates Home to `Signed! USB hex` or `Failed - retry left`.
A lock event leaves Home; late results do not unlock the device. A demo already
queued can finish after lock, **only with the public demo key**. This is not
fresh-physical-confirmation or lock-cancellation proof for production signing.

## Dummy PSBT

- Binary: [`../fixtures/demo.psbt`](../fixtures/demo.psbt), 272 bytes.
- Base64: [`../fixtures/demo.psbt.base64`](../fixtures/demo.psbt.base64).
- SHA-256: `ee3506269825acfd06f373e1fa5d842156c224d046e9f4b99635e234b14f02aa`.
- Unsigned txid: `32329399beae4a8f6e8e5052ffccce3fc45090ac800fa58b13e8de9166a6f0dd`.
- One fake funding output, spent to the same public testnet P2WPKH key, no change.
- The funding transaction is fabricated (invalid coinbase-like input); this PSBT
  has no spendable chain UTXO and cannot produce a broadcastable transaction.
- Signing inserts one partial signature; it does not finalize/extract/broadcast.

To regenerate exactly the binary fixture (host only, no feature required):

```sh
cargo run --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu --example demo_fixture
git diff --exit-code -- fixtures/demo.psbt
```

The Base64 file is a convenience encoding of the same bytes, not another fixture.
This firmware signs its built-in bytes; do **not** send the Base64 file to USB.

## Host checks (no Pico needed)

```sh
cargo fmt --all -- --check
cargo test --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu
cargo test --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu --features psbt-demo
cargo clippy --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
cargo clippy --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu --all-targets --all-features -- -D warnings
cargo clippy --locked --all-features -- --deny=warnings
cargo build --locked -p pico-ledger --features psbt-demo
cargo build --locked -p pico-ledger --release --features psbt-demo
```

Embedded commands need the target, flip-link and ARM-capable clang (see README /
Nix dev shell). Host tests verify the actual ECDSA signature against BIP143,
transaction/metadata preservation, signed-PSBT reimport, consumed context entropy,
fixture structure, default-disabled signing, duplicate presses, backpressure,
completion/error retry and lock/late-result handling. They do not exercise GPIO,
the display task, Embassy scheduling, actual TRNG or MCU memory limits.

For an actual **host-only** signing/capture-verifier exercise on Unix:

```sh
cargo run --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu --features psbt-demo --example demo_psbt -- sign > signed.hex
cargo run --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu --features psbt-demo --example demo_psbt -- verify signed.hex
```

The host utility draws blinding bytes from `/dev/urandom` and compares a capture
against the exact deterministic signed fixture. Success reports **379 bytes**.
Modified/truncated captures fail with a nonzero exit code. It is not a general
PSBT verifier or a hardware attestation.

## Hardware procedure (not hardware-tested here)

1. Use a **Pico 2 W / RP2350**, not an RP2040. Wire OLED SDA=GP4, SCL=GP5,
   VCC=3V3, common GND; left button=GP3 to GND, right button=GP2 to GND.
2. Build explicitly with the feature. Do not use `flash-watch` / `flash-pico`
   for this demo: those scripts rebuild the default, non-demo firmware.
   With an SWD probe, `cargo run --locked --release --features psbt-demo`
   builds/flashes via the configured probe-rs runner. Without a probe, build
   release as above, then use the README's UF2 conversion/BOOTSEL procedure on
   that exact ELF **without rebuilding**.
3. Open `tio /dev/ttyACM0` (adjust device) before pressing left; keep logs running.
   Complete the normal disposable wallet setup and seed backup flow to Home
   (right=next, left=back, both=select). Do not enter a valuable recovery seed or
   reuse a real password; the demo itself always uses its separate public seed.
4. Confirm the Home `TEST ONLY - PSBT` / `PUBLIC KEY / NO FUNDS` labels. Press and
   release **only left** once. Expected: `Signing demo...`, then `Signed! USB hex`.
   The synchronous cryptography runs in the signer task; UI may briefly pause.
5. Capture the complete USB line `TEST ONLY signed PSBT hex: ...` (also emitted
   via RTT). Save **only the hex payload** to `signed.hex`, without log prefix.
   Run the host `demo_psbt verify signed.hex` command above. Expected: exact
   379-byte signed fixture match. USB logging is best effort with a 1024-byte
   buffer: attach first; missing/truncated output requires another left press.
   OLED success means signing completed, not guaranteed delivery to the host.
6. Hold left: there must be no repeated signing while held. Right-only / both
   must not sign. Release and press left again: a second complete output should
   verify identically. Let the device lock and check that left does not sign
   while locked; an already queued public demo may finish. Reboot: ordinary
   setup behavior is unchanged, and real user-wallet signing remains absent.
7. Rebuild/flash **without** `psbt-demo`: Home must show `Signing disabled` and
   left must not emit a signed PSBT.

Hardware checks above are a manual acceptance procedure, **not claimed results**.
No board/probe/serial device was available during implementation. Cross-linked
ELFs do not prove runtime heap/stack fit, physical confirmation timing, display
rendering or side-channel resistance. Production support still requires full
on-device review of every output/amount/fee/metadata, a fresh physical approval
bound to that exact request, robust cancellation/lock semantics and a secure
transport; see [the original policy/blockers](psbt.md).
