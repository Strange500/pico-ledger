pub use bip39::Language;

pub fn word_list() -> &'static [&'static str; 2048] {
    Language::English.word_list()
}
