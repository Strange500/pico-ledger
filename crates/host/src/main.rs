use pico_ledger_core::{
    psbt::MAX_PSBT_BYTES,
    transport::{Header, Kind, HEADER_LEN},
};
use std::{
    env, fs,
    io::{self, Write},
    path::Path,
    process::ExitCode,
    time::Duration,
};

const REQUEST_ID: u16 = 1;

fn usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {program} --port <serial-port> ping");
    eprintln!("  {program} --port <serial-port> sign <input.psbt> [output.psbt]");
}

fn send(
    port_name: &str,
    kind: Kind,
    payload: &[u8],
) -> Result<(Header, Vec<u8>), Box<dyn std::error::Error>> {
    let payload_len = u32::try_from(payload.len()).map_err(|_| "Payload too large")?;
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_secs(300))
        .open()?;
    port.write_all(
        &Header {
            kind,
            request_id: REQUEST_ID,
            payload_len,
        }
        .encode(),
    )?;
    port.flush()?;

    if kind == Kind::SignPsbt {
        let (ready, ready_payload) = read_response(&mut *port)?;
        if ready.kind == Kind::Error {
            return Err(String::from_utf8_lossy(&ready_payload).into_owned().into());
        }
        if ready.request_id != REQUEST_ID || ready.kind != Kind::Ready || !ready_payload.is_empty()
        {
            return Err("Device did not accept the PSBT payload".into());
        }
        port.write_all(payload)?;
        port.flush()?;
    }

    read_response(&mut *port)
}

fn read_response(
    port: &mut dyn serialport::SerialPort,
) -> Result<(Header, Vec<u8>), Box<dyn std::error::Error>> {
    let mut encoded_header = [0; HEADER_LEN];
    port.read_exact(&mut encoded_header)?;
    let response = Header::decode(&encoded_header).map_err(io::Error::other)?;
    if response.request_id != REQUEST_ID {
        return Err("Mismatched response ID".into());
    }
    let response_len = usize::try_from(response.payload_len)?;
    if response_len > MAX_PSBT_BYTES {
        return Err("Device response exceeds the protocol limit".into());
    }
    let mut response_payload = vec![0; response_len];
    port.read_exact(&mut response_payload)?;
    Ok((response, response_payload))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 || args[1] != "--port" {
        usage(args.first().map_or("pico-ledger-cli", String::as_str));
        return Err("Invalid arguments".into());
    }
    let port = &args[2];
    match args[3].as_str() {
        "ping" if args.len() == 4 => {
            let (header, payload) = send(port, Kind::Ping, &[])?;
            if header.kind != Kind::Pong || !payload.is_empty() {
                return Err("Unexpected ping response".into());
            }
            println!("pico-ledger protocol v1 is ready");
        }
        "sign" if (5..=6).contains(&args.len()) => {
            let input = fs::read(&args[4])?;
            if input.len() > MAX_PSBT_BYTES {
                return Err(format!("PSBT exceeds {MAX_PSBT_BYTES} bytes").into());
            }
            let (header, payload) = send(port, Kind::SignPsbt, &input)?;
            if header.kind == Kind::Error {
                return Err(String::from_utf8_lossy(&payload).into_owned().into());
            }
            if header.kind != Kind::SignedPsbt {
                return Err("Unexpected signing response".into());
            }
            let output = args
                .get(5)
                .map_or_else(|| Path::new("signed.psbt"), Path::new);
            fs::write(output, payload)?;
            println!("signed PSBT written to {}", output.display());
        }
        _ => {
            usage(&args[0]);
            return Err("Invalid arguments".into());
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
