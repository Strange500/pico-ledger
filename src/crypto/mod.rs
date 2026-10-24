pub use bip39::*;

/// Converts 32 bytes of raw entropy into 24 BIP-39 mnemonic words.
pub fn entropy_to_words(entropy: &[u8; 32]) -> Result<[&'static str; 24], &'static str> {
    let mnemonic = bip39::Mnemonic::from_entropy(entropy).map_err(|_| "Invalid entropy")?;
    let mut words = [""; 24];
    for (i, word) in mnemonic.words().enumerate() {
        if i < 24 {
            words[i] = word;
        }
    }
    Ok(words)
}

/// Validates BIP-39 mnemonic derivation against official test vectors.
pub fn run_self_tests() -> Result<(), &'static str> {
    let zeros = [0u8; 32];
    let words = entropy_to_words(&zeros)?;
    if words[0] != "abandon" || words[23] != "art" {
        return Err("Self-test failed: all-zeros vector mismatch");
    }

    let ones = [0xffu8; 32];
    let words_ones = entropy_to_words(&ones)?;
    if words_ones[0] != "zoo" || words_ones[23] != "vote" {
        return Err("Self-test failed: all-ones vector mismatch");
    }

    Ok(())
}
