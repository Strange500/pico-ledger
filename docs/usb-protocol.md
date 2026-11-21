# USB PSBT transport

The Pico exposes a composite USB CDC-ACM device over its existing micro-USB
connector. Use a USB data cable. The first serial interface is intended for logs and
the second for the PSBT protocol. Operating systems may assign device names in a
different order, so use the `ping` command to identify the protocol interface.

```bash
cargo run -p pico-ledger-cli --target x86_64-unknown-linux-gnu -- \
  --port /dev/ttyACM1 ping
```

## Signing

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

Message kinds are `0x01` ping, `0x02` sign PSBT, `0x80` ready for payload, `0x81`
pong, `0x82` signed PSBT and `0xff` error. For signing, the host sends the header and
waits for `0x80` before transmitting the payload. This lets the device reject an invalid
or oversized length without buffering or draining attacker-controlled data. Error
payloads contain UTF-8 text. The serial baud setting is ignored by USB CDC, although
the supplied client requests 115200 baud for compatibility.
