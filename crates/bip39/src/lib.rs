#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod wordlist;
pub use wordlist::WORD_LIST;

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

/// Verifies that the entropy length is valid according to BIP-39.
/// Valid entropy lengths are 16, 20, 24, 28, or 32 bytes (128 to 256 bits, multiple of 32 bits / 4 bytes).
#[allow(clippy::manual_is_multiple_of)]
pub fn verify_entropy(entropy: &[u8]) -> Result<(), &'static str> {
    if entropy.len() < 16 || entropy.len() > 32 || entropy.len() % 4 != 0 {
        return Err("Invalid entropy length: must be 16, 20, 24, 28, or 32 bytes");
    }
    Ok(())
}

/// Computes the BIP-39 checksum for the given entropy.
/// Checksum length is ENT / 32 bits (i.e. entropy.len() / 4 bits).
pub fn checksum_entropy(entropy: &[u8]) -> Result<u8, &'static str> {
    verify_entropy(entropy)?;
    let hash = Sha256::digest(entropy);
    let cs_bits = entropy.len() / 4;
    Ok(hash[0] >> (8 - cs_bits))
}

#[cfg(feature = "alloc")]
/// Converts raw entropy bytes into a list of BIP-39 words.
pub fn entropy_to_words(entropy: &[u8]) -> Result<Vec<&'static str>, &'static str> {
    verify_entropy(entropy)?;
    let checksum = checksum_entropy(entropy)?;
    let cs_bits = entropy.len() / 4;

    let total_bits = entropy.len() * 8 + cs_bits;
    let mut bits = Vec::with_capacity(total_bits);

    for byte in entropy {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }
    for i in (0..cs_bits).rev() {
        bits.push((checksum >> i) & 1);
    }

    let mut words = Vec::with_capacity(bits.len() / 11);
    for chunk in bits.chunks(11) {
        let mut index = 0usize;
        for bit in chunk {
            index = (index << 1) | (*bit as usize);
        }
        words.push(WORD_LIST[index]);
    }

    Ok(words)
}

#[cfg(feature = "alloc")]
/// Converts raw entropy bytes into a space-separated BIP-39 mnemonic phrase.
pub fn entropy_to_mnemonic(entropy: &[u8]) -> Result<String, &'static str> {
    let words = entropy_to_words(entropy)?;
    Ok(words.join(" "))
}

