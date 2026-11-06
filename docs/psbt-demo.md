# TEST ONLY: physical left-button PSBT demo

**Never use this build for funds. Never fund the demo key.** This is a closed
hardware review/signing exercise, not a usable production hardware wallet.
It is off by default. The normal wallet's seed/password are not used, replaced,
unlocked, or passed into the demo signer.

## Scope and trust boundary

With `--features psbt-demo`, Home displays `TEST ONLY - PSBT`,
`PUBLIC KEY / NO FUNDS`, and `Left: review demo`. Physical left (GP3 to GND)
sends `Message::Back` through the existing 50 ms debounce/release logic. Home
requests **validation only** with payload-free `SignerMessage::PrepareDemo`.
The signer uses fresh RP2350 TRNG blinding, validates the compiled-in immutable
`fixtures/demo.psbt`, retains its complete review, and sends a display-only clone
with a monotonically increasing session token. No signatures are made here.

The review pages show network/public-key warning, full unsigned txid, version,
lock time, input/output counts, every input outpoint/address/value/sequence,
every output address/value/verified-change classification, total input/output,
fee in integer satoshis, and SIGHASH_ALL. Identifiers are split into numbered
parts, never ellipsized: at most 20 columns and 3 value lines per 128x64 page.
Left goes back (clamped at the first page), right advances. Both buttons cancel
immediately on any review page. The final decision is reachable only by stepping
through all pages and defaults to **CANCEL**. Left chooses Cancel, right chooses
Sign, **then a separate both-button press confirms the displayed choice**.
There is no wraparound from the first page to confirmation. Review/decision
buttons are ignored until that frame has successfully flushed to the OLED;
selecting Sign also requires a successful flush before confirmation. Physical
button events carry the display epoch sampled when the button task observes the
edge; queued events from older frames are rejected (including pre-render and
epoch-exhaustion states). This is a fail-closed software guard, not a measured
interrupt/scheduler latency guarantee; unusually fast presses may be discarded.

Only matching `ConfirmDemo { token }` consumes the signer-owned pending review.
Signing revalidates the **same immutable compiled-in bytes**, including PSBT
metadata, and its callback requires equality with the complete retained review.
Neither UI nor transport can substitute a PSBT or edit the signer's review.
Wrong-session, cancelled, locked and duplicate confirmations cannot sign.
`CancelDemo` and signer `Lock` discard pending review; a new request supersedes it.
There is no USB/Wi-Fi transaction import, user-wallet signing, remote approval,
or raw hash signing. Default firmware signing remains disconnected.

The demo seed is deliberately public: 32 zero BIP39 entropy bytes (24 words),
BIP39 passphrase `12345678`, testnet path `m/84'/1'/0'/0/0`. Its temporary encrypted
copy uses a fixed nonce only because plaintext and key are public and identical;
**never copy that nonce practice to real wallets**. Existing PSBT policy remains
unchanged: full previous transaction, matching origins/amounts/scripts,
SIGHASH_ALL and verified partial ECDSA signatures. The fixture is 1 input,
1 output, 100000 sat in, 99000 sat out and 1000 sat fee, not chain-spendable.

Preparation and signing are single-flight; full command queues allow retry
without reporting false success. Completion shows `Signed! USB hex` or
`Failed - retry left`, and cancellation shows `Cancelled - no sign`.
A lock event leaves Home; late results never unlock the device. Signing already
accepted after confirmation may finish before a subsequently queued Lock, using
**only the public demo key**. This demo does not claim production-grade physical
presence, atomic lock-vs-confirm ordering, delivery guarantees or hardware testing.

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
fixture structure, full review field pagination, host-rendered 128x64 frames,
default-disabled signing, explicit default-cancel confirmation, invisible-frame
blocking, duplicate/stale sessions, backpressure, completion/error retry and
lock/late-result handling. They do not exercise GPIO,
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
   release **only left** once. Expected: `Validating demo...`, then review pages.
   Right advances and left goes back; both cancels without producing signed hex.
   Review every full address/outpoint and the 1000 sat fee. After the final page,
   expect `Confirm signing?` with CANCEL selected. Right selects SIGN; release,
   then press both to authorize. Only now expect `Signing demo...`, then
   `Signed! USB hex`. Cryptography is synchronous; UI may briefly pause.
5. Capture the complete USB line `TEST ONLY signed PSBT hex: ...` (also emitted
   via RTT). Save **only the hex payload** to `signed.hex`, without log prefix.
   Run the host `demo_psbt verify signed.hex` command above. Expected: exact
   379-byte signed fixture match. USB logging is best effort with a 1024-byte
   buffer: attach first; missing/truncated output requires another complete review.
   OLED success means signing completed, not guaranteed delivery to the host.
6. Hold left: there must be no repeated actions while held. Navigation alone
   must not emit signed hex. Cancel from a review page and from the final default
   CANCEL choice: neither may sign. Start another review and confirm: the second
   complete output should verify identically. Let the device lock DURING review:
   pending review is discarded and left cannot sign while locked. A signing
   operation accepted AFTER confirmation may still finish before Lock. Reboot: ordinary
   setup behavior is unchanged, and real user-wallet signing remains absent.
7. Rebuild/flash **without** `psbt-demo`: Home must show `Signing disabled` and
   left must not emit a signed PSBT.

Hardware checks above are a manual acceptance procedure, **not claimed results**.
No board/probe/serial device was available during implementation. Cross-linked
ELFs do not prove runtime heap/stack fit, physical confirmation timing, display
rendering or side-channel resistance. Production user-wallet support still needs
an independently audited request/authorization path, MCU validation of all pages
and input freshness, robust lock/confirm race handling, bounded RAM and secure
transport; see [the original policy/blockers](psbt.md). The host-only `sign_demo`
shortcut is compiled out for `target_os = "none"`; embedded code uses the
two-stage session API exclusively.
