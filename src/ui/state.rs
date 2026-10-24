use buoyant::{match_view, view::prelude::*};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::BinaryColor, prelude::*};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    SeedPhrase {
        words: [&'static str; 3],
        words_per_page: usize,
        page: usize,
    },
}

impl Screen {
    pub fn view(&self) -> impl View<BinaryColor, ()> + use<'_> {
        match_view!(
            self,
        {
            Screen::Home => home_view(),
            Screen::SeedPhrase {
                words,
                words_per_page,
                page,
            } => {
                let start = page * words_per_page;
                let end = (start + words_per_page).min(words.len());
                seed_phrase_view(&words[start..end])
            }
        })
    }
}

fn home_view() -> impl View<BinaryColor, ()> {
    VStack::new((
        Text::new("Home", &FONT_6X10),
        Divider::new(1),
        Spacer::default(),
        Text::new("Welcome to Pico Ledger!", &FONT_6X10),
        Spacer::default(),
        Text::new("Press > to start", &FONT_6X10),
    ))
    .padding(Edges::Horizontal, 4)
    .padding(Edges::Vertical, 2)
}

fn seed_phrase_view<'a>(words: &'a [&'static str]) -> impl View<BinaryColor, ()> + 'a {
    VStack::new((
        Text::new("Seed Phrase", &FONT_6X10),
        Divider::new(1),
        Spacer::default(),
        ForEach::<3>::new_vertical(words, |word| Text::new(word, &FONT_6X10)),
        Spacer::default(),
        Text::new("Press > to continue", &FONT_6X10),
    ))
}