/// Runs self-tests on the bare-metal target without requiring `libtest`.
pub fn run_self_tests() -> Result<(), &'static str> {
    verify_entropy(&[0u8; 16])?;
    if verify_entropy(&[0u8; 15]).is_ok() {
        return Err("Self-test failed: invalid entropy accepted");
    }
    let cs = checksum_entropy(&[0u8; 16])?;
    if cs != 0x03 {
        return Err("Self-test failed: incorrect checksum");
    }
    #[cfg(feature = "alloc")]
    {
        let m = entropy_to_mnemonic(&[0u8; 16])?;
        if m != "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about" {
            return Err("Self-test failed: incorrect mnemonic for all-zeros vector");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_entropy_valid_lengths() {
        assert!(verify_entropy(&[0u8; 16]).is_ok()); // 128 bits -> 12 words
        assert!(verify_entropy(&[0u8; 20]).is_ok()); // 160 bits -> 15 words
        assert!(verify_entropy(&[0u8; 24]).is_ok()); // 192 bits -> 18 words
        assert!(verify_entropy(&[0u8; 28]).is_ok()); // 224 bits -> 21 words
        assert!(verify_entropy(&[0u8; 32]).is_ok()); // 256 bits -> 24 words
    }

    #[test]
    fn test_verify_entropy_invalid_lengths() {
        assert!(verify_entropy(&[]).is_err());
        assert!(verify_entropy(&[0u8; 12]).is_err());
        assert!(verify_entropy(&[0u8; 15]).is_err());
        assert!(verify_entropy(&[0u8; 17]).is_err());
        assert!(verify_entropy(&[0u8; 18]).is_err());
        assert!(verify_entropy(&[0u8; 31]).is_err());
        assert!(verify_entropy(&[0u8; 33]).is_err());
        assert!(verify_entropy(&[0u8; 64]).is_err());
    }

    #[test]
    fn test_checksum_calculation() {
        // 16 bytes of 0x00 -> SHA256 starts with 0x37 -> 4 MSB bits are 0x03
        let cs = checksum_entropy(&[0u8; 16]).unwrap();
        assert_eq!(cs, 0x03);

        // 16 bytes of 0xFF -> SHA256 starts with 0x5A -> 4 MSB bits are 0x05
        let cs_ff = checksum_entropy(&[0xff; 16]).unwrap();
        assert_eq!(cs_ff, 0x05);
    }

    // Official BIP-39 test vectors from Bitcoin / Trezor
    #[test]
    fn test_vector_128_bit_all_zeros() {
        let entropy = [0x00u8; 16];
        let mnemonic = entropy_to_mnemonic(&entropy).unwrap();
        assert_eq!(
            mnemonic,
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        );
    }

    #[test]
    fn test_vector_128_bit_7f() {
        let entropy = [0x7fu8; 16];
        let mnemonic = entropy_to_mnemonic(&entropy).unwrap();
        assert_eq!(
            mnemonic,
            "legal winner thank year wave sausage worth useful legal winner thank yellow"
        );
    }

    #[test]
    fn test_vector_128_bit_80() {
        let entropy = [0x80u8; 16];
        let mnemonic = entropy_to_mnemonic(&entropy).unwrap();
        assert_eq!(
            mnemonic,
            "letter advice cage absurd amount doctor acoustic avoid letter advice cage above"
        );
    }

    #[test]
    fn test_vector_128_bit_all_ones() {
        let entropy = [0xffu8; 16];
        let mnemonic = entropy_to_mnemonic(&entropy).unwrap();
        assert_eq!(
            mnemonic,
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"
        );
    }

    #[test]
    fn test_vector_160_bit_zeros() {
        let entropy = [0x00u8; 20];
        let words = entropy_to_words(&entropy).unwrap();
        assert_eq!(words.len(), 15);
        assert_eq!(
            words.join(" "),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon address"
        );
    }

    #[test]
    fn test_vector_192_bit_zeros() {
        let entropy = [0x00u8; 24];
        let words = entropy_to_words(&entropy).unwrap();
        assert_eq!(words.len(), 18);
        assert_eq!(
            words.join(" "),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent"
        );
    }

    #[test]
    fn test_vector_224_bit_zeros() {
        let entropy = [0x00u8; 28];
        let words = entropy_to_words(&entropy).unwrap();
        assert_eq!(words.len(), 21);
        assert_eq!(
            words.join(" "),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon admit"
        );
    }

    #[test]
    fn test_vector_256_bit_all_zeros() {
        let entropy = [0x00u8; 32];
        let words = entropy_to_words(&entropy).unwrap();
        assert_eq!(words.len(), 24);
        assert_eq!(
            words.join(" "),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art"
        );
    }

    #[test]
    fn test_vector_256_bit_all_ones() {
        let entropy = [0xffu8; 32];
        let words = entropy_to_words(&entropy).unwrap();
        assert_eq!(words.len(), 24);
        assert_eq!(
            words.join(" "),
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote"
        );
    }

    #[test]
    fn test_vector_mixed_entropy() {
        let entropy = [
            0x9e, 0x88, 0x5d, 0x95, 0x24, 0xd0, 0x3d, 0x9f, 0x46, 0x1b, 0x8a, 0x01, 0x0a, 0x0f,
            0x6e, 0xeb, 0x4d, 0x64, 0x76, 0x1a, 0xfa, 0xcc, 0x5e, 0x81, 0x10, 0x67, 0x5a, 0x3b,
            0xa5, 0x54, 0x56, 0xec,
        ];
        let mnemonic = entropy_to_mnemonic(&entropy).unwrap();
        assert_eq!(
            mnemonic,
            "ozone drill grab endless adult soul blur image absurd expire sweet story stone \
             deposit hip flower future ancient border refuse ripple pretty forum luxury"
        );
    }
}
