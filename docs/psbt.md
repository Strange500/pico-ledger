# PSBT signing policy and integration boundary

The opt-in [TEST ONLY hardware demo](psbt-demo.md) is a deliberately separate
exception: a left-button press signs only a compiled-in, unfunded fixture with a
public demo seed, never with the user's wallet. All production signing/review
blockers below remain unchanged.

## Status: tested core, not an end-to-end hardware signer

The portable core imports binary PSBT v0, validates the entire transaction, produces a
review, and returns a PSBT containing DER-encoded ECDSA partial signatures with the
SIGHASH_ALL byte. It does not finalize, extract, broadcast, or send a raw signature.

**Firmware signing is intentionally unavailable.** The old `Wallet::sign(tx_hash, index)`,
`SignerMessage::Sign`, dummy Home-screen transaction, and signature logging have been
removed. USB is currently a logger, not a PSBT transport. No host message, navigation
button, or boolean supplied over USB can authorize signing.

This is a backend milestone, NOT a claim that the cold wallet can safely sign on-device.
Use test funds only. Hardware review, physical confirmation, cancellation/locking races,
RAM/stack measurement, and real coordinator interoperability remain release blockers.

## Accepted policy

- Raw binary BIP174 PSBT v0 only (`psbt` + `0xff` magic). Base64/hex decoding is the
  caller's responsibility; do not pass textual encodings directly.
- Transaction versions 1 or 2; 1–8 inputs and 1–8 outputs; at most 16 KiB encoded,
  with 109 extra bytes reserved per input still missing a partial signature.
- A nonallocating framing pass bounds counts and lengths before rust-bitcoin decoding.
  Previous transactions are limited to 32 inputs/32 outputs and 32 witness items per
  input. These are resource-policy limits, not consensus rules. They do not replace
  hardware memory/stack profiling or make the implementation allocation-free.
- **Every input must belong to this wallet**: native SegWit v0 P2WPKH, one compressed
  public key and one full BIP32 origin, exact master fingerprint and rederived key,
  and `m/84'/1'/0'/{0,1}/index` with unhardened chain and index. No mixed-wallet inputs,
  multisig, legacy, wrapped SegWit, Taproot, CoinJoin, or alternate accounts.
- Every input requires its complete `non_witness_utxo`. Its txid and vout must match
  the outpoint. Optional `witness_utxo` must exactly match its amount and script.
  Witness-only PSBTs are deliberately rejected: requiring the previous transactions
  authenticates all input amounts and avoids coordinator fee misrepresentation across
  multiple SegWit signatures. This does not prove confirmations, chain membership, or
  that an outpoint is unspent; an offline signer cannot establish those facts.
- Non-null, nonduplicate outpoints; no signed scriptSig or witness in the unsigned tx.
- Absent sighash means ALL; the only explicit accepted value is `0x00000001`.
  NONE, SINGLE, ANYONECANPAY, DEFAULT, and undefined encodings are rejected.
- Existing partial signatures must belong to the exact owned compressed key, use ALL,
  and verify against the independently computed transaction digest. No invalid or
  unrelated signature is silently overwritten. Re-signing this implementation's own
  PSBT is deterministic/idempotent.
- All outputs must be P2WPKH. Output origins are absent (external/unclassified) or a
  single **verified** `m/84'/1'/0'/1/index` change origin matching the script. A
  receive-chain origin is not a valid change claim. Missing metadata never hides an
  output: it is reviewed as an external destination even if it happens to be ours.
- Unknown/proprietary fields, global xpubs, script metadata, finalization fields, and
  unsupported extensions are rejected, not interpreted or silently discarded. Duplicate
  map keys, noncanonical CompactSize, malformed/trailing data, and unsupported versions
  fail closed. Accepted metadata and the unsigned transaction survive serialization.
- Individual amounts and aggregate input/output totals are bounded by Bitcoin's monetary
  supply. Negative fees and arithmetic overflow are errors. There is no automatic fee
  cap or fee-rate estimation; the **absolute satoshi fee must be reviewed explicitly**.
  Zero-valued/dust outputs are not relay-policy checked.

## Network and recovery compatibility

The firmware's existing network is Testnet. The old blind signer incorrectly used the
mainnet coin type `0'` in its BIP84 path. This implementation deliberately requires
**coin type `1'`**, testnet account zero, and displays `tb1q…` addresses. There is no
legacy-path fallback or automatic migration: old coin-0 derived funds/PSBTs are rejected.
Restore any such wallet in a separately audited recovery tool with its original path;
never change a coordinator's origin metadata to make a PSBT pass validation.

