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

/// Computes the BIP-39 checksum for the given entropy.
/// Checksum length is ENT / 32 bits (i.e. entropy.len() / 4 bits).
pub fn checksum_entropy(entropy: &[u8; 32]) -> Result<u8, &'static str> {
    let hash = Sha256::digest(entropy);
    let cs_bits = entropy.len() / 4;
    Ok(hash[0] >> (8 - cs_bits))
}

#[cfg(feature = "alloc")]
/// Converts raw entropy bytes into a list of BIP-39 words.
pub fn entropy_to_words(entropy: &[u8; 32]) -> Result<[&'static str; 24], &'static str> {
    let checksum = checksum_entropy(entropy)?;
    let cs_bits = entropy.len() / 4;

    let mut bits: [u8; 264] = [0; 264];

    for (i, &byte) in entropy.iter().enumerate() {
        for j in 0..8 {
            bits[i * 8 + j] = (byte >> (7 - j)) & 1;
        }
    }

    for j in 0..cs_bits {
        bits[256 + j] = (checksum >> (cs_bits - 1 - j)) & 1;
    }

    let mut words: [&'static str; 24] = [""; 24];
    for i in 0..24 {
        let mut index: usize = 0;
        for j in 0..11 {
            index = (index << 1) | bits[i * 11 + j] as usize;
        }
        words[i] = WORD_LIST[index];
    }
    Ok(words)
}

#[cfg(feature = "alloc")]
/// Converts raw entropy bytes into a space-separated BIP-39 mnemonic phrase.
pub fn entropy_to_mnemonic(entropy: &[u8; 32]) -> Result<String, &'static str> {
    let words = entropy_to_words(entropy)?;
    Ok(words.join(" "))
}

/// Runs self-tests on the bare-metal target without requiring `libtest`.
pub fn run_self_tests() -> Result<(), &'static str> {
    let cs = checksum_entropy(&[0u8; 32])?;
    if cs != 0x03 {
        return Err("Self-test failed: incorrect checksum");
    }
    #[cfg(feature = "alloc")]
    {
        let m = entropy_to_mnemonic(&[0u8; 32])?;
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
