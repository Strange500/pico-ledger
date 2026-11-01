use crate::signals::{Effects, Message, SignerMessage};
use crate::ui::{
    screens::{
        init_screen::InitScreen,
        loading_screen::{LoadingReason, LoadingScreen},
        seed_phrase_screen::SeedPhraseScreen,
    },
    state::AppModel,
};
use crate::wallet::*;
use alloc::vec::Vec;

const PASSWORD: [&str; 8] = ["1", "2", "3", "4", "5", "6", "7", "8"];
const WRONG: [&str; 8] = ["8"; 8];

#[derive(Default)]
struct Recorder {
    commands: Vec<SignerMessage>,
    activity: usize,
    full: bool,
}
impl Effects for Recorder {
    fn send(&mut self, command: SignerMessage) -> Result<(), crate::signals::CommandQueueFull> {
        if self.full {
            return Err(crate::signals::CommandQueueFull);
        }
        self.commands.push(command);
        Ok(())
    }
    fn activity(&mut self) {
        self.activity += 1;
    }
}

#[test]
fn bip39_known_entropy_and_seed_vector() {
    let mnemonic = bip39::Mnemonic::from_entropy(&[0; 32]).unwrap();
    let words: Vec<_> = mnemonic.words().collect();
    assert_eq!(&words[..23], &["abandon"; 23]);
    assert_eq!(words[23], "art");
    // BIP-39 published 128-bit entropy / TREZOR passphrase vector.
    let mnemonic = bip39::Mnemonic::from_entropy(&[0; 16]).unwrap();
    assert_eq!(mnemonic.to_seed_normalized("TREZOR"), hex::<64>("c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04"));
}
fn hex<const N: usize>(text: &str) -> [u8; N] {
    let mut result = [0; N];
    for (out, pair) in result
        .iter_mut()
        .zip(text.as_bytes().as_chunks::<2>().0.iter())
    {
        *out = u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap();
    }
    result
}
#[test]
fn pbkdf2_matches_independent_vector() {
    assert_eq!(
        derive_encryption_key(&PASSWORD),
        hex::<32>("a4ae140c8d00062c4405d7b437a1e380d6a7b85eff9dd488c4fef325f65e6b1c")
    );
}
#[test]
fn encryption_rejects_wrong_password_tampering_and_nonce() {
    let (mut encrypted, nonce) = encrypt_entropy(&[42; 32], &PASSWORD, [7; 12]).unwrap();
    assert_eq!(decrypt_entropy(&encrypted, &PASSWORD, &nonce), Ok([42; 32]));
    assert!(decrypt_entropy(&encrypted, &WRONG, &nonce).is_err());
    assert!(decrypt_entropy(&encrypted, &PASSWORD, &[8; 12]).is_err());
    encrypted[0] ^= 1;
    assert!(decrypt_entropy(&encrypted, &PASSWORD, &nonce).is_err());
    assert!(decrypt_entropy(&[], &PASSWORD, &nonce).is_err());
}
#[test]
fn authenticated_wrong_length_is_an_error() {
    use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit, Nonce};
    let cipher = ChaCha20Poly1305::new(&derive_encryption_key(&PASSWORD).into());
    let ciphertext = cipher
        .encrypt(&Nonce::from([0; 12]), &[1u8; 31][..])
        .unwrap();
    assert_eq!(
        decrypt_entropy(&ciphertext, &PASSWORD, &[0; 12]),
        Err("Invalid entropy length")
    );
}
#[test]
fn wallet_lock_unlock_and_signing() {
    use bitcoin::{
        bip32::Xpriv,
        key::Secp256k1,
        secp256k1::{ecdsa::Signature, Message as Digest},
    };
    let mut wallet = Wallet::default();
    assert!(wallet.sign([1; 32], 1).is_err());
    assert!(wallet.unlock(&PASSWORD).is_err());
    wallet.generate(&mut [0; 32], &PASSWORD, [1; 12]).unwrap();
    assert!(wallet.generate(&mut [2; 32], &PASSWORD, [2; 12]).is_err());
    let signature = wallet.sign([1; 32], 1).unwrap();
    let secp = Secp256k1::new();
    let mnemonic = bip39::Mnemonic::from_entropy(&[0; 32]).unwrap();
    let master = Xpriv::new_master(
        bitcoin::Network::Testnet,
        &mnemonic.to_seed_normalized("12345678"),
    )
    .unwrap();
    let path: bitcoin::bip32::DerivationPath = "m/84'/0'/0'/0/1".parse().unwrap();
    let child = master.derive_priv(&secp, &path).unwrap();
    let public = child.private_key.public_key(&secp);
    secp.verify_ecdsa(
        &Digest::from_digest([1; 32]),
        &Signature::from_compact(&signature).unwrap(),
        &public,
    )
    .unwrap();
    assert_ne!(signature, wallet.sign([1; 32], 2).unwrap());
    wallet.lock();
    assert!(wallet.sign([1; 32], 1).is_err());
    assert!(wallet.unlock(&WRONG).is_err());
    assert!(wallet.sign([1; 32], 1).is_err());
    wallet.unlock(&PASSWORD).unwrap();
    assert_eq!(wallet.sign([1; 32], 1).unwrap(), signature);
}
#[test]
fn password_entry_wraps_and_emits_generate_once_complete() {
    let mut app = AppModel::Init(InitScreen::new());
    let mut effects = Recorder::default();
    app.update(Message::Next, &mut effects);
    app.update(Message::Back, &mut effects);
    if let AppModel::PasswordCreation(screen) = &app {
        assert_eq!(screen.digit, 9);
    } else {
        panic!();
    }
    app.update(Message::Next, &mut effects);
    for _ in 0..7 {
        app.update(Message::Select, &mut effects);
    }
    assert!(effects.commands.is_empty());
    app.update(Message::Select, &mut effects);
    assert!(matches!(app, AppModel::Loading(_)));
    assert_eq!(
        effects.commands,
        [SignerMessage::GenerateSeed { password: ["0"; 8] }]
    );
    assert_eq!(effects.activity, 11);
}
#[test]
fn full_queue_keeps_password_screen_for_retry() {
    let mut app = AppModel::Init(InitScreen::new());
    let mut effects = Recorder {
        full: true,
        ..Recorder::default()
    };
    app.update(Message::Next, &mut effects);
    for _ in 0..8 {
        app.update(Message::Select, &mut effects);
    }
    assert!(matches!(app, AppModel::PasswordCreation(_)));
    effects.full = false;
    app.update(Message::Select, &mut effects);
    assert!(matches!(app, AppModel::Loading(_)));
    assert_eq!(effects.commands.len(), 1);
}
#[test]
fn loading_ignores_unrelated_responses() {
    let mut app = AppModel::Loading(LoadingScreen::new(LoadingReason::GeneratingSeedPhrase));
    let mut effects = Recorder::default();
    app.update(Message::PasswordChecked(true), &mut effects);
    assert!(matches!(app, AppModel::Loading(_)));
    app.update(Message::SeedPhraseGenerated(["abandon"; 24]), &mut effects);
    assert!(matches!(app, AppModel::SeedPhrase(_)));
}
#[test]
fn seed_pagination_stays_in_bounds_and_exits() {
    let mut app = AppModel::SeedPhrase(SeedPhraseScreen::new(["abandon"; 24]));
    let mut effects = Recorder::default();
    for step in 1..=7 {
        app.update(Message::Next, &mut effects);
        if let AppModel::SeedPhrase(screen) = &app {
            assert_eq!(screen.step, step);
        } else {
            panic!();
        }
    }
    app.update(Message::Back, &mut effects);
    if let AppModel::SeedPhrase(screen) = &app {
        assert_eq!(screen.step, 6);
    }
    app.update(Message::Next, &mut effects);
    app.update(Message::Next, &mut effects);
    assert!(matches!(app, AppModel::Home(_)));
    app = AppModel::SeedPhrase(SeedPhraseScreen::new(["abandon"; 24]));
    app.update(Message::Back, &mut effects);
    assert!(matches!(app, AppModel::Home(_)));
}
#[test]
fn lock_and_password_responses_drive_navigation() {
    let mut app = AppModel::Home(crate::ui::screens::home_screen::HomeScreen::new());
    let mut effects = Recorder::default();
    app.update(Message::Next, &mut effects);
    assert_eq!(
        effects.commands.pop(),
        Some(SignerMessage::Sign {
            tx_hash: [1; 32],
            address_index: 1
        })
    );
    app.update(Message::Lock, &mut effects);
    assert!(matches!(app, AppModel::Lock(_)));
    assert_eq!(effects.commands.pop(), Some(SignerMessage::Lock));
    for _ in 0..8 {
        app.update(Message::Select, &mut effects);
    }
    assert_eq!(
        effects.commands.pop(),
        Some(SignerMessage::CheckPassword { password: ["0"; 8] })
    );
    app.update(Message::PasswordChecked(false), &mut effects);
    assert!(matches!(app, AppModel::Lock(_)));
    for _ in 0..8 {
        app.update(Message::Select, &mut effects);
    }
    app.update(Message::PasswordChecked(true), &mut effects);
    assert!(matches!(app, AppModel::Home(_)));
}
#[test]
fn inactivity_does_not_interrupt_setup_or_seed_backup() {
    let mut app = AppModel::Init(InitScreen::new());
    let mut effects = Recorder::default();
    app.update(Message::Lock, &mut effects);
    assert!(matches!(app, AppModel::Init(_)));
    app.update(Message::Next, &mut effects);
    app.update(Message::Lock, &mut effects);
    assert!(matches!(app, AppModel::PasswordCreation(_)));
    app = AppModel::SeedPhrase(SeedPhraseScreen::new(["abandon"; 24]));
    app.update(Message::Lock, &mut effects);
    assert!(matches!(app, AppModel::SeedPhrase(_)));
    assert!(effects.commands.is_empty());
}

#[test]
fn bip32_master_matches_published_vector_one() {
    use bitcoin::{bip32::Xpriv, Network};
    let key = Xpriv::new_master(
        Network::Testnet,
        &hex::<16>("000102030405060708090a0b0c0d0e0f"),
    )
    .unwrap();
    assert_eq!(
        key.private_key.secret_bytes(),
        hex::<32>("e8f32e723decf4051aefac8e2c93c9c5b214313817cdb01a1494b917c8436b35")
    );
    assert_eq!(
        key.chain_code.as_bytes(),
        &hex::<32>("873dff81c02f525623fd1fe5167eac3a55a049de3d314bb42ee227ffed37d508")
    );
}

#[test]
fn generation_clears_callers_entropy_on_success_and_rejection() {
    let mut wallet = Wallet::default();
    let mut entropy = [42; 32];
    wallet.generate(&mut entropy, &PASSWORD, [1; 12]).unwrap();
    assert_eq!(entropy, [0; 32]);
    entropy.fill(42);
    assert!(wallet.generate(&mut entropy, &PASSWORD, [2; 12]).is_err());
    assert_eq!(entropy, [0; 32]);
}
