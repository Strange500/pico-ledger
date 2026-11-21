//! Framing shared by the USB firmware and the host command-line client.

pub const MAGIC: [u8; 4] = *b"PICO";
pub const VERSION: u8 = 1;
pub const HEADER_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Ping = 1,
    SignPsbt = 2,
    Ready = 0x80,
    Pong = 0x81,
    SignedPsbt = 0x82,
    Error = 0xff,
}

impl TryFrom<u8> for Kind {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, &'static str> {
        match value {
            1 => Ok(Self::Ping),
            2 => Ok(Self::SignPsbt),
            0x80 => Ok(Self::Ready),
            0x81 => Ok(Self::Pong),
            0x82 => Ok(Self::SignedPsbt),
            0xff => Ok(Self::Error),
            _ => Err("Unknown message kind"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub kind: Kind,
    pub request_id: u16,
    pub payload_len: u32,
}

impl Header {
    #[must_use]
    pub const fn encode(self) -> [u8; HEADER_LEN] {
        let request_id = self.request_id.to_le_bytes();
        let payload_len = self.payload_len.to_le_bytes();
        [
            MAGIC[0],
            MAGIC[1],
            MAGIC[2],
            MAGIC[3],
            VERSION,
            self.kind as u8,
            request_id[0],
            request_id[1],
            payload_len[0],
            payload_len[1],
            payload_len[2],
            payload_len[3],
        ]
    }

    pub fn decode(bytes: &[u8; HEADER_LEN]) -> Result<Self, &'static str> {
        if bytes[..4] != MAGIC {
            return Err("Invalid protocol magic");
        }
        if bytes[4] != VERSION {
            return Err("Unsupported protocol version");
        }
        Ok(Self {
            kind: Kind::try_from(bytes[5])?,
            request_id: u16::from_le_bytes([bytes[6], bytes[7]]),
            payload_len: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip_and_rejections() {
        let header = Header {
            kind: Kind::SignPsbt,
            request_id: 0x1234,
            payload_len: 16_384,
        };
        assert_eq!(Header::decode(&header.encode()), Ok(header));

        let mut invalid = header.encode();
        invalid[0] = b'X';
        assert!(Header::decode(&invalid).is_err());
        invalid = header.encode();
        invalid[4] = VERSION + 1;
        assert!(Header::decode(&invalid).is_err());
        invalid = header.encode();
        invalid[5] = 42;
        assert!(Header::decode(&invalid).is_err());
    }
}
