# USB PSBT transport

The Pico exposes a composite USB CDC-ACM device over its existing micro-USB
connector. Use a USB data cable. The first serial interface is intended for logs and
the second for the PSBT protocol. Operating systems may assign device names in a
different order, so use the `ping` command to identify the protocol interface.

For a no-funds, reproducible hardware walkthrough, use the repository's explicitly
unsafe deterministic firmware and fixture generator described in
[`README.md`](../README.md#deterministic-hardware-test-mode). The generated PSBT exercises
this complete transport and signing protocol but cannot be broadcast.

```bash
cargo run -p pico-ledger-cli --target x86_64-unknown-linux-gnu -- \
  --port /dev/ttyACM1 ping
```

## Signing

First export the unlocked wallet's watch-only descriptor. It contains the master
fingerprint and testnet BIP84 account xpub, covering external and change chains. It does
not contain private keys or seed material.

```bash
cargo run -p pico-ledger-cli --target x86_64-unknown-linux-gnu -- \
  --port /dev/ttyACM1 export-descriptor wallet.desc
```

The OLED asks for privacy approval before releasing the descriptor. Press both buttons
to allow the export or the left button to reject it.

Derive the first receive address and fund it on Bitcoin testnet:

```bash
cargo run -p pico-ledger-cli --target x86_64-unknown-linux-gnu -- \
  address wallet.desc 0 0
```

Save the complete raw funding transaction as hexadecimal text in `previous-tx.hex`. Build the
unsigned PSBT with the funding output number, external address index, recipient, amount,
fee, change index and output filename:

```bash
cargo run -p pico-ledger-cli --target x86_64-unknown-linux-gnu -- \
  make-psbt wallet.desc previous-tx.hex 0 0 tb1qrecipient... \
  90000 1000 0 unsigned.psbt
```

The builder supports one input. It verifies that the selected previous output belongs to
the exported descriptor, requires a testnet P2WPKH recipient, embeds the complete previous
transaction and creates change at `m/84'/1'/0'/1/change-index`. The amount and fee are
specified in satoshis. If the input exactly equals amount plus fee, no change output is
created.

The builder cannot determine whether the funding output is confirmed or still unspent.
Check that state with your testnet node or block explorer before constructing the PSBT.

Send the resulting PSBT to the device:

```bash
cargo run -p pico-ledger-cli --target x86_64-unknown-linux-gnu -- \
  --port /dev/ttyACM1 sign unsigned.psbt signed.psbt
```

The client accepts binary PSBT v0 files up to 16 KiB. The device validates the PSBT
against the unlocked testnet BIP84 wallet and then displays the network, transaction
identifier, transaction fields, every input sequence, every output address and amount,
verified change, totals, fee and sighash policy. Right advances, left goes back and both
buttons reject during review. On the final screen, both buttons approve, left rejects and
right returns to the final review page. The exact validated PSBT remains owned by the
signer until approval, so the host cannot swap it after review.

The host waits up to five minutes for the physical review. Rejection, validation failure
and device locking are returned as explicit errors. Only one request is processed at a
time.

## Framing

Every request and response starts with a 12-byte little-endian header.

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | ASCII `PICO` |
| 4 | 1 | Protocol version, currently `1` |
| 5 | 1 | Message kind |
| 6 | 2 | Request identifier |
| 8 | 4 | Payload length |

Message kinds are `0x01` ping, `0x02` sign PSBT, `0x03` export descriptor, `0x80` ready
for payload, `0x81` pong, `0x82` signed PSBT, `0x83` descriptor and `0xff` error. For
signing, the host sends the header and waits for `0x80` before transmitting the payload.
This lets the device reject an invalid or oversized length without buffering or draining
attacker-controlled data. Error payloads contain UTF-8 text. The serial baud setting is
ignored by USB CDC, although the supplied client requests 115200 baud for compatibility.
