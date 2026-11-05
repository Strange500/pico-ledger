//! Host exercise/capture verifier. Never accepts a transaction for signing.
use pico_ledger_core::demo::sign_demo;
use std::{fs::File, io::Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut randomness = [0; 32];
    // Host tool is Linux/Unix-only; firmware uses the RP2350 TRNG instead.
    File::open("/dev/urandom")?.read_exact(&mut randomness)?;
    let signed = sign_demo(&mut randomness)?;
    let expected: String = signed.iter().map(|byte| format!("{byte:02x}")).collect();
    match args.get(1).map(String::as_str) {
        Some("sign") if args.len() == 2 => println!("{expected}"),
        Some("verify") if args.len() == 3 => {
            let captured = std::fs::read_to_string(&args[2])?;
            if captured.trim() != expected {
                return Err(
                    "Not the exact signed TEST ONLY demo PSBT (or truncated USB log)".into(),
                );
            }
            println!("TEST ONLY: fixed PSBT and deterministic signature match ({} bytes). No chain UTXO; not broadcastable.", signed.len());
        }
        _ => return Err("Usage: demo_psbt sign | demo_psbt verify signed.hex".into()),
    }
    Ok(())
}