The existing eight-digit password remains both the encryption password **and the BIP39
passphrase** (joined without separators). It has NOT been changed into a mere local PIN.
Recovery requires the mnemonic, the exact passphrase, and the correct derivation path.
Changing that password changes the derived wallet. Existing persistent-storage and
secret-memory hardening limitations are not solved by PSBT support; in particular this
is not a claim that every temporary copy of derived keys is securely erased.

## Core API / trust boundary

`Wallet::review_psbt(&mut randomness, bytes)` checks the unlocked wallet and returns `TransactionReview`:
transaction id, version, lock time, input count/sequences, all output addresses and
integer satoshi amounts, verified change flags, input/output totals, and absolute fee.
There is no mutation or signature.

`Wallet::sign_psbt(&mut randomness, bytes, approve)` independently imports and validates the bytes,
then calls a **trusted in-process approver** with a shared reference to the exact
review. Only a true result permits signing. The validated PSBT is privately owned and
cannot be replaced or edited between approval and signing. False, parse/validation
errors, or a locked/uninitialized wallet return an error and no signed PSBT. All inputs
are validated before the callback, and the returned result is atomic (never a partially
returned failure).

Both methods require **32 fresh cryptographically secure random bytes** from the trusted
caller on every invocation (RP2350 TRNG in any future firmware integration, an OS CSPRNG
on a host). They consume and clear the caller's buffer even on error. Do not use fixed
bytes, transaction data, wallet-derived material, or randomness supplied by the remote
coordinator. Fixed seeds in host tests are test-only; the core cannot assess entropy quality.
One context is randomized with `seeded_randomize` before any secret-key public derivation,
then reused for ownership validation and signing. Blinding does not change deterministic
ECDSA signatures and does not replace fresh physical approval.

Backend-owned master and derived `Xpriv` values have a non-Copy RAII owner which explicitly
erases the secret scalar with secp256k1's `non_secure_erase` and zeroizes the chain code on
scope exit, including rejection/error paths. Signing children are dropped immediately after
signing. The blinding-seed copy is zeroized too. This is **best-effort cleanup, not a full
secret-memory guarantee**: rust-bitcoin 0.32.102's BIP32 internals create Copy intermediates
and HMAC state outside these guards, and compiler-generated copies/register spills and
secp256k1 internals have not been audited. No dependency-level erasure is claimed.

The callback is an application trust boundary, **not proof of physical presence**.
Passing `|_| true` is suitable only for controlled host tests, not production firmware.
Do not expose this API as a remote RPC with a caller-controlled approval flag. A cached
review from `review_psbt` is not authorization: signing always reviews its own validated
transaction, so a trusted approver must inspect that callback's exact argument.

### Required follow-up before firmware wiring

1. Bounded binary transport with backpressure, deterministic cancellation, timeout, and
   safe handling of concurrent/queued imports. Do not reuse the USB logging interface
   as an implicit approval channel.
2. Private pending-transaction state in the signer; immutable binding of all PSBT bytes,
   the displayed review, and a one-shot request identity. No public confirm-by-hash API.
3. Paginate every destination's **entire** address and satoshi amount, change and network
   labels, totals, absolute fee, and transaction locktime/RBF/sequence information.
   No ellipses, auto-confirm, or skipped output pages; suspicious fees need clear warnings.
4. Fresh, deliberate physical confirmation only after every review page, plus explicit
   rejection. A host message, stale button event, or normal Home navigation is insufficient.
5. Lock, disconnect, replacement, timeout, or rejection must destroy pending approval.
   Test those races and request replay; do not let a later unlock revive old approval.
6. Measure peak parser/secp256k1 heap and stack use on RP2350; enforce an input budget
   compatible with the real firmware heap. Cross-building alone does not prove fit.
7. Hardware button/display verification and interoperability with real testnet PSBTs
   from a coordinator configured with full previous transactions and this exact policy.

## Verification

Existing CI runs the integration tests automatically, with no new dependencies:

```sh
cargo fmt --all -- --check
cargo test --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu
cargo clippy --locked -p pico-ledger-core --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
cargo clippy --locked --all-features -- --deny=warnings
cargo build --locked -p pico-ledger
cargo build --locked -p pico-ledger --release
```

The PSBT suite exercises real ECDSA verification, an independently assembled BIP143
preimage (not SighashCache for that cross-check), multiple inputs, verified change,
optional witness UTXOs, refusal/locking, transaction substitution, idempotent partial
signing, malformed/oversized/truncated inputs, hostile vector counts, duplicate keys,
UTXO/origin/path/sighash mismatches, amount bounds, and unsupported metadata/scripts.
It does not substitute for a fuzzing campaign, external security audit, coordinator
round-trip, or tests on real hardware.
